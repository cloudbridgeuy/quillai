use axum::{extract::State, routing::post, Json, Router};
use futures_util::StreamExt;
use quillai_api::ai::{
    models::{AiAction, GenerateInput, GenerateRequest},
    prompt::build_prompt,
    provider::{AiProvider, OpenAiCompatibleConfig, OpenAiCompatibleProvider},
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Deserialize)]
struct Fixture {
    models: Vec<String>,
    cases: Vec<Example>,
}

#[derive(Deserialize)]
struct Example {
    name: String,
    #[serde(default, rename = "authorGuidance")]
    author_guidance: Option<String>,
    input: Value,
    history: Vec<Value>,
    sections: Vec<Section>,
}

#[derive(Deserialize)]
struct Section {
    id: String,
    tags: Vec<String>,
    text: String,
}

async fn capture_request(
    State(sender): State<mpsc::UnboundedSender<Value>>,
    Json(body): Json<Value>,
) -> &'static str {
    sender.send(body).expect("fixture receiver is open");
    concat!(
        "data: {\"choices\":[{\"delta\":{\"content\":\"Note <replacement>Fixed</replacement> more\"}}]}\n\n",
        "data: [DONE]\n\n"
    )
}

#[tokio::test]
async fn shared_prompt_sections_match_build_prompt_and_chat_messages() {
    replay_prompt_fixture(None).await;
}

#[tokio::test]
async fn overridden_text_reaches_provider_request() {
    replay_prompt_fixture(Some("override")).await;
}

#[tokio::test]
async fn closing_tags_in_overrides_reach_provider_escaped() {
    replay_prompt_fixture(Some("escaping-")).await;
}

#[tokio::test]
async fn history_override_preserves_speaker_turns() {
    replay_prompt_fixture(Some("history-order-")).await;
}

#[tokio::test]
async fn edit_overridden_text_reaches_provider_request() {
    replay_prompt_fixture(Some("edit-overrides")).await;
}

async fn replay_prompt_fixture(filter: Option<&str>) {
    let fixture: Fixture =
        serde_json::from_str(include_str!("../../../fixtures/prompt/parity.json"))
            .expect("valid shared prompt fixture");
    let (sender, mut receiver) = mpsc::unbounded_channel();
    let app = Router::new()
        .route("/chat/completions", post(capture_request))
        .with_state(sender);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind fixture provider");
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let mut replayed = 0;
    for example in fixture.cases {
        if filter.is_some_and(|filter| !example.name.contains(filter)) {
            continue;
        }
        replayed += 1;
        let has_lore = example.input["lore"]
            .as_array()
            .is_some_and(|entries| entries.iter().any(|entry| entry["level"] != "leave_out"));
        let mut expected_ids = vec!["system"];
        if example
            .author_guidance
            .as_ref()
            .is_some_and(|text| !text.is_empty())
        {
            expected_ids.push("author_guidance");
        }
        expected_ids.extend(["history", "instruction", "story_context"]);
        if has_lore {
            expected_ids.push("lore");
        }
        expected_ids.extend(["selection", "document"]);
        assert_eq!(
            example
                .sections
                .iter()
                .map(|s| s.id.as_str())
                .collect::<Vec<_>>(),
            expected_ids,
            "{}: section ids",
            example.name
        );
        let request: GenerateRequest = serde_json::from_value(example.input.clone()).unwrap();
        let mut input = GenerateInput::try_from(request).expect("accepted fixture request");
        input.author_guidance = example.author_guidance;
        let history_index = example
            .sections
            .iter()
            .position(|section| section.id == "history")
            .unwrap();
        let expected_prompt: String = example.sections[history_index + 1..]
            .iter()
            .map(|s| s.text.as_str())
            .collect();
        assert_eq!(
            build_prompt(&input),
            expected_prompt,
            "{}: build_prompt",
            example.name
        );
        for section in example
            .sections
            .iter()
            .filter(|section| section.id != "system" && section.id != "history")
        {
            let tags: Vec<_> = section
                .text
                .split('<')
                .skip(1)
                .filter_map(|part| part.split_once('>'))
                .map(|(tag, _)| tag)
                .filter(|tag| !tag.starts_with('/'))
                .collect();
            assert_eq!(tags, section.tags, "{}: {} tags", example.name, section.id);
        }
        assert_eq!(
            example.sections[history_index].text,
            example
                .history
                .iter()
                .map(|message| message["content"].as_str().unwrap())
                .collect::<String>(),
            "{}: history section",
            example.name
        );
        let mut expected_messages =
            vec![json!({"role": "system", "content": example.sections[0].text})];
        if let Some(section) = example
            .sections
            .iter()
            .find(|section| section.id == "author_guidance")
        {
            expected_messages.push(json!({"role": "system", "content": section.text}));
        }
        expected_messages.extend(example.history);
        expected_messages.push(json!({"role": "user", "content": expected_prompt}));

        for model in &fixture.models {
            let provider = OpenAiCompatibleProvider::new(OpenAiCompatibleConfig {
                api_key: "fixture-key".into(),
                model: model.clone(),
                base_url: base_url.clone(),
            })
            .unwrap();
            let mut stream = provider.stream(input.clone());
            let event = tokio::time::timeout(Duration::from_secs(5), stream.next())
                .await
                .expect("fixture provider responds")
                .expect("completion event");
            let inserted = match input.action {
                AiAction::Generate => "Note <replacement>Fixed</replacement> more",
                AiAction::Edit => "Fixed",
            };
            assert_eq!(
                serde_json::to_value(event).unwrap(),
                json!({"type":"delta", "delta":{"ops":[{"insert":inserted}]}})
            );
            let body = receiver.try_recv().expect("captured provider request");
            assert_eq!(
                body["messages"],
                json!(expected_messages),
                "{} ({model}): chat_messages",
                example.name
            );
            assert_eq!(body["model"], *model, "{}: model passthrough", example.name);
            assert_eq!(body["max_tokens"], input.max_output_tokens);
            assert_eq!(body["stream"], true);
        }
    }
    let minimum_cases = match filter {
        Some("history-order-") => 4,
        Some("escaping-") => 2,
        _ => 1,
    };
    assert!(
        replayed >= minimum_cases,
        "fixture coverage must not shrink"
    );
    server.abort();
}
