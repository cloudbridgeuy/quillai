use quillai_api::ai::section_overrides::{
    SectionOverrides, MAX_OVERRIDE_SECTION_BYTES, MAX_OVERRIDE_TOTAL_BYTES,
};
use serde_json::json;

#[test]
fn override_size_limits_accept_exact_utf8_boundaries() {
    let full = "💡".repeat(MAX_OVERRIDE_SECTION_BYTES / 4);
    assert_eq!(full.len() * 4, MAX_OVERRIDE_TOTAL_BYTES);
    for value in [
        json!({"system": full, "instruction": full, "story_context": full, "document": full}),
        json!({"system": full, "instruction": full, "story_context": full, "history": [
            {"role": "user", "text": "💡".repeat(MAX_OVERRIDE_SECTION_BYTES / 8)},
            {"role": "assistant", "text": "💡".repeat(MAX_OVERRIDE_SECTION_BYTES / 8)}
        ]}),
    ] {
        assert!(SectionOverrides::try_from(value).is_ok());
    }
}

#[test]
fn empty_override_map_and_empty_history_remain_distinct() {
    assert_eq!(SectionOverrides::try_from(json!({})).unwrap().history, None);
    assert_eq!(
        SectionOverrides::try_from(json!({"history": []}))
            .unwrap()
            .history,
        Some(vec![])
    );
}
