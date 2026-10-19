//! Faithful port of the quill-delta 5.1.0 pieces that `compose`, `transform` and
//! `transform_position` rely on, working on plain JSON objects with `f64` lengths so
//! that odd-but-accepted ops (negative or fractional counts, several op keys, unknown
//! keys, `"attributes": null`, array attributes, non-object entries, Infinity)
//! behave exactly like the JS library.

use serde_json::{Map, Value};

use crate::error::{DeltaError, SPLIT_SURROGATE_MSG};
use crate::number::js_f64;
pub(crate) use crate::number::num_value;

/// An `ops` entry as quill-delta holds it: usually an object, but any JSON value.
pub(crate) type JOp = Value;

fn num(v: Option<&Value>) -> Option<f64> {
    match v {
        Some(Value::Number(n)) => Some(js_f64(n)),
        _ => None,
    }
}

pub(crate) fn delete_n(op: &JOp) -> Option<f64> {
    num(op.get("delete"))
}

pub(crate) fn retain_n(op: &JOp) -> Option<f64> {
    num(op.get("retain"))
}

/// JS `typeof v === 'object' && v !== null` (objects and arrays).
fn is_js_object(v: Option<&Value>) -> bool {
    matches!(v, Some(Value::Object(_) | Value::Array(_)))
}

/// JS truthiness of a JSON value (`undefined`/absent is falsy).
fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|f| f != 0.0 && !f.is_nan()),
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Array(_) | Value::Object(_)) => true,
    }
}

fn retain_obj(op: &JOp) -> bool {
    is_js_object(op.get("retain"))
}

/// quill-delta's `Op.length`.
pub(crate) fn op_len(op: &JOp) -> f64 {
    if let Some(n) = delete_n(op) {
        n
    } else if let Some(n) = retain_n(op) {
        n
    } else if retain_obj(op) {
        1.0
    } else if let Some(Value::String(s)) = op.get("insert") {
        s.encode_utf16().count() as f64
    } else {
        1.0
    }
}

fn attrs(op: &JOp) -> Option<&Value> {
    op.get("attributes")
}

/// `Object.keys(v)` for a JS object or array (array indices as strings).
fn keys(v: &Value) -> Vec<String> {
    match v {
        Value::Object(m) => m.keys().cloned().collect(),
        Value::Array(a) => (0..a.len()).map(|i| i.to_string()).collect(),
        _ => Vec::new(),
    }
}

/// A canonical JS array index (`"0"`, `"12"`, not `"01"`).
fn array_index(k: &str) -> Option<usize> {
    let i: u32 = k.parse().ok()?;
    (i.to_string() == k && i < u32::MAX).then_some(i as usize)
}

/// `v[k] !== undefined` for a JS object or array (arrays also have `length`).
fn has(v: &Value, k: &str) -> bool {
    match v {
        Value::Object(m) => m.contains_key(k),
        Value::Array(a) => k == "length" || array_index(k).is_some_and(|i| i < a.len()),
        _ => false,
    }
}

/// `v[k]` for a key from `keys(v)`.
fn get<'a>(v: &'a Value, k: &str) -> Option<&'a Value> {
    match v {
        Value::Object(m) => m.get(k),
        Value::Array(a) => array_index(k).and_then(|i| a.get(i)),
        _ => None,
    }
}

/// JS `String.prototype.substr(start, len)` over UTF-16 units.
fn substr_utf16(s: &str, start: f64, len: f64) -> Result<String, DeltaError> {
    let units: Vec<u16> = s.encode_utf16().collect();
    let size = units.len() as f64;
    let int = |x: f64| if x.is_nan() { 0.0 } else { x.trunc() };
    let mut from = int(start);
    if from < 0.0 {
        from = (size + from).max(0.0);
    } else {
        from = from.min(size);
    }
    let count = int(len).clamp(0.0, size);
    let to = (from + count).min(size);
    if from >= to {
        return Ok(String::new());
    }
    if from == 0.0 && to == size {
        return Ok(s.to_string());
    }
    String::from_utf16(&units[from as usize..to as usize])
        .map_err(|_| DeltaError::SplitSurrogate(SPLIT_SURROGATE_MSG.to_string()))
}

/// quill-delta's `OpIterator`. An exhausted iterator yields endless retains.
pub(crate) struct It<'a> {
    ops: &'a [JOp],
    index: usize,
    offset: f64,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub(crate) enum Kind {
    Insert,
    Delete,
    Retain,
}

