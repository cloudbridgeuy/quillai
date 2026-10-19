use quillai_delta::{Delta, DeltaError};
use serde_json::{json, Value};

fn delta(v: Value) -> Delta {
    Delta::from_json(&v.to_string()).unwrap()
}

fn text(s: &str) -> Delta {
    delta(json!({"ops": [{"insert": s}]}))
}

#[test]
fn semantic_character_diffs_match_fast_diff() {
    for (a, b, expected) in [
        (
            "The cat came.",
            "The came.",
            json!([{"retain":4},{"delete":4}]),
        ),
        (
            "abcxxx",
            "xxxdef",
            json!([{"delete":3},{"retain":3},{"insert":"def"}]),
        ),
        (
            "AAAABBBBCCCCDDDDEEEE",
            "AAAAXXXXCCCCYYYYEEEE",
            json!([{"retain":4},{"insert":"XXXXCCCCYYYY"},{"delete":12}]),
        ),
        (
            "one\n\ntwo",
            "one\n\nnew\n\ntwo",
            json!([{"retain":5},{"insert":"new\n\n"}]),
        ),
        ("😀x😀", "😇x😇", json!([{"insert":"😇x😇"},{"delete":5}])),
    ] {
        let got = text(a).diff(&text(b)).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&got.to_json()).unwrap(),
            json!({"ops":expected})
        );
        assert_eq!(text(a).compose(&got).unwrap(), text(b));
    }
}

#[test]
fn invert_uses_js_string_keys_and_count_coercion() {
    let base = text("a");
    for (attr, expected) in [
        ("😀", json!({"0":null,"1":null})),
        ("abc", json!({"0":null,"1":null,"2":null})),
    ] {
        let got = delta(json!({"ops":[{"retain":1,"attributes":attr}]}))
            .invert(&base)
            .unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&got.to_json()).unwrap(),
            json!({"ops":[{"retain":1,"attributes":expected}]})
        );
    }
    let got = delta(json!({"ops":[{"delete":"2"},{"retain":1},{"delete":1}]}))
        .invert(&text("abcdefghij"))
        .unwrap();
    assert_eq!(got, text("ab"));
}

#[test]
fn invert_restores_nested_attributes_with_js_reference_equality() {
    let base = delta(json!({"ops":[{"insert":"a","attributes":{"nested":{"x":1}}}]}));
    let change = delta(json!({"ops":[{"retain":1,"attributes":{"nested":{"x":1}}}]}));
    assert_eq!(change.invert(&base).unwrap(), change);
}

#[test]
fn unsupported_documents_and_embed_handlers_return_errors() {
    let change = delta(json!({"ops":[{"retain":1}]}));
    assert!(matches!(
        change.diff(&text("a")),
        Err(DeltaError::Unsupported(_))
    ));
    assert_eq!(change.diff(&change).unwrap(), Delta::new());
    let embed = delta(json!({"ops":[{"retain":{"image":"new"}}]}));
    assert!(matches!(
        embed.invert(&text("a")),
        Err(DeltaError::Unsupported(_))
    ));
}
