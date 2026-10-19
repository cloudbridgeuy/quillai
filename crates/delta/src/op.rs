use serde::de::Deserializer;
use serde::ser::{SerializeMap, Serializer};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::number::{Js, JsMap};

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
    /// Any op object the typed variants cannot hold exactly: more than one of
    /// `insert`/`retain`/`delete` or none of them, values of other types (`{"insert":5}`,
    /// `{"retain":"3"}`), unknown keys, non-object `attributes`, a `delete` with
    /// attributes, or a negative/fractional `retain`/`delete`. The map is kept
    /// verbatim; `compose`, `transform` and `transform_position` treat it the way
    /// quill-delta does (`delete` wins over `retain`, which wins over `insert`).
    ///
    /// A whole float ±1e18 in the map stands for ±Infinity (JSON such as `1e400`,
    /// which JS reads as `Infinity`); it serializes as `null`, like JS.
    Raw(Map<String, Value>),
    /// An `ops` entry that is not an object (`5`, `"s"`, `[1]`, `true`). quill-delta
    /// keeps it as given; it has length 1 and otherwise acts like `{}`.
    NonObject(Value),
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

    /// quill-delta `Op.length`, in JS numbers: UTF-16 code units for text, the count
    /// for `retain`/`delete` exactly as given (negative or fractional too), 1 otherwise.
    pub fn len(&self) -> f64 {
        match self {
            Op::Insert {
                insert: Insert::Text(t),
                ..
            } => t.encode_utf16().count() as f64,
            Op::Insert { .. } => 1.0,
            Op::Retain {
                retain: Retain::Count(n),
                ..
            } => *n as f64,
            Op::Retain { .. } => 1.0,
            Op::Delete(n) => *n as f64,
            Op::Raw(_) | Op::NonObject(_) => crate::js::op_len(&self.to_js()),
        }
    }

    /// True for a zero-length op (`{"insert":""}`, `{"retain":0}`, `{"delete":0}`).
    pub fn is_empty(&self) -> bool {
        self.len() == 0.0
    }

    pub fn attributes(&self) -> Option<&Attributes> {
        match self {
            Op::Insert { attributes, .. } | Op::Retain { attributes, .. } => attributes.as_ref(),
            Op::Delete(_) | Op::NonObject(_) => None,
            Op::Raw(m) => m.get("attributes").and_then(Value::as_object),
        }
    }

    /// True for ops only the JS port handles ([`Op::Raw`], [`Op::NonObject`]).
    pub(crate) fn is_odd(&self) -> bool {
        matches!(self, Op::Raw(_) | Op::NonObject(_))
    }

    /// The op as the plain JSON value quill-delta holds (Infinity markers kept).
    pub(crate) fn to_js(&self) -> Value {
        let mut m = Map::new();
        let attributes = match self {
            Op::Raw(r) => return Value::Object(r.clone()),
            Op::NonObject(v) => return v.clone(),
            Op::Insert { insert, attributes } => {
                let v = match insert {
                    Insert::Text(t) => Value::String(t.clone()),
                    Insert::Embed(e) => Value::Object(e.clone()),
                };
                m.insert("insert".into(), v);
                attributes
            }
            Op::Retain { retain, attributes } => {
                let v = match retain {
                    Retain::Count(n) => Value::from(*n),
                    Retain::Embed(e) => Value::Object(e.clone()),
                };
                m.insert("retain".into(), v);
                attributes
            }
            Op::Delete(n) => {
                m.insert("delete".into(), Value::from(*n));
                &None
            }
        };
        if let Some(a) = attributes {
            m.insert("attributes".into(), Value::Object(a.clone()));
        }
        Value::Object(m)
    }

    /// Typed op when the object has exactly the canonical shape, `Op::Raw` otherwise.
    /// Numbers must already be canonical (see `number.rs`).
    pub(crate) fn from_js(v: Value) -> Op {
        let m = match v {
            Value::Object(m) => m,
            v => return Op::NonObject(v),
        };
        let attributes = match m.get("attributes") {
            None => Some(None),
            Some(Value::Object(a)) => Some(Some(a.clone())),
            Some(_) => None,
        };
        let keys = m.keys().filter(|k| *k != "attributes").count();
        if let (Some(attributes), 1) = (attributes, keys) {
            if let Some(v) = m.get("insert") {
                match v {
                    Value::String(s) => {
                        return Op::Insert {
                            insert: Insert::Text(s.clone()),
                            attributes,
                        }
                    }
                    Value::Object(e) => {
                        return Op::Insert {
                            insert: Insert::Embed(e.clone()),
                            attributes,
                        }
                    }
                    _ => {}
                }
            } else if let Some(v) = m.get("retain") {
                match v {
                    Value::Object(e) => {
                        return Op::Retain {
                            retain: Retain::Embed(e.clone()),
                            attributes,
                        }
                    }
                    v => {
                        if let Some(n) = v.as_u64().and_then(|n| usize::try_from(n).ok()) {
                            return Op::Retain {
                                retain: Retain::Count(n),
                                attributes,
                            };
                        }
                    }
                }
            } else if let Some(n) = m.get("delete").and_then(Value::as_u64) {
                if attributes.is_none() {
                    if let Ok(n) = usize::try_from(n) {
                        return Op::Delete(n);
                    }
                }
            }
        }
        Op::Raw(m)
    }
}

impl Serialize for Op {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Op::Raw(m) => return JsMap(m).serialize(s),
            Op::NonObject(v) => return Js(v).serialize(s),
            _ => {}
        }
        let (attrs, n) = match self {
            Op::Insert { attributes, .. } | Op::Retain { attributes, .. } => {
                (attributes.as_ref(), 1 + usize::from(attributes.is_some()))
            }
            Op::Delete(_) | Op::Raw(_) | Op::NonObject(_) => (None, 1),
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
            } => m.serialize_entry("insert", &JsMap(e))?,
            Op::Retain {
                retain: Retain::Count(c),
                ..
            } => m.serialize_entry("retain", c)?,
            Op::Retain {
                retain: Retain::Embed(e),
                ..
            } => m.serialize_entry("retain", &JsMap(e))?,
            Op::Delete(c) => m.serialize_entry("delete", c)?,
            Op::Raw(_) | Op::NonObject(_) => unreachable!("handled above"),
        }
        if let Some(a) = attrs {
            m.serialize_entry("attributes", &JsMap(a))?;
        }
        m.end()
    }
}

impl<'de> Deserialize<'de> for Op {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Op, D::Error> {
        let mut v = Value::deserialize(d)?;
        crate::number::normalize(&mut v, false);
        Ok(Op::from_js(v))
    }
}
