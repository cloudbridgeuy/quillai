use serde_json::{Map, Value};

use crate::invert::push_delete;
use crate::js::{chop, get, keys, num_value, push, push_retain, It};
use crate::{Delta, DeltaError, Op};

fn attributes(a: Option<&Value>, b: Option<&Value>) -> Option<Value> {
    let empty = Value::Object(Map::new());
    let a = a.unwrap_or(&empty);
    let b = b.unwrap_or(&empty);
    let mut out = Map::new();
    for k in keys(a).into_iter().chain(keys(b)) {
        if get(a, &k) != get(b, &k) {
            out.insert(k.clone(), get(b, &k).cloned().unwrap_or(Value::Null));
        }
    }
    (!out.is_empty()).then_some(Value::Object(out))
}

fn text(ops: &[Value]) -> Result<Vec<u16>, DeltaError> {
    let mut out = Vec::new();
    for op in ops {
        match op.get("insert") {
            Some(Value::String(s)) => out.extend(s.encode_utf16()),
            Some(v) if !v.is_null() => out.push(0),
            _ => {
                return Err(DeltaError::Unsupported(
                    "diff called on non-document".into(),
                ))
            }
        }
    }
    Ok(out)
}

impl Delta {
    pub fn diff(&self, other: &Delta) -> Result<Delta, DeltaError> {
        self.diff_with_cursor(other, None)
    }

    pub fn diff_with_cursor(
        &self,
        other: &Delta,
        cursor: Option<usize>,
    ) -> Result<Delta, DeltaError> {
        if std::ptr::eq(self, other) {
            return Ok(Delta::new());
        }
        let a: Vec<_> = self.ops.iter().map(Op::to_js).collect();
        let b: Vec<_> = other.ops.iter().map(Op::to_js).collect();
        let components = crate::text_diff::diff(&text(&a)?, &text(&b)?, cursor);
        let mut a = It::new(&a);
        let mut b = It::new(&b);
        let mut out = Vec::new();
        for (kind, component) in components {
            let mut length = component.len() as f64;
            while length > 0.0 {
                let n = match kind {
                    1 => {
                        let n = b.peek_len().min(length);
                        push(&mut out, b.next(n)?);
                        n
                    }
                    -1 => {
                        let n = a.peek_len().min(length);
                        a.next(n)?;
                        push_delete(&mut out, n);
                        n
                    }
                    _ => {
                        let n = a.peek_len().min(b.peek_len()).min(length);
                        let x = a.next(n)?;
                        let y = b.next(n)?;
                        if x.get("insert") == y.get("insert") {
                            push_retain(
                                &mut out,
                                num_value(n),
                                attributes(x.get("attributes"), y.get("attributes")),
                            );
                        } else {
                            push(&mut out, y);
                            push_delete(&mut out, n);
                        }
                        n
                    }
                };
                length -= n;
            }
        }
        chop(&mut out);
        Ok(Delta {
            ops: out.into_iter().map(Op::from_js).collect(),
        })
    }
}
