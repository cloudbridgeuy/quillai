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
    Count(usize),
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
    Delete(usize),
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

    /// Retain `n` units. `None` if `n` is zero (like quill-delta's `retain()` builder).
    /// A zero retain is still a valid op: build `Op::Retain` directly to keep it.
    pub fn retain(n: usize, attributes: Option<Attributes>) -> Option<Op> {
        (n > 0).then_some(Op::Retain {
            retain: Retain::Count(n),
            attributes,
        })
    }

    /// Delete `n` units. `None` if `n` is zero (like quill-delta's `delete()` builder).
    /// A zero delete is still a valid op: build `Op::Delete(0)` to keep it.
    pub fn delete(n: usize) -> Option<Op> {
        (n > 0).then_some(Op::Delete(n))
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
            } => *n,
            Op::Retain { .. } => 1,
            Op::Delete(n) => *n,
        }
    }

    /// True for a zero-length op (`{"insert":""}`, `{"retain":0}`, `{"delete":0}`).
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
            } => m.serialize_entry("retain", c)?,
            Op::Retain {
                retain: Retain::Embed(e),
                ..
            } => m.serialize_entry("retain", e)?,
            Op::Delete(c) => m.serialize_entry("delete", c)?,
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

/// Non-negative count. Zero is kept: quill-delta keeps `{"retain":0}` / `{"delete":0}`.
fn count<E: de::Error>(key: &str, v: Value) -> Result<usize, E> {
    v.as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| E::custom(format!("`{key}` must be a non-negative integer, got {v}")))
}

impl<'de> Deserialize<'de> for Op {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Op, D::Error> {
        parse_op(RawOp::deserialize(d)?)
    }
}

fn parse_op<E: de::Error>(raw: RawOp) -> Result<Op, E> {
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
                Value::String(s) => Ok(Op::Insert {
                    insert: Insert::Text(s),
                    attributes,
                }),
                Value::Object(e) => Ok(Op::Insert {
                    insert: Insert::Embed(e),
                    attributes,
                }),
                other => Err(E::custom(format!(
                    "`insert` must be a string or object, got {other}"
                ))),
            };
        }
        if let Some(v) = raw.retain {
            let retain = match v {
                Value::Object(e) => Retain::Embed(e),
                other => Retain::Count(count("retain", other)?),
            };
            return Ok(Op::Retain { retain, attributes });
        }
        match raw.delete {
            Some(v) => Ok(Op::Delete(count("delete", v)?)),
            None => unreachable!("exactly one key checked above"),
        }
    }
}
