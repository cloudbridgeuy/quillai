use std::{pin::Pin, str, sync::LazyLock};

use async_stream::stream;
use futures_util::{Stream, StreamExt};
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;

use super::catalog::ReasoningEffort;
use super::models::{AiAction, FinishReason, GenerateInput, StreamEvent};
use super::prompt::{build_prompt, escape_prompt_text};
use super::replacement::ReplacementExtractor;
use super::tagged_text::{into_delta, TaggedText};
use crate::error::Error;

const DEFAULT_OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
const SYSTEM_PROMPT: &str =
    "You are QuillAI, a concise fiction-writing assistant. Reply with plain text only.";

pub type EventStream = Pin<Box<dyn Stream<Item = StreamEvent> + Send>>;

pub trait AiProvider {
    fn stream(&self, input: GenerateInput) -> EventStream;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAiCompatibleConfig {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
}

impl OpenAiCompatibleConfig {
    pub fn from_env() -> Result<Self, Error> {
        Self::from_env_with_model(read_required_env("QUILLAI_AI_MODEL")?)
    }

    pub fn from_env_with_model(model: String) -> Result<Self, Error> {
        Ok(Self {
            api_key: read_required_env("QUILLAI_AI_API_KEY")?,
            model,
            base_url: read_optional_env("QUILLAI_AI_BASE_URL")
                .unwrap_or_else(|| DEFAULT_OPENAI_BASE_URL.into()),
        })
    }
}

#[derive(Clone)]
pub struct OpenAiCompatibleProvider {
    client: Client,
    config: OpenAiCompatibleConfig,
    include_usage: bool,
    reasoning_effort: Option<ReasoningEffort>,
}

impl OpenAiCompatibleProvider {
    pub fn from_env() -> Result<Self, Error> {
        Self::new(OpenAiCompatibleConfig::from_env()?)
    }

    pub fn new(config: OpenAiCompatibleConfig) -> Result<Self, Error> {
        let client = shared_http_client().clone();

        Ok(Self {
            client,
            config,
            include_usage: true,
            reasoning_effort: None,
        })
    }

    pub fn with_include_usage(mut self, include_usage: bool) -> Self {
        self.include_usage = include_usage;
        self
    }

    pub fn with_reasoning_effort(mut self, reasoning_effort: Option<ReasoningEffort>) -> Self {
        self.reasoning_effort = reasoning_effort;
        self
    }
}

fn shared_http_client() -> &'static Client {
    static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

    &HTTP_CLIENT
}

impl AiProvider for OpenAiCompatibleProvider {
    fn stream(&self, input: GenerateInput) -> EventStream {
        let client = self.client.clone();
        let config = self.config.clone();
        let include_usage = self.include_usage;
        let reasoning_effort = self.reasoning_effort;

        Box::pin(stream! {
            let request_body = ChatCompletionRequest::from_input(config.model.clone(), include_usage, reasoning_effort, &input);
            let response = match client
                .post(format!("{}/chat/completions", config.base_url))
                .bearer_auth(config.api_key)
                .header("accept", "text/event-stream")
                .json(&request_body)
                .send()
                .await
            {
                Ok(response) => response,
                Err(err) => {
                    yield StreamEvent::Error {
                        error: format!("failed to reach AI provider: {err}"),
                    };
                    return;
                }
            };

            if !response.status().is_success() {
                let error = match response.text().await {
                    Ok(body) => provider_error_message(&body, "AI provider request failed"),
                    Err(err) => format!("AI provider request failed: {err}"),
                };

                yield StreamEvent::Error { error };
                return;
            }

            let mut upstream = response.bytes_stream();
            let mut utf8_remainder = Vec::new();
            let mut sse_remainder = String::new();
            let mut assembler = UpstreamAssembler::new(include_usage, input.action);

            while let Some(next_chunk) = upstream.next().await {
                let chunk = match next_chunk {
                    Ok(chunk) => chunk,
                    Err(err) => {
                        yield StreamEvent::Error {
                            error: format!("failed to read AI provider stream: {err}"),
                        };
                        return;
                    }
                };

                let (text, next_utf8_remainder) = match decode_utf8_chunk(&utf8_remainder, &chunk) {
                    Ok(decoded) => decoded,
                    Err(err) => {
                        yield StreamEvent::Error {
                            error: format!("AI provider stream was not valid UTF-8: {err}"),
                        };
                        return;
                    }
                };
                utf8_remainder = next_utf8_remainder;

                let parsed = parse_sse_chunk(&sse_remainder, &text);
                sse_remainder = parsed.remainder;

                let (events, stop) = consume_payloads(&mut assembler, &parsed.payloads);
                for event in events {
                    yield event;
                }
                if stop {
                    return;
                }
            }

            if !utf8_remainder.is_empty() {
                yield StreamEvent::Error {
                    error: "AI provider stream ended with an incomplete UTF-8 sequence.".into(),
                };
                return;
            }

            let finalized = finalize_sse_remainder(&sse_remainder);
            let (events, stop) = consume_payloads(&mut assembler, &finalized.payloads);
            for event in events {
                yield event;
            }
            if stop {
                return;
            }

            if let Some(events) = assembler.end_of_stream() {
                for event in events {
                    yield event;
                }
                return;
            }

            yield StreamEvent::Error {
                error: "AI provider stream ended before a completion event arrived.".into(),
            };
        })
    }
}