impl<'a> It<'a> {
    pub(crate) fn new(ops: &'a [JOp]) -> Self {
        It {
            ops,
            index: 0,
            offset: 0.0,
        }
    }

    pub(crate) fn peek(&self) -> Option<&'a JOp> {
        self.ops.get(self.index)
    }

    pub(crate) fn peek_len(&self) -> f64 {
        self.peek()
            .map_or(f64::INFINITY, |o| op_len(o) - self.offset)
    }

    pub(crate) fn has_next(&self) -> bool {
        self.peek_len() < f64::INFINITY
    }

    pub(crate) fn peek_kind(&self) -> Kind {
        match self.peek() {
            Some(op) if delete_n(op).is_some() => Kind::Delete,
            Some(op) if retain_n(op).is_some() || retain_obj(op) => Kind::Retain,
            Some(_) => Kind::Insert,
            None => Kind::Retain,
        }
    }

    /// Skip the current op whole.
    pub(crate) fn skip(&mut self) {
        self.index += 1;
        self.offset = 0.0;
    }

    /// `next(length)`: a falsy length (0, NaN) means "no limit".
    pub(crate) fn next(&mut self, length: f64) -> Result<JOp, DeltaError> {
        let mut length = if length == 0.0 || length.is_nan() {
            f64::INFINITY
        } else {
            length
        };
        let mut out = Map::new();
        let Some(op) = self.peek() else {
            out.insert("retain".into(), num_value(f64::INFINITY));
            return Ok(Value::Object(out));
        };
        let offset = self.offset;
        let op_length = op_len(op);
        if length >= op_length - offset {
            length = op_length - offset;
            self.skip();
        } else {
            self.offset += length;
        }
        if delete_n(op).is_some() {
            out.insert("delete".into(), num_value(length));
            return Ok(Value::Object(out));
        }
        // `if (nextOp.attributes)`: any truthy value is copied as is.
        if truthy(op.get("attributes")) {
            out.insert("attributes".into(), op["attributes"].clone());
        }
        if retain_n(op).is_some() {
            out.insert("retain".into(), num_value(length));
        } else if retain_obj(op) {
            out.insert("retain".into(), op["retain"].clone());
        } else if let Some(Value::String(s)) = op.get("insert") {
            out.insert(
                "insert".into(),
                Value::String(substr_utf16(s, offset, length)?),
            );
        } else if let Some(v) = op.get("insert") {
            out.insert("insert".into(), v.clone());
        }
        Ok(Value::Object(out))
    }

    pub(crate) fn rest(&mut self) -> Result<Vec<JOp>, DeltaError> {
        if !self.has_next() {
            return Ok(Vec::new());
        }
        if self.offset == 0.0 {
            return Ok(self.ops[self.index..].to_vec());
        }
        let (offset, index) = (self.offset, self.index);
        let next = self.next(0.0)?;
        let mut out = vec![next];
        out.extend_from_slice(&self.ops[self.index..]);
        self.offset = offset;
        self.index = index;
        Ok(out)
    }
}

fn attr_eq(a: &JOp, b: &JOp) -> bool {
    a.get("attributes") == b.get("attributes")
}

/// quill-delta's `Delta.push` on a plain op list.
pub(crate) fn push(ops: &mut Vec<JOp>, new: JOp) {
    // `typeof lastOp === 'object'` (`null` would make JS throw).
    let object = |o: &JOp| matches!(o, Value::Object(_) | Value::Array(_) | Value::Null);
    let mut index = ops.len();
    if index > 0 && object(&ops[index - 1]) {
        let last = &ops[index - 1];
        if let (Some(a), Some(b)) = (delete_n(&new), delete_n(last)) {
            let mut m = Map::new();
            m.insert("delete".into(), num_value(a + b));
            ops[index - 1] = Value::Object(m);
            return;
        }
        if delete_n(last).is_some() && new.get("insert").is_some_and(|v| !v.is_null()) {
            index -= 1;
            if index == 0 || !object(&ops[index - 1]) {
                ops.insert(0, new);
                return;
            }
        }
        let last = &ops[index - 1];
        if attr_eq(&new, last) {
            let merged = match (new.get("insert"), last.get("insert")) {
                (Some(Value::String(b)), Some(Value::String(a))) => {
                    Some(("insert", Value::String(format!("{a}{b}"))))
                }
                _ => match (retain_n(&new), retain_n(last)) {
                    (Some(b), Some(a)) => Some(("retain", num_value(a + b))),
                    _ => None,
                },
            };
            if let Some((key, value)) = merged {
                let mut m = Map::new();
                m.insert(key.into(), value);
                if let Some(a) = new.get("attributes") {
                    if a.is_null() || is_js_object(Some(a)) {
                        m.insert("attributes".into(), a.clone());
                    }
                }
                ops[index - 1] = Value::Object(m);
                return;
            }
        }
    }
    ops.insert(index, new);
}

