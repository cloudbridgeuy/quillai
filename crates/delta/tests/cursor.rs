use quillai_delta::{CursorHint, CursorRange, Delta};
use serde_json::json;

#[test]
fn cursor_hint_wire_shapes_match_js() {
    for wire in [
        json!(1.5),
        json!({"oldRange":{"index":1,"length":2},"newRange":null}),
        json!({"oldRange":{"index":1,"length":2},"newRange":{"index":2,"length":0}}),
    ] {
        let hint: CursorHint = serde_json::from_value(wire.clone()).unwrap();
        let back = serde_json::to_value(hint).unwrap();
        let again: CursorHint = serde_json::from_value(back).unwrap();
        assert_eq!(again, hint);
    }
}

#[test]
fn selection_keeps_the_selected_replacement_not_the_shortest_edit() {
    let a = Delta::from_json(r#"{"ops":[{"insert":"xxx"}]}"#).unwrap();
    let b = Delta::from_json(r#"{"ops":[{"insert":"xx"}]}"#).unwrap();
    let cursor = CursorHint::Selection {
        old_range: CursorRange {
            index: 1.0,
            length: 2.0,
        },
        new_range: Some(CursorRange {
            index: 2.0,
            length: 0.0,
        }),
    };
    let got = a.diff_with_cursor(&b, Some(cursor)).unwrap();
    assert_eq!(
        got.to_json(),
        r#"{"ops":[{"retain":1},{"insert":"x"},{"delete":2}]}"#
    );
    assert_eq!(a.compose(&got).unwrap(), b);
}

#[test]
fn non_finite_position_hints_fall_back_like_js() {
    let a = Delta::from_json(r#"{"ops":[{"insert":"xxx"}]}"#).unwrap();
    let b = Delta::from_json(r#"{"ops":[{"insert":"xx"}]}"#).unwrap();
    for position in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            a.diff_with_cursor(&b, Some(CursorHint::Position(position)))
                .unwrap(),
            a.diff(&b).unwrap()
        );
    }
}