#[derive(Debug, Serialize)]
struct StreamOptions {
    include_usage: bool,
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream_options: Option<StreamOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<ReasoningEffort>,
    max_tokens: usize,
    messages: Vec<ChatMessage>,
}

impl ChatCompletionRequest {
    fn from_input(
        model: String,
        include_usage: bool,
        reasoning_effort: Option<ReasoningEffort>,
        input: &GenerateInput,
    ) -> Self {
        let current_turn = build_prompt(input);
        Self {
            model,
            stream: true,
            stream_options: include_usage.then_some(StreamOptions {
                include_usage: true,
            }),
            reasoning_effort,
            max_tokens: input.max_output_tokens,
            messages: chat_messages(input, current_turn),
        }
    }
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

fn chat_messages(input: &GenerateInput, current_turn: String) -> Vec<ChatMessage> {
    let history: Vec<ChatMessage> = match &input.section_overrides.history {
        Some(messages) => messages
            .iter()
            .map(|message| ChatMessage {
                role: message.role.as_str().into(),
                content: escape_prompt_text(&message.text),
            })
            .collect(),
        None => input
            .messages
            .iter()
            .map(|message| ChatMessage {
                role: message.role.as_str().into(),
                content: escape_prompt_text(&message.content),
            })
            .collect(),
    };
    std::iter::once(ChatMessage {
        role: "system".into(),
        content: SYSTEM_PROMPT.into(),
    })
    .chain(
        input
            .author_guidance
            .as_deref()
            .filter(|text| !text.is_empty())
            .map(|text| ChatMessage {
                role: "system".into(),
                content: format!(
                    "<author_guidance>\n{}\n</author_guidance>",
                    escape_prompt_text(text)
                ),
            }),
    )
    .chain(history)
    .chain(std::iter::once(ChatMessage {
        role: "user".into(),
        content: current_turn,
    }))
    .collect()
}

#[derive(Debug, PartialEq, Eq)]
struct ParsedSseChunk {
    payloads: Vec<String>,
    remainder: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TokenUsage {
    prompt_tokens: u64,
    completion_tokens: u64,
}

#[derive(Debug, PartialEq, Eq)]
enum StreamPayload {
    Text(String),
    Usage(TokenUsage),
    Finish(FinishReason),
    Done(FinishReason),
    Skip,
    Error(String),
}

struct UpstreamAssembler {
    include_usage: bool,
    pending_finish: Option<FinishReason>,
    pending_usage: Option<TokenUsage>,
    replacement: Option<ReplacementExtractor>,
}

impl UpstreamAssembler {
    fn new(include_usage: bool, action: AiAction) -> Self {
        Self {
            include_usage,
            pending_finish: None,
            pending_usage: None,
            replacement: (action == AiAction::Edit).then(ReplacementExtractor::default),
        }
    }