/// quill-delta's `Delta.chop`.
pub(crate) fn chop(ops: &mut Vec<JOp>) {
    if let Some(last) = ops.last() {
        if retain_n(last).is_some() && !truthy(last.get("attributes")) {
            ops.pop();
        }
    }
}

/// `Delta.retain(length, attributes)`: a non-positive number is dropped; attributes
/// are kept if they are an object or array with at least one key.
fn push_retain(ops: &mut Vec<JOp>, length: Value, attributes: Option<Value>) {
    if let Value::Number(n) = &length {
        if js_f64(n) <= 0.0 {
            return;
        }
    }
    let mut m = Map::new();
    m.insert("retain".into(), length);
    if let Some(a) = attributes.filter(|a| !keys(a).is_empty()) {
        m.insert("attributes".into(), a);
    }
    push(ops, Value::Object(m));
}

/// `AttributeMap.compose`. Non-objects count as `{}`; arrays are objects whose keys
/// are their indices. With `keep_null` an array `b` stays an array; keys of `a` that
/// are not indices then land on it as properties `JSON.stringify` leaves out.
fn compose_attrs(a: Option<&Value>, b: Option<&Value>, keep_null: bool) -> Option<Value> {
    let empty = Value::Object(Map::new());
    let a = a.filter(|v| is_js_object(Some(v))).unwrap_or(&empty);
    let b = b.filter(|v| is_js_object(Some(v))).unwrap_or(&empty);
    let from_a = keys(a).into_iter().filter(|k| !has(b, k));
    if let (true, Value::Array(arr)) = (keep_null, b) {
        let mut arr = arr.clone();
        let mut hidden = false;
        for k in from_a {
            let v = get(a, &k).cloned().unwrap_or(Value::Null);
            match array_index(&k) {
                Some(i) => {
                    if i >= arr.len() {
                        // Holes serialize as `null`.
                        arr.resize(i + 1, Value::Null);
                    }
                    arr[i] = v;
                }
                None => hidden = true,
            }
        }
        return (!arr.is_empty() || hidden).then_some(Value::Array(arr));
    }
    let mut out = Map::new();
    for k in keys(b) {
        let v = get(b, &k).expect("own key");
        if keep_null || !v.is_null() {
            out.insert(k, v.clone());
        }
    }
    for k in from_a {
        let v = get(a, &k).expect("own key").clone();
        out.insert(k, v);
    }
    (!out.is_empty()).then_some(Value::Object(out))
}

/// `AttributeMap.transform`.
fn transform_attrs(a: Option<&Value>, b: Option<&Value>, priority: bool) -> Option<Value> {
    let Some(a) = a.filter(|v| is_js_object(Some(v))) else {
        return b.cloned();
    };
    let b = b.filter(|v| is_js_object(Some(v)))?;
    if !priority {
        return Some(b.clone());
    }
    let out: Map<String, Value> = keys(b)
        .into_iter()
        .filter(|k| !has(a, k))
        .map(|k| {
            let v = get(b, &k).expect("own key").clone();
            (k, v)
        })
        .collect();
    (!out.is_empty()).then_some(Value::Object(out))
}

