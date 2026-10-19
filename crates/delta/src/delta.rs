use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};

use crate::error::{is_split_surrogate_message, DeltaError, SPLIT_SURROGATE_MSG};
use crate::op::{Insert, Op, Retain, WireOp};

/// A Quill Delta: an ordered list of ops.
///
/// Serializes as `{"ops":[...]}`. Deserialization drops zero-length no-op ops
/// (`{"insert":""}`, `{"retain":0}`, `{"delete":0}`) but otherwise keeps ops as
/// given; build fully normalized deltas with [`Delta::push`].
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Delta {
    pub ops: Vec<Op>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeltaWire {
    ops: Vec<WireOp>,
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
        Ok(Delta {
            ops: wire.ops.into_iter().filter_map(|w| w.0).collect(),
        })
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

    /// Total length in UTF-16 code units.
    pub fn len(&self) -> usize {
        self.ops.iter().map(Op::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// Push an op, normalizing like quill-delta's `Delta.push`:
    /// zero-length ops are dropped, adjacent deletes merge, an insert goes
    /// before a trailing delete, and adjacent text inserts / count retains
    /// with equal attributes merge.
    pub fn push(&mut self, op: Op) -> &mut Self {
        if op.is_empty() {
            return self;
        }
        let mut index = self.ops.len();
        if let Some(Op::Delete(last)) = self.ops.last() {
            match &op {
                Op::Delete(n) => {
                    let sum = last.checked_add(n.get()).expect("delete length overflow");
                    *self.ops.last_mut().expect("non-empty") = Op::Delete(sum);
                    return self;
                }
                Op::Insert { .. } => index -= 1,
                Op::Retain { .. } => {}
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
                        *a = a.checked_add(b.get()).expect("retain length overflow");
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
    fn drops_empty_and_counts_utf16() {
        let mut d = Delta::new();
        d.push(Op::Insert {
            insert: Insert::Text(String::new()),
            attributes: None,
        });
        assert!(d.is_empty());
        d.push(text("😀日"));
        assert_eq!(d.len(), 3);
    }

    #[test]
    fn rejects_bad_ops() {
        for bad in [
            r#"{"ops":[{"retain":-1}]}"#,
            r#"{"ops":[{"delete":0,"attributes":{}}]}"#,
            r#"{"ops":[{"insert":"a","delete":1}]}"#,
            r#"{"ops":[{"delete":1,"attributes":{}}]}"#,
            r#"{"ops":[{}]}"#,
        ] {
            assert!(Delta::from_json(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn zero_length_ops_parse_and_are_dropped() {
        for (wire, expect) in [
            (r#"{"ops":[{"insert":""}]}"#, r#"{"ops":[]}"#),
            (r#"{"ops":[{"retain":0}]}"#, r#"{"ops":[]}"#),
            (r#"{"ops":[{"delete":0}]}"#, r#"{"ops":[]}"#),
            (
                r#"{"ops":[{"insert":"","attributes":{"bold":true}}]}"#,
                r#"{"ops":[]}"#,
            ),
            (
                r#"{"ops":[{"insert":""},{"insert":"a"},{"retain":0},{"retain":2,"attributes":{"bold":true}},{"delete":0},{"delete":1},{"insert":""}]}"#,
                r#"{"ops":[{"insert":"a"},{"retain":2,"attributes":{"bold":true}},{"delete":1}]}"#,
            ),
        ] {
            let d = Delta::from_json(wire).unwrap();
            assert_eq!(d.to_json(), expect, "{wire}");
            let d2: Delta = serde_json::from_str(wire).unwrap();
            assert_eq!(d, d2);
        }
    }

    #[test]
    fn escaped_backslash_u_is_not_a_surrogate() {
        let wire = r#"{"ops":[{"insert":"\\ud83d"}]}"#;
        let d = Delta::from_json(wire).unwrap();
        assert_eq!(d.ops, vec![text("\\ud83d")]);
        assert_eq!(d.len(), 6);
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