    fn push(&mut self, payload: StreamPayload) -> (Vec<StreamEvent>, bool) {
        match payload {
            StreamPayload::Text(text) => {
                let text = match &mut self.replacement {
                    Some(replacement) => replacement.push(&text),
                    None => text,
                };
                let events = (!text.is_empty())
                    .then(|| StreamEvent::Delta {
                        delta: into_delta(&TaggedText::generated(&text)),
                    })
                    .into_iter()
                    .collect();
                (events, false)
            }
            StreamPayload::Usage(usage) => {
                if self.include_usage {
                    self.pending_usage = Some(usage);
                }
                (Vec::new(), false)
            }
            StreamPayload::Finish(reason) => {
                self.pending_finish = Some(reason);
                (Vec::new(), false)
            }
            StreamPayload::Done(reason) => (self.completion(reason), true),
            StreamPayload::Skip => (Vec::new(), false),
            StreamPayload::Error(error) => (vec![StreamEvent::Error { error }], true),
        }
    }

    fn end_of_stream(&mut self) -> Option<Vec<StreamEvent>> {
        let finish = self.pending_finish.take()?;
        Some(self.completion(finish))
    }

    fn completion(&mut self, fallback: FinishReason) -> Vec<StreamEvent> {
        let finish = self.pending_finish.take().unwrap_or(fallback);
        let mut events = Vec::new();
        if let Some(usage) = self.pending_usage.take() {
            events.push(StreamEvent::Usage {
                prompt_tokens: usage.prompt_tokens,
                completion_tokens: usage.completion_tokens,
            });
        }
        events.push(
            match self.replacement.as_ref().map(ReplacementExtractor::finish) {
                Some(Err(error)) => StreamEvent::Error { error },
                _ => StreamEvent::Done {
                    finish_reason: finish,
                },
            },
        );
        events
    }
}

fn consume_payloads(
    assembler: &mut UpstreamAssembler,
    payloads: &[String],
) -> (Vec<StreamEvent>, bool) {
    let mut events = Vec::new();
    for payload in payloads {
        for item in parse_stream_payload(payload) {
            let (next, stop) = assembler.push(item);
            events.extend(next);
            if stop {
                return (events, true);
            }
        }
    }
    (events, false)
}

fn read_required_env(name: &str) -> Result<String, Error> {
    read_optional_env(name).ok_or_else(|| {
        Error::AiProviderConfig(format!("missing required environment variable {name}"))
    })
}

fn decode_utf8_chunk(remainder: &[u8], chunk: &[u8]) -> Result<(String, Vec<u8>), str::Utf8Error> {
    let mut combined = remainder.to_vec();
    combined.extend_from_slice(chunk);

    match str::from_utf8(&combined) {
        Ok(text) => Ok((text.to_string(), Vec::new())),
        Err(err) if err.error_len().is_none() => {
            let valid_up_to = err.valid_up_to();
            let valid = str::from_utf8(&combined[..valid_up_to])?;
            Ok((valid.to_string(), combined[valid_up_to..].to_vec()))
        }
        Err(err) => Err(err),
    }
}

fn read_optional_env(name: &str) -> Option<String> {
    std::env::var(name).ok().and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.trim_end_matches('/').to_string())
        }
    })
}

fn parse_sse_chunk(remainder: &str, chunk: &str) -> ParsedSseChunk {
    let combined = normalize_sse_newlines(&format!("{remainder}{chunk}"));
    let mut parts = combined
        .split("\n\n")
        .map(str::to_string)
        .collect::<Vec<_>>();
    let remainder = parts.pop().unwrap_or_default();

    ParsedSseChunk {
        payloads: parts
            .into_iter()
            .filter_map(|event| parse_sse_event(&event))
            .collect(),
        remainder,
    }
}

fn normalize_sse_newlines(input: &str) -> String {
    input.replace("\r\n", "\n").replace('\r', "\n")
}

fn finalize_sse_remainder(remainder: &str) -> ParsedSseChunk {
    ParsedSseChunk {
        payloads: parse_sse_event(remainder).into_iter().collect(),
        remainder: String::new(),
    }
}

