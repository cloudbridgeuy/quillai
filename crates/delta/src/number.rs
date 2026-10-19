//! JS numbers inside `serde_json` values, so parse/serialize print what
//! `JSON.stringify(new Delta(JSON.parse(text)))` prints.
//!
//! - A whole number up to 2^53 is held as an integer (`1.0` and `1e2` become `1` and
//!   `100`), any other number as a float (`2^63` too: JS has only `f64`), so equal JS
//!   numbers are equal `Value`s.
//! - `serde_json` cannot hold `Infinity` (JSON.parse reads `1e400` as `Infinity`), so
//!   here ±Infinity is the *float* ±1e15. Because of the rule above no parsed number is
//!   a whole float up to 2^53, so that float only ever means Infinity. It serializes
//!   as `null`, like `JSON.stringify(Infinity)`.
//! - [`JsFormatter`] writes the other floats the way JS `Number.prototype.toString` does.

use std::borrow::Cow;
use std::io;

use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};
use serde_json::{Map, Number, Value};

const INF: f64 = 1e15;
const TWO_53: f64 = 9_007_199_254_740_992.0;

fn is_inf_marker(n: &Number) -> bool {
    n.is_f64() && n.as_f64().is_some_and(|f| f.abs() == INF)
}

/// The JS number a JSON number stands for.
pub(crate) fn js_f64(n: &Number) -> f64 {
    let f = n.as_f64().unwrap_or(f64::NAN);
    if is_inf_marker(n) {
        f64::INFINITY.copysign(f)
    } else {
        f
    }
}

/// A JSON value for a JS number (`NaN` has no JSON form and becomes `null`).
pub(crate) fn num_value(f: f64) -> Value {
    if f.is_nan() {
        return Value::Null;
    }
    if f.is_infinite() {
        return Number::from_f64(INF.copysign(f)).map_or(Value::Null, Value::Number);
    }
    if f.fract() == 0.0 && f.abs() <= TWO_53 {
        return Value::Number(Number::from(f as i64));
    }
    Number::from_f64(f).map_or(Value::Null, Value::Number)
}

/// Rewrite every number in `v` to its canonical form (see the module docs). With
/// `keep_inf` false, a whole float ±1e15 is an ordinary number, not Infinity.
pub(crate) fn normalize(v: &mut Value, keep_inf: bool) {
    match v {
        Value::Number(n) if !(keep_inf && is_inf_marker(n)) => {
            *v = num_value(n.as_f64().unwrap_or(f64::NAN));
        }
        Value::Array(a) => a.iter_mut().for_each(|x| normalize(x, keep_inf)),
        Value::Object(m) => m.values_mut().for_each(|x| normalize(x, keep_inf)),
        _ => {}
    }
}

/// Before `serde_json` parses `text`: number literals that overflow to ±Infinity
/// (which `serde_json` rejects, JS reads as `Infinity`) become the Infinity marker,
/// and literals that would themselves read as the marker become integers.
pub(crate) fn prescan(text: &str) -> Cow<'_, str> {
    let b = text.as_bytes();
    let mut out: Option<String> = None;
    let (mut i, mut last) = (0, 0);
    while i < b.len() {
        match b[i] {
            b'"' => {
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    i += if b[i] == b'\\' { 2 } else { 1 };
                }
                i += 1;
            }
            b'-' | b'0'..=b'9' => {
                let start = i;
                while i < b.len() && matches!(b[i], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
                {
                    i += 1;
                }
                let token = &text[start..i];
                let new = match token.parse::<f64>() {
                    Ok(f) if f.is_infinite() => Some(if f > 0.0 { "1e15" } else { "-1e15" }),
                    Ok(f) if f.abs() == INF && token.contains(['.', 'e', 'E']) => {
                        Some(if f > 0.0 {
                            "1000000000000000"
                        } else {
                            "-1000000000000000"
                        })
                    }
                    _ => None,
                };
                if let Some(new) = new {
                    let s = out.get_or_insert_with(String::new);
                    s.push_str(&text[last..start]);
                    s.push_str(new);
                    last = i;
                }
            }
            _ => i += 1,
        }
    }
    match out {
        Some(mut s) => {
            s.push_str(&text[last..]);
            Cow::Owned(s)
        }
        None => Cow::Borrowed(text),
    }
}

/// Serializes a value with the Infinity marker as `null`.
pub(crate) struct Js<'a>(pub &'a Value);

/// Serializes a map with the Infinity marker as `null`.
pub(crate) struct JsMap<'a>(pub &'a Map<String, Value>);

impl Serialize for Js<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            Value::Number(n) if is_inf_marker(n) => s.serialize_unit(),
            Value::Array(a) => {
                let mut seq = s.serialize_seq(Some(a.len()))?;
                for x in a {
                    seq.serialize_element(&Js(x))?;
                }
                seq.end()
            }
            Value::Object(m) => JsMap(m).serialize(s),
            v => v.serialize(s),
        }
    }
}

impl Serialize for JsMap<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut map = s.serialize_map(Some(self.0.len()))?;
        for (k, v) in self.0 {
            map.serialize_entry(k, &Js(v))?;
        }
        map.end()
    }
}

/// JS `Number.prototype.toString` for a finite number.
fn js_number_string(v: f64) -> String {
    if v == 0.0 {
        return "0".into();
    }
    // `{:e}` gives the shortest round-trip digits, like JS.
    let e = format!("{:e}", v.abs());
    let (mant, exp) = e.split_once('e').expect("LowerExp has an exponent");
    let exp: i32 = exp.parse().expect("LowerExp exponent is an integer");
    let digits: String = mant.chars().filter(|c| *c != '.').collect();
    let k = digits.len() as i32;
    let n = exp + 1;
    let body = if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        let sign = if n > 0 { "+" } else { "-" };
        let e = (n - 1).abs();
        match k {
            1 => format!("{digits}e{sign}{e}"),
            _ => format!("{}.{}e{sign}{e}", &digits[..1], &digits[1..]),
        }
    };
    if v < 0.0 {
        format!("-{body}")
    } else {
        body
    }
}

/// `serde_json` formatter that writes floats like `JSON.stringify`.
pub(crate) struct JsFormatter;

impl serde_json::ser::Formatter for JsFormatter {
    fn write_f64<W: ?Sized + io::Write>(&mut self, w: &mut W, value: f64) -> io::Result<()> {
        if value.is_finite() {
            w.write_all(js_number_string(value).as_bytes())
        } else {
            w.write_all(b"null")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_like_js() {
        // Expectations from `String(x)` in JS.
        for (f, js) in [
            (0.5, "0.5"),
            (-1.5, "-1.5"),
            (1e21, "1e+21"),
            (1.5e300, "1.5e+300"),
            (1e20, "100000000000000000000"),
            (0.000001, "0.000001"),
            (1e-7, "1e-7"),
            (1.25e-7, "1.25e-7"),
            (123.456, "123.456"),
            (0.1 + 0.2, "0.30000000000000004"),
        ] {
            assert_eq!(js_number_string(f), js, "{f}");
        }
    }

    #[test]
    fn prescan_rewrites_only_overflow_and_marker() {
        assert_eq!(
            prescan(r#"{"a":"1e400","b":1e400,"c":-1E999,"d":1e15,"e":1.0}"#),
            r#"{"a":"1e400","b":1e15,"c":-1e15,"d":1000000000000000,"e":1.0}"#
        );
    }
}
