use std::num::NonZeroUsize;

use serde::de::{self, Deserializer};
use serde::ser::{SerializeMap, Serializer};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Attribute map. `Value::Null` means "remove this attribute" (as in quill-delta).
pub type Attributes = Map<String, Value>;

/// An embed: a JSON object such as `{"image": "url"}`. Has length 1.
pub type Embed = Map<String, Value>;

/// What an insert op inserts.
#[derive(Debug, Clone, PartialEq)]
pub enum Insert {
    Text(String),
    Embed(Embed),
}

/// What a retain op retains: a count of units or a single embed.
#[derive(Debug, Clone, PartialEq)]
pub enum Retain {
    Count(NonZeroUsize),
    Embed(Embed),
}

/// A single Delta operation.
#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    Insert {
        insert: Insert,
        attributes: Option<Attributes>,
    },
    Retain {
        retain: Retain,
        attributes: Option<Attributes>,
    },
    Delete(NonZeroUsize),
}

impl Op {
    /// Insert text. `None` if the text is empty.
    pub fn insert_text(text: impl Into<String>, attributes: Option<Attributes>) -> Option<Op> {
        let text = text.into();
        (!text.is_empty()).then_some(Op::Insert {
            insert: Insert::Text(text),
            attributes,
        })
    }

    pub fn insert_embed(embed: Embed, attributes: Option<Attributes>) -> Op {
        Op::Insert {
            insert: Insert::Embed(embed),
            attributes,
        }
    }

    /// Retain `n` units. `None` if `n` is zero.
    pub fn retain(n: usize, attributes: Option<Attributes>) -> Option<Op> {
        NonZeroUsize::new(n).map(|n| Op::Retain {
            retain: Retain::Count(n),
            attributes,
        })
    }

    /// Delete `n` units. `None` if `n` is zero.
    pub fn delete(n: usize) -> Option<Op> {
        NonZeroUsize::new(n).map(Op::Delete)
    }

    /// Length in UTF-16 code units (embeds are 1).
    pub fn len(&self) -> usize {
        match self {
            Op::Insert {
                insert: Insert::Text(t),
                ..
            } => t.encode_utf16().count(),
            Op::Insert { .. } => 1,
            Op::Retain {
                retain: Retain::Count(n),
                ..
            } => n.get(),
            Op::Retain { .. } => 1,
            Op::Delete(n) => n.get(),
        }
    }

    /// True for a zero-length op (only possible for hand-built empty text).
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn attributes(&self) -> Option<&Attributes> {
        match self {
            Op::Insert { attributes, .. } | Op::Retain { attributes, .. } => attributes.as_ref(),
            Op::Delete(_) => None,
        }
    }
}

impl Serialize for Op {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let (attrs, n) = match self {
            Op::Insert { attributes, .. } | Op::Retain { attributes, .. } => {
                (attributes.as_ref(), 1 + usize::from(attributes.is_some()))
            }
            Op::Delete(_) => (None, 1),
        };
        let mut m = s.serialize_map(Some(n))?;
        match self {
            Op::Insert {
                insert: Insert::Text(t),
                ..
            } => m.serialize_entry("insert", t)?,
            Op::Insert {
                insert: Insert::Embed(e),
                ..
            } => m.serialize_entry("insert", e)?,
            Op::Retain {
                retain: Retain::Count(c),
                ..
            } => m.serialize_entry("retain", &c.get())?,
            Op::Retain {
                retain: Retain::Embed(e),
                ..
            } => m.serialize_entry("retain", e)?,
            Op::Delete(c) => m.serialize_entry("delete", &c.get())?,
        }
        if let Some(a) = attrs {
            m.serialize_entry("attributes", a)?;
        }
        m.end()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOp {
    insert: Option<Value>,
    retain: Option<Value>,
    delete: Option<Value>,
    attributes: Option<Attributes>,
}

fn count<E: de::Error>(key: &str, v: Value) -> Result<NonZeroUsize, E> {
    v.as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .and_then(NonZeroUsize::new)
        .ok_or_else(|| E::custom(format!("`{key}` must be a positive integer, got {v}")))
}

/// Parse result of one wire op: a real op, or a zero-length no-op
/// (`{"insert":""}`, `{"retain":0}`, `{"delete":0}`) that normalization drops.
pub(crate) struct WireOp(pub(crate) Option<Op>);

impl<'de> Deserialize<'de> for WireOp {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<WireOp, D::Error> {
        parse_op(RawOp::deserialize(d)?).map(WireOp)
    }
}

impl<'de> Deserialize<'de> for Op {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Op, D::Error> {
        parse_op(RawOp::deserialize(d)?)?
            .ok_or_else(|| de::Error::custom("zero-length op is a no-op, not an `Op`"))
    }
}

fn zero_ok(v: &Value) -> bool {
    v.as_u64() == Some(0)
}

fn parse_op<E: de::Error>(raw: RawOp) -> Result<Option<Op>, E> {
    {
        let present = [&raw.insert, &raw.retain, &raw.delete]
            .iter()
            .filter(|x| x.is_some())
            .count();
        if present != 1 {
            return Err(E::custom(
                "op must have exactly one of `insert`, `retain`, `delete`",
            ));
        }
        if raw.delete.is_some() && raw.attributes.is_some() {
            return Err(E::custom("`delete` op cannot have `attributes`"));
        }
        let attributes = raw.attributes;
        if let Some(v) = raw.insert {
            return match v {
                Value::String(s) if s.is_empty() => Ok(None),
                Value::String(s) => Ok(Some(Op::Insert {
                    insert: Insert::Text(s),
                    attributes,
                })),
                Value::Object(e) => Ok(Some(Op::Insert {
                    insert: Insert::Embed(e),
                    attributes,
                })),
                other => Err(E::custom(format!(
                    "`insert` must be a string or object, got {other}"
                ))),
            };
        }
        if let Some(v) = raw.retain {
            if zero_ok(&v) {
                return Ok(None);
            }
            let retain = match v {
                Value::Object(e) => Retain::Embed(e),
                other => Retain::Count(count("retain", other)?),
            };
            return Ok(Some(Op::Retain { retain, attributes }));
        }
        match raw.delete {
            Some(v) if zero_ok(&v) => Ok(None),
            Some(v) => Ok(Some(Op::Delete(count("delete", v)?))),
            None => unreachable!("exactly one key checked above"),
        }
    }
}