fn parse_sse_event(event: &str) -> Option<String> {
    let data = event
        .lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .map(str::trim_start)
        .collect::<Vec<_>>()
        .join("\n");

    if data.trim().is_empty() {
        None
    } else {
        Some(data)
    }
}

fn parse_stream_payload(payload: &str) -> Vec<StreamPayload> {
    if payload == "[DONE]" {
        return vec![StreamPayload::Done(FinishReason::Unknown)];
    }

    let value: Value = match serde_json::from_str(payload) {
        Ok(value) => value,
        Err(err) => {
            return vec![StreamPayload::Error(format!(
                "AI provider returned an invalid stream payload: {err}"
            ))];
        }
    };

    if let Some(error) = extract_provider_error(&value) {
        return vec![StreamPayload::Error(error)];
    }

    let mut events = Vec::new();
    if let Some(text) = extract_chunk_text(&value).filter(|text| !text.is_empty()) {
        events.push(StreamPayload::Text(text));
    }
    if let Some(usage) = extract_usage(&value) {
        events.push(StreamPayload::Usage(usage));
    }
    if let Some(reason) = extract_finish_reason(&value) {
        events.push(StreamPayload::Finish(reason));
    }
    if events.is_empty() {
        events.push(StreamPayload::Skip);
    }
    events
}

fn extract_usage(value: &Value) -> Option<TokenUsage> {
    let usage = value.get("usage")?;
    if usage.is_null() {
        return None;
    }
    let prompt_tokens = json_u64(usage.get("prompt_tokens")?)?;
    let completion_tokens = json_u64(usage.get("completion_tokens")?)?;
    Some(TokenUsage {
        prompt_tokens,
        completion_tokens,
    })
}

fn json_u64(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_i64().and_then(|count| u64::try_from(count).ok()))
}

fn extract_provider_error(value: &Value) -> Option<String> {
    let error = value.get("error").filter(|error| !error.is_null())?;
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("AI provider request failed");
    Some(message.to_string())
}

