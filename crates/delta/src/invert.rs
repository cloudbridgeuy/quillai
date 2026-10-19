use serde_json::{Map, Value};

use crate::js::{chop, num_value, op_len, push, push_retain, retain_n, truthy, It, JOp};
use crate::{Delta, DeltaError, Op};

fn slice(base: &[JOp], start: f64, end: f64) -> Result<Vec<JOp>, DeltaError> {
    let mut iter = It::new(base);
    let mut index = 0.0;
    let mut ops = Vec::new();
    while index < end && iter.has_next() {
        let skip = index < start;
        let next = iter.next(if skip { start - index } else { end - index })?;
        index += op_len(&next);
        if !skip {
            ops.push(next);
        }
    }
    Ok(ops)
}

#[derive(PartialEq)]
enum AttrValue {
    Json(Value),
    Surrogate(u16),
}

fn attr_values(value: Option<&Value>) -> std::collections::BTreeMap<String, AttrValue> {
    match value {
        Some(Value::Object(m)) => m
            .iter()
            .map(|(k, v)| (k.clone(), AttrValue::Json(v.clone())))
            .collect(),
        Some(Value::Array(a)) => a
            .iter()
            .enumerate()
            .map(|(i, v)| (i.to_string(), AttrValue::Json(v.clone())))
            .collect(),
        Some(Value::String(s)) => s
            .encode_utf16()
            .enumerate()
            .map(|(i, c)| {
                (
                    i.to_string(),
                    String::from_utf16(&[c]).map_or(AttrValue::Surrogate(c), |s| {
                        AttrValue::Json(Value::String(s))
                    }),
                )
            })
            .collect(),
        _ => std::collections::BTreeMap::new(),
    }
}

fn invert_attrs(attr: Option<&Value>, base: Option<&Value>) -> Result<Value, DeltaError> {
    let attr = attr_values(attr);
    let base = attr_values(base);
    let mut out = Map::new();
    for (k, b) in &base {
        if let Some(a) = attr.get(k) {
            if a != b || matches!(a, AttrValue::Json(Value::Object(_) | Value::Array(_))) {
                match b {
                    AttrValue::Json(b) => {
                        out.insert(k.clone(), b.clone());
                    }
                    AttrValue::Surrogate(_) => {
                        return Err(DeltaError::SplitSurrogate(
                            crate::error::SPLIT_SURROGATE_MSG.into(),
                        ))
                    }
                }
            }
        }
    }
    for k in attr.keys() {
        if !base.contains_key(k) {
            out.insert(k.clone(), Value::Null);
        }
    }
    Ok(Value::Object(out))
}

pub(crate) fn push_delete(ops: &mut Vec<JOp>, length: f64) {
    if length > 0.0 {
        push(ops, serde_json::json!({"delete": num_value(length)}));
    }
}

enum Index {
    Number(f64),
    Text(String),
}

fn number_text(n: f64) -> String {
    if n.is_nan() {
        return "NaN".into();
    }
    if n.is_infinite() {
        return if n > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    let mut out = Vec::new();
    let mut serializer =
        serde_json::Serializer::with_formatter(&mut out, crate::number::JsFormatter);
    serde::Serialize::serialize(&num_value(n), &mut serializer).expect("number serialization");
    String::from_utf8(out).expect("JSON is UTF-8")
}

fn value_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Array(a) => a
            .iter()
            .map(|v| {
                if v.is_null() {
                    String::new()
                } else {
                    value_text(v)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".into(),
        Value::Number(n) => number_text(crate::number::js_f64(n)),
        _ => v.to_string(),
    }
}

impl Index {
    fn from_value(v: &Value) -> Self {
        match v {
            Value::Number(n) => Self::Number(crate::number::js_f64(n)),
            Value::Bool(b) => Self::Number(if *b { 1.0 } else { 0.0 }),
            Value::Null => Self::Number(0.0),
            _ => Self::Text(value_text(v)),
        }
    }

    fn text(&self) -> String {
        match self {
            Self::Number(n) => number_text(*n),
            Self::Text(s) => s.clone(),
        }
    }

    fn add(&self, other: &Self) -> Self {
        match (self, other) {
            (Self::Number(a), Self::Number(b)) => Self::Number(a + b),
            _ => Self::Text(self.text() + &other.text()),
        }
    }

    fn number(&self) -> f64 {
        match self {
            Self::Number(n) => *n,
            Self::Text(s) => {
                let s = s.trim_matches(|c: char| matches!(c as u32, 0x0009..=0x000d | 0x0020 | 0x00a0 | 0x1680 | 0x2000..=0x200a | 0x2028 | 0x2029 | 0x202f | 0x205f | 0x3000 | 0xfeff));
                if s.is_empty() {
                    return 0.0;
                }
                for (prefix, radix) in [
                    ("0x", 16),
                    ("0X", 16),
                    ("0b", 2),
                    ("0B", 2),
                    ("0o", 8),
                    ("0O", 8),
                ] {
                    if let Some(digits) = s.strip_prefix(prefix) {
                        return if digits.is_empty() {
                            f64::NAN
                        } else {
                            digits
                                .chars()
                                .try_fold(0.0, |n, c| {
                                    c.to_digit(radix).map(|d| n * radix as f64 + d as f64)
                                })
                                .unwrap_or(f64::NAN)
                        };
                    }
                }
                if matches!(s, "inf" | "+inf" | "-inf") {
                    f64::NAN
                } else {
                    s.parse().unwrap_or(f64::NAN)
                }
            }
        }
    }
}

impl Delta {
    pub fn invert(&self, base: &Delta) -> Result<Delta, DeltaError> {
        let base: Vec<_> = base.ops.iter().map(Op::to_js).collect();
        let mut index = Index::Number(0.0);
        let mut out = Vec::new();
        for op in self.ops.iter().map(Op::to_js) {
            let attr = op.get("attributes");
            if truthy(op.get("insert")) {
                push_delete(&mut out, op_len(&op));
            } else if let Some(n) = retain_n(&op).filter(|_| attr.is_none_or(Value::is_null)) {
                push_retain(&mut out, num_value(n), None);
                index = index.add(&Index::Number(n));
            } else if truthy(op.get("delete")) || retain_n(&op).is_some() {
                let length = if truthy(op.get("delete")) {
                    Index::from_value(&op["delete"])
                } else {
                    Index::Number(retain_n(&op).unwrap_or(f64::NAN))
                };
                let end = index.add(&length);
                for base_op in slice(&base, index.number(), end.number())? {
                    if truthy(op.get("delete")) {
                        push(&mut out, base_op);
                    } else if truthy(op.get("retain")) && truthy(attr) {
                        push_retain(
                            &mut out,
                            num_value(op_len(&base_op)),
                            Some(invert_attrs(attr, base_op.get("attributes"))?),
                        );
                    }
                }
                index = end;
            } else if matches!(op.get("retain"), Some(Value::Object(_) | Value::Array(_))) {
                return Err(DeltaError::Unsupported("inverting an embed retain".into()));
            }
        }
        chop(&mut out);
        Ok(Delta {
            ops: out.into_iter().map(Op::from_js).collect(),
        })
    }
}
