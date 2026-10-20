use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use quillai_api::ai::{
    lore::MAX_LORE_BYTES,
    models::{GenerateInput, GenerateRequest},
    prompt::{build_prompt, lore_text},
};
use serde_json::{json, Value};
use tower::ServiceExt;

mod common;

fn request(lore: Value) -> Value {
    json!({"action": "generate", "document": {"delta": {"ops": []}, "text": ""}, "lore": lore})
}

fn parse(lore: Value) -> Result<GenerateInput, quillai_api::error::Error> {
    GenerateInput::try_from(serde_json::from_value::<GenerateRequest>(request(lore)).unwrap())
}

#[test]
fn lore_byte_limit_counts_utf8_and_markers_before_escaping() {
    for level in ["include", "known_dont_reveal"] {
        let base = json!({"name": "世界", "kind": "character", "visible_text": "", "level": level});
        let overhead = lore_text(&parse(json!([base])).unwrap().lore).len();
        let mut entry = base;
        entry["visible_text"] = json!(format!(
            "{}{}",
            "💡".repeat((MAX_LORE_BYTES - overhead) / 4),
            "&".repeat((MAX_LORE_BYTES - overhead) % 4)
        ));
        let input = parse(json!([entry])).unwrap();
        assert_eq!(lore_text(&input.lore).len(), MAX_LORE_BYTES);
        assert!(build_prompt(&input).contains("<lore>"));
        entry["visible_text"] = json!(format!("{}x", entry["visible_text"].as_str().unwrap()));
        assert!(parse(json!([entry]))
            .unwrap_err()
            .to_string()
            .contains("65536 UTF-8 bytes"));
    }
}

#[test]
fn leave_out_is_removed_at_boundary_and_renderer() {
    let entry = json!({"name": "NEVER", "kind": "character", "visible_text": "SECRET".repeat(20000), "level": "leave_out"});
    let mut input = parse(json!([entry])).unwrap();
    assert!(input.lore.is_empty());
    assert!(!build_prompt(&input).contains("<lore>"));
    input.lore = serde_json::from_value(json!([entry])).unwrap();
    assert!(!build_prompt(&input).contains("SECRET"));
    assert!(!build_prompt(&input).contains("<lore>"));
}

#[test]
fn lore_cap_is_shared_across_entries_and_includes_names_and_separators() {
    let entry = json!({"name": "Mara", "kind": "term", "visible_text": "x".repeat(MAX_LORE_BYTES / 2), "level": "include"});
    assert!(parse(json!([entry])).is_ok());
    assert!(parse(json!([entry, entry])).is_err());
    let entry = json!({"name": "x".repeat(MAX_LORE_BYTES), "kind": "term", "visible_text": "", "level": "include"});
    assert!(parse(json!([entry])).is_err());
}

#[tokio::test]
async fn invalid_lore_and_overflow_return_422_before_provider_start() {
    let app = quillai_api::app(common::test_pool().await);
    for lore in [
        json!([{"name":"Mara","kind":"character","visible_text":"x".repeat(MAX_LORE_BYTES),"level":"include"}]),
        json!([{"name":"Mara","kind":"character","visible_text":"safe","level":"include","hidden_notes":"PRIVATE"}]),
        json!([{"name":"Mara","kind":"unknown","visible_text":"safe","level":"include"}]),
        json!([{"name":"Mara","kind":"character","visible_text":"safe","level":"unknown"}]),
        json!(null),
        json!("raw text"),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/ai/generate")
                    .header("content-type", "application/json")
                    .body(Body::from(request(lore).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}
