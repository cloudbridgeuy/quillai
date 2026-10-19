use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};

use crate::error::{is_split_surrogate_message, DeltaError, SPLIT_SURROGATE_MSG};
use crate::op::{Insert, Op, Retain};

/// A Quill Delta: an ordered list of ops.
///
/// Serializes as `{"ops":[...]}`. Deserialization keeps every op as given, including
/// zero-length ones (`{"insert":""}`, `{"retain":0}`, `{"delete":0}`), exactly like
/// `new Delta(ops)` in quill-delta; build normalized deltas with [`Delta::push`].
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Delta {
    pub ops: Vec<Op>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeltaWire {
    ops: Vec<Op>,
}

/// The single parse path: `from_json` and `serde_json::from_str::<Delta>` both
/// go through here, so split surrogate pairs get the same error wording.
impl<'de> Deserialize<'de> for Delta {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Delta, D::Error> {
        let wire = DeltaWire::deserialize(d).map_err(|e| {
            if is_split_surrogate_message(&e.to_string()) {
                de::Error::custom(SPLIT_SURROGATE_MSG)
            } else {
                e
            }
        })?;
        Ok(Delta { ops: wire.ops })
    }
}

impl Delta {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse a Delta from Quill JSON, with clear errors (including split surrogate pairs).
    pub fn from_json(json: &str) -> Result<Delta, DeltaError> {
        serde_json::from_str(json).map_err(|e| {
            if e.to_string().contains(SPLIT_SURROGATE_MSG) {
                DeltaError::SplitSurrogate(e.to_string())
            } else {
                DeltaError::Json(e)
            }
        })
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("Delta serialization is infallible")
    }

    /// quill-delta `length()`: the sum of [`Op::len`] in JS numbers.
    pub fn len(&self) -> f64 {
        self.ops.iter().map(Op::len).fold(0.0, |a, b| a + b)
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// Push an op, normalizing like quill-delta's `Delta.push` (which, like Rust here,
    /// does NOT drop zero-length ops; only the `insert()/retain()/delete()` builders
    /// do): adjacent deletes merge, an insert goes
    /// before a trailing delete, and adjacent text inserts / count retains
    /// with equal attributes merge.
    pub fn push(&mut self, op: Op) -> &mut Self {
        let tail = self.ops.len().saturating_sub(2);
        if matches!(op, Op::Raw(_)) || self.ops[tail..].iter().any(|o| matches!(o, Op::Raw(_))) {
            // Odd ops (see `Op::Raw`) go through the exact JS port.
            let mut js_tail: Vec<_> = self.ops[tail..].iter().map(Op::to_js).collect();
            crate::js::push(&mut js_tail, op.to_js());
            self.ops.truncate(tail);
            self.ops.extend(js_tail.into_iter().map(Op::from_js));
            return self;
        }
        let mut index = self.ops.len();
        if let Some(Op::Delete(last)) = self.ops.last() {
            match &op {
                Op::Delete(n) => {
                    let sum = last.checked_add(*n).expect("delete length overflow");
                    *self.ops.last_mut().expect("non-empty") = Op::Delete(sum);
                    return self;
                }
                Op::Insert { .. } => index -= 1,
                Op::Retain { .. } | Op::Raw(_) => {}
            }
        }
        if index > 0 {
            let prev = &mut self.ops[index - 1];
            if prev.attributes() == op.attributes() {
                match (prev, &op) {
                    (
                        Op::Insert {
                            insert: Insert::Text(a),
                            ..
                        },
                        Op::Insert {
                            insert: Insert::Text(b),
                            ..
                        },
                    ) => {
                        a.push_str(b);
                        return self;
                    }
                    (
                        Op::Retain {
                            retain: Retain::Count(a),
                            ..
                        },
                        Op::Retain {
                            retain: Retain::Count(b),
                            ..
                        },
                    ) => {
                        *a = a.checked_add(*b).expect("retain length overflow");
                        return self;
                    }
                    _ => {}
                }
            }
        }
        self.ops.insert(index, op);
        self
    }
}

impl FromIterator<Op> for Delta {
    fn from_iter<I: IntoIterator<Item = Op>>(iter: I) -> Self {
        let mut d = Delta::new();
        for op in iter {
            d.push(op);
        }
        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DeltaError;
    use serde_json::json;

    fn text(s: &str) -> Op {
        Op::insert_text(s, None).unwrap()
    }
    fn bold() -> Option<crate::Attributes> {
        json!({"bold": true}).as_object().cloned()
    }

    #[test]
    fn merges_text_and_retain() {
        let mut d = Delta::new();
        d.push(text("a")).push(text("b"));
        d.push(Op::retain(1, None).unwrap())
            .push(Op::retain(2, None).unwrap());
        assert_eq!(d.to_json(), r#"{"ops":[{"insert":"ab"},{"retain":3}]}"#);
    }

    #[test]
    fn differing_attributes_do_not_merge() {
        let mut d = Delta::new();
        d.push(text("a"))
            .push(Op::insert_text("b", bold()).unwrap());
        assert_eq!(d.ops.len(), 2);
    }

    #[test]
    fn insert_goes_before_delete_and_merges() {
        let mut d = Delta::new();
        d.push(text("a"))
            .push(Op::delete(2).unwrap())
            .push(text("b"));
        d.push(Op::delete(1).unwrap());
        assert_eq!(d.to_json(), r#"{"ops":[{"insert":"ab"},{"delete":3}]}"#);
        let mut d = Delta::new();
        d.push(Op::delete(1).unwrap()).push(text("x"));
        assert_eq!(d.to_json(), r#"{"ops":[{"insert":"x"},{"delete":1}]}"#);
    }

    #[test]
    fn push_keeps_zero_length_ops_like_quill_delta() {
        // quill-delta: `push` keeps them, only the insert()/retain()/delete() builders drop.
        let mut d = Delta::new();
        d.push(Op::Insert {
            insert: Insert::Text(String::new()),
            attributes: None,
        });
        assert_eq!(d.to_json(), r#"{"ops":[{"insert":""}]}"#);
        assert!(Op::insert_text("", None).is_none());
        assert!(Op::retain(0, None).is_none());
        assert!(Op::delete(0).is_none());
        let mut d = Delta::new();
        d.push(text("😀日"));
        assert_eq!(d.len(), 3.0);
    }

    #[test]
    fn any_object_is_an_op_like_quill_delta() {
        // quill-delta 5.1.0 accepts these; Op.length falls back to 1 for all of them.
        for wire in [
            r#"{"ops":[{}]}"#,
            r#"{"ops":[{"insert":5}]}"#,
            r#"{"ops":[{"retain":"3"}]}"#,
            r#"{"ops":[{"delete":null}]}"#,
            r#"{"ops":[{"insert":"a","attributes":5}]}"#,
        ] {
            let d = Delta::from_json(wire).unwrap();
            assert!(matches!(d.ops[0], Op::Raw(_)), "{wire}");
            let back: serde_json::Value = serde_json::from_str(&d.to_json()).unwrap();
            assert_eq!(back, serde_json::from_str::<serde_json::Value>(wire).unwrap());
            assert_eq!(d.len(), 1.0, "{wire}");
        }
        assert!(Delta::from_json(r#"{"ops":[5]}"#).is_err());
    }

    #[test]
    fn lengths_are_js_numbers() {
        let d = Delta::from_json(r#"{"ops":[{"retain":-2},{"delete":1.5},{"insert":"ab"}]}"#)
            .unwrap();
        assert_eq!(d.ops[0].len(), -2.0);
        assert_eq!(d.ops[1].len(), 1.5);
        assert_eq!(d.len(), 1.5);
    }

    #[test]
    fn odd_ops_are_kept_verbatim_like_quill_delta() {
        // quill-delta 5.1.0 accepts and round-trips all of these.
        for wire in [
            r#"{"ops":[{"retain":-1}]}"#,
            r#"{"ops":[{"delete":-2},{"retain":1.5}]}"#,
            r#"{"ops":[{"insert":"a","delete":1}]}"#,
            r#"{"ops":[{"insert":"a","foo":{"x":[1]}}]}"#,
            r#"{"ops":[{"insert":"a","attributes":null}]}"#,
            r#"{"ops":[{"delete":1,"attributes":{"bold":true}}]}"#,
            r#"{"ops":[{"retain":{"image":"x"},"attributes":null}]}"#,
        ] {
            let d = Delta::from_json(wire).unwrap();
            // Object key order is not preserved (serde_json sorts keys); content is.
            let back: serde_json::Value = serde_json::from_str(&d.to_json()).unwrap();
            assert_eq!(
                back,
                serde_json::from_str::<serde_json::Value>(wire).unwrap()
            );
        }
    }

    #[test]
    fn zero_length_ops_are_kept_verbatim_like_quill_delta() {
        // Each expectation was produced with quill-delta 5.1.0 (`new Delta(ops)`).
        for wire in [
            r#"{"ops":[{"insert":""}]}"#,
            r#"{"ops":[{"retain":0}]}"#,
            r#"{"ops":[{"delete":0}]}"#,
            r#"{"ops":[{"insert":"","attributes":{"bold":true}}]}"#,
            r#"{"ops":[{"insert":""},{"retain":0},{"delete":0},{"insert":"a"}]}"#,
        ] {
            let d = Delta::from_json(wire).unwrap();
            assert_eq!(d.to_json(), wire, "{wire}");
            let d2: Delta = serde_json::from_str(wire).unwrap();
            assert_eq!(d, d2);
        }
    }

    #[test]
    fn zero_length_ops_in_compose_transform_match_quill_delta() {
        let d = |w: &str| Delta::from_json(w).unwrap();
        let ab = d(r#"{"ops":[{"insert":"ab"}]}"#);
        // JS: new Delta([{insert:"ab"}]).compose(new Delta([{insert:""},{retain:1}])).ops
        let got = ab
            .compose(&d(r#"{"ops":[{"insert":""},{"retain":1}]}"#))
            .unwrap();
        assert_eq!(got.to_json(), r#"{"ops":[{"insert":"ab"}]}"#);
        // JS: ... .compose(new Delta([{retain:0},{delete:0},{retain:1}])).ops
        let got = ab
            .compose(&d(r#"{"ops":[{"retain":0},{"delete":0},{"retain":1}]}"#))
            .unwrap();
        assert_eq!(got.to_json(), r#"{"ops":[{"delete":0}]}"#);
        // JS: new Delta([{insert:"",},{retain:0},{insert:"a"}]).transform(new Delta([{retain:0},{insert:"b"}]), true).ops
        let a = d(r#"{"ops":[{"insert":""},{"retain":0},{"insert":"a"}]}"#);
        let got = a.transform(&d(r#"{"ops":[{"retain":0},{"insert":"b"}]}"#), true);
        assert_eq!(got.to_json(), r#"{"ops":[{"retain":1},{"insert":"b"}]}"#);
        // JS: .transformPosition(0) === 1
        assert_eq!(a.transform_position(0.0, false), 1.0);
    }

    #[test]
    fn escaped_backslash_u_is_not_a_surrogate() {
        let wire = r#"{"ops":[{"insert":"\\ud83d"}]}"#;
        let d = Delta::from_json(wire).unwrap();
        assert_eq!(d.ops, vec![text("\\ud83d")]);
        assert_eq!(d.len(), 6.0);
        assert_eq!(d.to_json(), wire);
    }

    #[test]
    fn both_parse_paths_give_same_split_surrogate_error() {
        for wire in [
            r#"{"ops":[{"insert":"a\ud83d"},{"insert":"\ude00b"}]}"#,
            r#"{"ops":[{"insert":"\ude00"}]}"#,
            r#"{"ops":[{"insert":"\ud83dx"}]}"#,
        ] {
            let a = Delta::from_json(wire).unwrap_err();
            assert!(matches!(a, DeltaError::SplitSurrogate(_)), "{wire}");
            let b = serde_json::from_str::<Delta>(wire).unwrap_err();
            assert_eq!(a.to_string(), b.to_string(), "{wire}");
            assert!(a.to_string().contains("splits a UTF-16 surrogate pair"));
        }
    }
}
