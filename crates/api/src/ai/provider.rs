use std::{pin::Pin, str, sync::LazyLock};

use async_stream::stream;
use futures_util::{Stream, StreamExt};
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;

use super::models::{FinishReason, GenerateInput, StreamEvent};
use super::prompt::{build_prompt, escape_prompt_text};
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
        Ok(Self {
            api_key: read_required_env("QUILLAI_AI_API_KEY")?,
            model: read_required_env("QUILLAI_AI_MODEL")?,
            base_url: read_optional_env("QUILLAI_AI_BASE_URL")
                .unwrap_or_else(|| DEFAULT_OPENAI_BASE_URL.into()),
        })
    }
}

#[derive(Clone)]
pub struct OpenAiCompatibleProvider {
    client: Client,
    config: OpenAiCompatibleConfig,
}

impl OpenAiCompatibleProvider {
    pub fn from_env() -> Result<Self, Error> {
        Self::new(OpenAiCompatibleConfig::from_env()?)
    }

    pub fn new(config: OpenAiCompatibleConfig) -> Result<Self, Error> {
        let client = shared_http_client().clone();

        Ok(Self { client, config })
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

        Box::pin(stream! {
            let request_body = ChatCompletionRequest::from_input(config.model.clone(), &input);
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

                for payload in parsed.payloads {
                    if let Some((event, should_stop)) = stream_event_from_payload(parse_stream_payload(&payload)) {
                        yield event;

                        if should_stop {
                            return;
                        }
                    }
                }
            }

            if !utf8_remainder.is_empty() {
                yield StreamEvent::Error {
                    error: "AI provider stream ended with an incomplete UTF-8 sequence.".into(),
                };
                return;
            }

            for payload in finalize_sse_remainder(&sse_remainder).payloads {
                if let Some((event, should_stop)) = stream_event_from_payload(parse_stream_payload(&payload)) {
                    yield event;

                    if should_stop {
                        return;
                    }
                }
            }

            yield StreamEvent::Error {
                error: "AI provider stream ended before a completion event arrived.".into(),
            };
        })
    }
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    stream: bool,
    max_tokens: usize,
    messages: Vec<ChatMessage>,
}

impl ChatCompletionRequest {
    fn from_input(model: String, input: &GenerateInput) -> Self {
        let current_turn = build_prompt(input);
        Self {
            model,
            stream: true,
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
    use super::models::PromptSectionId;

    let history: Vec<ChatMessage> = match &input.section_overrides.history {
        Some(messages) => messages
            .iter()
            .map(|message| ChatMessage {
                role: message.role.as_str().into(),
                content: message.text.clone(),
            })
            .collect(),
        None => input
            .messages
            .iter()
            .map(|message| ChatMessage {
                role: message.role.as_str().into(),
                content: message.content.clone(),
            })
            .collect(),
    };
    std::iter::once(ChatMessage {
        role: "system".into(),
        content: input
            .section_overrides
            .text
            .get(&PromptSectionId::System)
            .map(|text| escape_prompt_text(text))
            .unwrap_or_else(|| SYSTEM_PROMPT.into()),
    })
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

#[derive(Debug, PartialEq, Eq)]
enum StreamPayload {
    Text(String),
    Done(FinishReason),
    Skip,
    Error(String),
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

fn parse_stream_payload(payload: &str) -> StreamPayload {
    if payload == "[DONE]" {
        return StreamPayload::Done(FinishReason::Unknown);
    }

    let value: Value = match serde_json::from_str(payload) {
        Ok(value) => value,
        Err(err) => {
            return StreamPayload::Error(format!(
                "AI provider returned an invalid stream payload: {err}"
            ));
        }
    };

    if let Some(error) = extract_provider_error(&value) {
        return StreamPayload::Error(error);
    }

    match extract_chunk_text(&value) {
        Some(text) if !text.is_empty() => StreamPayload::Text(text),
        Some(_) => StreamPayload::Skip,
        None => extract_finish_reason(&value)
            .map(StreamPayload::Done)
            .unwrap_or(StreamPayload::Skip),
    }
}

fn stream_event_from_payload(payload: StreamPayload) -> Option<(StreamEvent, bool)> {
    match payload {
        StreamPayload::Text(text) => Some((
            StreamEvent::Delta {
                delta: into_delta(&TaggedText::generated(&text)),
            },
            false,
        )),
        StreamPayload::Done(finish_reason) => Some((StreamEvent::Done { finish_reason }, true)),
        StreamPayload::Skip => None,
        StreamPayload::Error(error) => Some((StreamEvent::Error { error }, true)),
    }
}

fn extract_provider_error(value: &Value) -> Option<String> {
    let error = value.get("error")?;
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
            StreamPayload::Text("Hello".into())
        );
    }

    #[test]
    fn parse_stream_payload_extracts_finish_reason() {
        assert_eq!(
            parse_stream_payload("{\"choices\":[{\"delta\":{},\"finish_reason\":\"length\"}]}"),
            StreamPayload::Done(FinishReason::Length)
        );
    }

    #[test]
    fn parse_stream_payload_recognizes_done_marker() {
        assert_eq!(
            parse_stream_payload("[DONE]"),
            StreamPayload::Done(FinishReason::Unknown)
        );
    }

    #[test]
    fn parse_stream_payload_surfaces_provider_errors() {
        assert_eq!(
            parse_stream_payload("{\"error\":{\"message\":\"bad key\"}}"),
            StreamPayload::Error("bad key".into())
        );
    }

    #[test]
    fn chat_completion_request_includes_history_and_max_tokens() {
        use crate::ai::models::{AiAction, ConversationMessage, ConversationRole, DocumentSlice};
        use crate::ai::segments::{DeltaDocument, DeltaOp, InsertValue};

        let input = GenerateInput {
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

        let body =
            serde_json::to_value(ChatCompletionRequest::from_input("model-a".into(), &input))
                .unwrap();

        assert_eq!(body["model"], "model-a");
        assert_eq!(body["stream"], true);
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