pub(crate) fn compose(a: &[JOp], b: &[JOp]) -> Result<Vec<JOp>, DeltaError> {
    let mut this = It::new(a);
    let mut oth = It::new(b);
    let mut ops: Vec<JOp> = Vec::new();

    if let Some(first) = oth.peek() {
        if let (Some(retain), true) = (
            retain_n(first),
            first.get("attributes").is_none_or(Value::is_null),
        ) {
            let mut left = retain;
            while this.peek_kind() == Kind::Insert && this.peek_len() <= left {
                left -= this.peek_len();
                // quill-delta pushes these onto a plain array: no normalization.
                ops.push(this.next(0.0)?);
            }
            if retain - left > 0.0 {
                oth.next(retain - left)?;
            }
        }
    }

    let mut delta = ops;
    while this.has_next() || oth.has_next() {
        if oth.peek_kind() == Kind::Insert {
            push(&mut delta, oth.next(0.0)?);
        } else if this.peek_kind() == Kind::Delete {
            push(&mut delta, this.next(0.0)?);
        } else {
            let length = this.peek_len().min(oth.peek_len());
            let this_op = this.next(length)?;
            let other_op = oth.next(length)?;
            let other_retain = other_op.get("retain");
            if truthy(other_retain) {
                let mut new_op = Map::new();
                let this_retain = this_op.get("retain");
                let this_is_count = matches!(this_retain, Some(Value::Number(_)));
                if this_is_count {
                    let v = if matches!(other_retain, Some(Value::Number(_))) {
                        num_value(length)
                    } else {
                        other_retain.cloned().unwrap_or(Value::Null)
                    };
                    new_op.insert("retain".into(), v);
                } else if matches!(other_retain, Some(Value::Number(_))) {
                    match this_retain {
                        None => {
                            if let Some(v) = this_op.get("insert") {
                                new_op.insert("insert".into(), v.clone());
                            }
                        }
                        Some(v) => {
                            new_op.insert("retain".into(), v.clone());
                        }
                    }
                } else {
                    // quill-delta throws here: no embed handler is registered.
                    return Err(DeltaError::Unsupported(
                        "composing an embed retain with an embed".into(),
                    ));
                }
                if let Some(a) = compose_attrs(attrs(&this_op), attrs(&other_op), this_is_count) {
                    new_op.insert("attributes".into(), a);
                }
                let new_op = Value::Object(new_op);
                push(&mut delta, new_op.clone());
                if !oth.has_next() && delta.last() == Some(&new_op) {
                    let mut rest = this.rest()?.into_iter();
                    if let Some(first) = rest.next() {
                        push(&mut delta, first);
                        delta.extend(rest);
                    }
                    chop(&mut delta);
                    return Ok(delta);
                }
            } else if delete_n(&other_op).is_some()
                && (retain_n(&this_op).is_some() || retain_obj(&this_op))
            {
                push(&mut delta, other_op);
            }
        }
    }
    chop(&mut delta);
    Ok(delta)
}

pub(crate) fn transform(a: &[JOp], b: &[JOp], priority: bool) -> Vec<JOp> {
    let mut this = It::new(a);
    let mut oth = It::new(b);
    let mut delta: Vec<JOp> = Vec::new();
    while this.has_next() || oth.has_next() {
        if this.peek_kind() == Kind::Insert && (priority || oth.peek_kind() != Kind::Insert) {
            let n = this.peek_len();
            this.skip();
            push_retain(&mut delta, num_value(n), None);
        } else if oth.peek_kind() == Kind::Insert {
            // Inserts are consumed whole, so no text is ever sliced here.
            push(&mut delta, oth.next(0.0).expect("whole op never splits"));
        } else {
            let length = this.peek_len().min(oth.peek_len());
            let this_op = this.next(length).expect("retain/delete never splits");
            let other_op = oth.next(length).expect("retain/delete never splits");
            if truthy(this_op.get("delete")) {
                continue;
            } else if truthy(other_op.get("delete")) {
                push(&mut delta, other_op);
            } else {
                // Two embed retains of one type make quill-delta throw (no handler);
                // here the other side's embed is kept.
                let data = match other_op.get("retain") {
                    Some(v) if is_js_object(Some(v)) => v.clone(),
                    _ => num_value(length),
                };
                let attributes = transform_attrs(attrs(&this_op), attrs(&other_op), priority);
                push_retain(&mut delta, data, attributes);
            }
        }
    }
    chop(&mut delta);
    delta
}

pub(crate) fn transform_position(ops: &[JOp], mut index: f64, priority: bool) -> f64 {
    let mut it = It::new(ops);
    let mut offset = 0.0;
    while it.has_next() && offset <= index {
        let length = it.peek_len();
        let kind = it.peek_kind();
        it.skip();
        if kind == Kind::Delete {
            index -= length.min(index - offset);
            continue;
        } else if kind == Kind::Insert && (offset < index || !priority) {
            index += length;
        }
        offset += length;
    }
    index
}