fn extract_chunk_text(value: &Value) -> Option<String> {
    let choices = value.get("choices")?.as_array()?;
    let choice = choices.first()?;
    let delta = choice.get("delta")?;
    delta
        .get("content")
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn extract_finish_reason(value: &Value) -> Option<FinishReason> {
    let choices = value.get("choices")?.as_array()?;
    let choice = choices.first()?;
    let reason = choice.get("finish_reason")?.as_str()?;

    Some(match reason {
        "stop" => FinishReason::Stop,
        "length" => FinishReason::Length,
        _ => FinishReason::Unknown,
    })
}

fn provider_error_message(body: &str, fallback: &str) -> String {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| extract_provider_error(&value))
        .unwrap_or_else(|| {
            let trimmed = body.trim();
            if trimmed.is_empty() {
                fallback.into()
            } else {
                trimmed.into()
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sse_chunk_collects_complete_events() {
        let parsed = parse_sse_chunk(
            "",
            "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\ndata: [DONE]\n\n",
        );

        assert_eq!(
            parsed,
            ParsedSseChunk {
                payloads: vec![
                    "{\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}".into(),
                    "[DONE]".into(),
                ],
                remainder: String::new(),
            }
        );
    }

    #[test]
    fn parse_sse_chunk_handles_crlf_framing() {
        let parsed = parse_sse_chunk(
            "",
            "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\r\n\r\ndata: [DONE]\r\n\r\n",
        );

        assert_eq!(
            parsed,
            ParsedSseChunk {
                payloads: vec![
                    "{\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}".into(),
                    "[DONE]".into(),
                ],
                remainder: String::new(),
            }
        );
    }

    #[test]
    fn parse_sse_chunk_preserves_partial_remainder() {
        let parsed = parse_sse_chunk(
            "data: {\"choices\":[{\"delta\":{",
            "\"content\":\"Hello\"}}]}\n\n",
        );

        assert_eq!(
            parsed.payloads,
            vec!["{\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}"],
        );
        assert_eq!(parsed.remainder, "");
    }

    #[test]
    fn decode_utf8_chunk_carries_partial_multibyte_sequences() {
        let (decoded, remainder) = decode_utf8_chunk(&[], &[0xF0, 0x9F]).unwrap();
        assert_eq!(decoded, "");
        assert_eq!(remainder, vec![0xF0, 0x9F]);

        let (decoded, remainder) = decode_utf8_chunk(&remainder, &[0x92, 0xA1]).unwrap();
        assert_eq!(decoded, "💡");
        assert!(remainder.is_empty());
    }

    #[test]
    fn parse_stream_payload_extracts_text() {
        assert_eq!(
            parse_stream_payload("{\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}"),
            vec![StreamPayload::Text("Hello".into())]
        );
    }

    #[test]
    fn parse_stream_payload_extracts_finish_reason() {
        assert_eq!(
            parse_stream_payload("{\"choices\":[{\"delta\":{},\"finish_reason\":\"length\"}]}"),
            vec![StreamPayload::Finish(FinishReason::Length)]
        );
    }

    #[test]
    fn parse_stream_payload_extracts_final_usage_chunk() {
        assert_eq!(
            parse_stream_payload(
                "{\"choices\":[],\"usage\":{\"prompt_tokens\":11,\"completion_tokens\":4,\"total_tokens\":15}}"
            ),
            vec![StreamPayload::Usage(TokenUsage {
                prompt_tokens: 11,
                completion_tokens: 4,
            })]
        );
    }

    #[test]
    fn parse_stream_payload_ignores_absent_or_incomplete_usage() {
        assert_eq!(
            parse_stream_payload(
                "{\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":null}"
            ),
            vec![StreamPayload::Finish(FinishReason::Stop)]
        );
        assert_eq!(
            parse_stream_payload("{\"choices\":[],\"usage\":{\"prompt_tokens\":\"11\"}}"),
            vec![StreamPayload::Skip]
        );
    }

    #[test]
    fn assembler_emits_usage_before_done_and_omits_it_when_absent() {
        let with_usage = collect_upstream(&[
            "{\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}",
            "{\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}",
            "{\"choices\":[],\"usage\":{\"prompt_tokens\":11,\"completion_tokens\":4,\"total_tokens\":15}}",
            "[DONE]",
        ]);
        assert_eq!(event_types(&with_usage), vec!["delta", "usage", "done"]);
        assert_eq!(
            serde_json::to_value(&with_usage[1]).unwrap()["promptTokens"],
            11
        );
        assert_eq!(
            serde_json::to_value(&with_usage[1]).unwrap()["completionTokens"],
            4
        );
        assert_eq!(
            serde_json::to_value(&with_usage[2]).unwrap()["finishReason"],
            "stop"
        );

        let without_usage = collect_upstream(&[
            "{\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}",
            "{\"choices\":[{\"delta\":{},\"finish_reason\":\"length\"}]}",
            "[DONE]",
        ]);
        assert_eq!(event_types(&without_usage), vec!["delta", "done"]);
        assert_eq!(
            serde_json::to_value(&without_usage[1]).unwrap()["finishReason"],
            "length"
        );
    }

    #[test]
    fn assembler_flushes_buffered_usage_when_the_stream_ends_without_done_marker() {
        let events = collect_upstream(&[
            "{\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}",
            "{\"choices\":[],\"usage\":{\"prompt_tokens\":2,\"completion_tokens\":3}}",
        ]);
        assert_eq!(event_types(&events), vec!["usage", "done"]);
    }

    fn collect_upstream(payloads: &[&str]) -> Vec<StreamEvent> {
        collect_action_upstream(AiAction::Generate, payloads)
    }

    fn collect_action_upstream(action: AiAction, payloads: &[&str]) -> Vec<StreamEvent> {
        let mut assembler = UpstreamAssembler::new(true, action);
        let owned = payloads
            .iter()
            .map(|payload| (*payload).to_string())
            .collect::<Vec<_>>();
        let (mut events, stop) = consume_payloads(&mut assembler, &owned);
        if !stop {
            if let Some(rest) = assembler.end_of_stream() {
                events.extend(rest);
            }
        }
        events
    }

    fn event_types(events: &[StreamEvent]) -> Vec<&'static str> {
        events
            .iter()
            .map(|event| match event {
                StreamEvent::Delta { .. } => "delta",
                StreamEvent::Usage { .. } => "usage",
                StreamEvent::Done { .. } => "done",
                StreamEvent::Error { .. } => "error",
                StreamEvent::Cancelled => "cancelled",
            })
            .collect()
    }

    #[test]
    fn parse_stream_payload_recognizes_done_marker() {
        assert_eq!(
            parse_stream_payload("[DONE]"),
            vec![StreamPayload::Done(FinishReason::Unknown)]
        );
    }

    #[test]
    fn parse_stream_payload_surfaces_provider_errors() {
        assert_eq!(
            parse_stream_payload("{\"error\":{\"message\":\"bad key\"}}"),
            vec![StreamPayload::Error("bad key".into())]
        );
    }

    fn content_payload(text: &str) -> String {
        serde_json::json!({"choices": [{"delta": {"content": text}}]}).to_string()
    }

    fn delta_text(events: &[StreamEvent]) -> String {
        events
            .iter()
            .filter_map(|event| match event {
                StreamEvent::Delta { delta } => Some(serde_json::to_value(delta).unwrap()),
                _ => None,
            })
            .flat_map(|delta| {
                delta["ops"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|op| op["insert"].as_str().unwrap().to_string())
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    #[test]
    fn edit_assembler_streams_only_replacement_text_across_split_payloads() {
        let response = "Sure: <replacement>He tried hard.</replacement> This fixes the agreement.\n\nNew story.";
        for split in 0..=response.len() {
            let payloads = [
                content_payload(&response[..split]),
                content_payload(&response[split..]),
                "{\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}".into(),
                "[DONE]".into(),
            ];
            let payloads: Vec<&str> = payloads.iter().map(String::as_str).collect();
            let events = collect_action_upstream(AiAction::Edit, &payloads);
            assert_eq!(delta_text(&events), "He tried hard.", "split at {split}");
            assert_eq!(
                event_types(&events).last(),
                Some(&"done"),
                "split at {split}"
            );
        }
    }

    #[test]
    fn edit_assembler_errors_after_usage_when_replacement_tag_is_missing_or_unclosed() {
        for (text, message) in [
            (
                "He tried hard. This fixes the agreement.",
                "did not include",
            ),
            ("<replacement>He tried hard", "closing"),
        ] {
            let payloads = [
                content_payload(text),
                "{\"choices\":[{\"delta\":{},\"finish_reason\":\"length\"}]}".into(),
                "{\"choices\":[],\"usage\":{\"prompt_tokens\":2,\"completion_tokens\":3}}".into(),
                "[DONE]".into(),
            ];
            let payloads: Vec<&str> = payloads.iter().map(String::as_str).collect();
            let events = collect_action_upstream(AiAction::Edit, &payloads);
            assert_eq!(
                event_types(events.split_last().unwrap().1).last(),
                Some(&"usage")
            );
            match events.last() {
                Some(StreamEvent::Error { error }) => assert!(error.contains(message), "{error}"),
                other => panic!("expected error, got {other:?}"),
            }
        }
        let generate = collect_upstream(&[
            &content_payload("<replacement>x</replacement> note"),
            "[DONE]",
        ]);
        assert_eq!(delta_text(&generate), "<replacement>x</replacement> note");
        assert_eq!(event_types(&generate), vec!["delta", "done"]);
    }

    #[test]
    fn edit_request_uses_max_output_tokens_unchanged() {
        use crate::ai::models::{DocumentSlice, SelectionRange, SelectionSlice};
        use crate::ai::segments::{DeltaDocument, DeltaOp, InsertValue};

        let text_delta = |text: &str| DeltaDocument {
            ops: vec![DeltaOp {
                insert: Some(InsertValue::Text(text.into())),
                delete: None,
                retain: None,
                attributes: None,
            }],
        };
        let input = GenerateInput {
            lore: vec![],
            author_guidance: None,
            section_overrides: Default::default(),
            action: AiAction::Edit,
            document: DocumentSlice {
                delta: text_delta("Hello world"),
                text: "Hello world".into(),
            },
            selection: Some(SelectionSlice {
                delta: text_delta("world"),
                text: "world".into(),
                range: SelectionRange {
                    index: 6,
                    length: 5,
                },
            }),
            story_context: None,
            instruction: Some("fix".into()),
            messages: vec![],
            resume_prompt: None,
            max_output_tokens: 777,
        };

        let body = serde_json::to_value(ChatCompletionRequest::from_input(
            "m".into(),
            false,
            Some(ReasoningEffort::Low),
            &input,
        ))
        .unwrap();

        assert_eq!(body["max_tokens"], 777);
        assert_eq!(body["reasoning_effort"], "low");
        let body = serde_json::to_value(ChatCompletionRequest::from_input(
            "m".into(),
            false,
            None,
            &input,
        ))
        .unwrap();
        assert!(body.get("reasoning_effort").is_none());
    }

    #[test]
    fn chat_completion_request_includes_history_and_max_tokens() {
        use crate::ai::models::{AiAction, ConversationMessage, ConversationRole, DocumentSlice};
        use crate::ai::segments::{DeltaDocument, DeltaOp, InsertValue};

        let input = GenerateInput {
            lore: vec![],
            author_guidance: None,
            section_overrides: Default::default(),
            action: AiAction::Generate,
            document: DocumentSlice {
                delta: DeltaDocument {
                    ops: vec![DeltaOp {
                        insert: Some(InsertValue::Text("Doc".into())),
                        delete: None,
                        retain: None,
                        attributes: None,
                    }],
                },
                text: "Doc".into(),
            },
            selection: None,
            story_context: None,
            instruction: Some("continue".into()),
            messages: vec![
                ConversationMessage {
                    role: ConversationRole::User,
                    content: "previous prompt".into(),
                },
                ConversationMessage {
                    role: ConversationRole::Assistant,
                    content: "previous output".into(),
                },
            ],
            resume_prompt: Some("resume".into()),
            max_output_tokens: 777,
        };

        let body = serde_json::to_value(ChatCompletionRequest::from_input(
            "model-a".into(),
            true,
            None,
            &input,
        ))
        .unwrap();

        assert_eq!(body["model"], "model-a");
        assert_eq!(body["stream"], true);
        assert!(body.get("reasoning_effort").is_none());
        let configured = serde_json::to_value(ChatCompletionRequest::from_input(
            "model-a".into(),
            true,
            Some(ReasoningEffort::Low),
            &input,
        ))
        .unwrap();
        assert_eq!(configured["reasoning_effort"], "low");
        assert_eq!(body["stream_options"]["include_usage"], true);
        assert_eq!(body["max_tokens"], 777);
        assert_eq!(body["messages"][1]["role"], "user");
        assert_eq!(body["messages"][1]["content"], "previous prompt");
        assert_eq!(body["messages"][2]["role"], "assistant");
        assert_eq!(body["messages"][3]["role"], "user");
        assert!(body["messages"][3]["content"]
            .as_str()
            .unwrap()
            .contains("resume"));
    }

    #[test]
    fn provider_error_message_prefers_provider_json_message() {
        assert_eq!(
            provider_error_message("{\"error\":{\"message\":\"bad key\"}}", "fallback"),
            "bad key"
        );
    }

    #[test]
    fn read_optional_env_trims_and_drops_empty_values() {
        std::env::set_var("QUILLAI_PROVIDER_TEST", " https://example.com/v1/ ");
        assert_eq!(
            read_optional_env("QUILLAI_PROVIDER_TEST"),
            Some("https://example.com/v1".into())
        );

        std::env::set_var("QUILLAI_PROVIDER_TEST", "   ");
        assert_eq!(read_optional_env("QUILLAI_PROVIDER_TEST"), None);

        std::env::remove_var("QUILLAI_PROVIDER_TEST");
    }
}
