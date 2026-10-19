use std::cell::RefCell;

use serde_json::Value;

thread_local! {
    static SOURCE: RefCell<Option<String>> = const { RefCell::new(None) };
}

#[cfg(test)]
struct SourceGuard {
    prev: Option<String>,
}

#[cfg(test)]
impl Drop for SourceGuard {
    fn drop(&mut self) {
        let prev = self.prev.take();
        SOURCE.with(|slot| *slot.borrow_mut() = prev);
    }
}

#[cfg(test)]
pub(crate) fn with_source<T>(text: &str, f: impl FnOnce() -> T) -> T {
    let prev = SOURCE.with(|slot| slot.borrow_mut().replace(text.to_string()));
    let _guard = SourceGuard { prev };
    f()
}

pub(crate) fn source_is_split_surrogate() -> bool {
    SOURCE.with(|slot| {
        let Some(text) = slot.borrow().clone() else {
            return false;
        };
        match serde_json::from_str::<Value>(&text) {
            Ok(_) => false,
            Err(err) => failure_is_split_surrogate(&text, &err),
        }
    })
}

pub(crate) fn failure_is_split_surrogate(text: &str, err: &serde_json::Error) -> bool {
    let Some(masked) = mask_lone_surrogates(text) else {
        return false;
    };
    match serde_json::from_str::<Value>(&masked) {
        Ok(_) => true,
        Err(next) => (next.line(), next.column()) > (err.line(), err.column()),
    }
}

const MARK: &[u8] = b"\\u003f";

struct Mask<'a> {
    src: &'a [u8],
    out: Vec<u8>,
    last: usize,
    found: bool,
}

impl Mask<'_> {
    fn replace(&mut self, at: usize) {
        if !self.found {
            self.out = Vec::with_capacity(self.src.len());
        }
        self.out.extend_from_slice(&self.src[self.last..at]);
        self.out.extend_from_slice(MARK);
        self.last = at + MARK.len();
        self.found = true;
    }

    fn finish(self) -> Option<String> {
        if !self.found {
            return None;
        }
        let mut out = self.out;
        out.extend_from_slice(&self.src[self.last..]);
        Some(String::from_utf8(out).expect("surrogate mask stays UTF-8"))
    }
}

fn mask_lone_surrogates(text: &str) -> Option<String> {
    let src = text.as_bytes();
    let mut mask = Mask {
        src,
        out: Vec::new(),
        last: 0,
        found: false,
    };
    let mut i = 0;
    while i < src.len() {
        if src[i] != b'"' {
            i += 1;
            continue;
        }
        i += 1;
        while i < src.len() {
            match src[i] {
                b'"' => {
                    i += 1;
                    break;
                }
                b'\\' => {
                    if !on_escape(&mut mask, &mut i) {
                        return mask.finish();
                    }
                }
                _ => i += 1,
            }
        }
    }
    mask.finish()
}

fn on_escape(mask: &mut Mask<'_>, i: &mut usize) -> bool {
    let src = mask.src;
    let esc = *i;
    let Some(kind) = src.get(esc + 1).copied() else {
        return false;
    };
    match kind {
        b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't' => {
            *i = esc + 2;
            true
        }
        b'u' => on_unicode(mask, i, esc),
        _ => false,
    }
}

fn on_unicode(mask: &mut Mask<'_>, i: &mut usize, esc: usize) -> bool {
    let src = mask.src;
    let Some(unit) = hex4(src, esc + 2) else {
        return false;
    };
    let next = esc + 6;
    if is_high(unit) {
        if src.get(next) == Some(&b'\\') && src.get(next + 1) == Some(&b'u') {
            match hex4(src, next + 2) {
                None => return false,
                Some(low) if is_low(low) => {
                    *i = next + 6;
                    return true;
                }
                Some(_) => {}
            }
        }
        mask.replace(esc);
        *i = next;
        return true;
    }
    if is_low(unit) {
        mask.replace(esc);
        *i = next;
        return true;
    }
    *i = next;
    true
}

fn hex4(src: &[u8], i: usize) -> Option<u16> {
    let bytes = src.get(i..i + 4)?;
    let mut n = 0u16;
    for b in bytes {
        n = (n << 4) | hex_digit(*b)?;
    }
    Some(n)
}

fn hex_digit(b: u8) -> Option<u16> {
    match b {
        b'0'..=b'9' => Some((b - b'0') as u16),
        b'a'..=b'f' => Some((b - b'a' + 10) as u16),
        b'A'..=b'F' => Some((b - b'A' + 10) as u16),
        _ => None,
    }
}

fn is_high(n: u16) -> bool {
    (0xD800..0xDC00).contains(&n)
}

fn is_low(n: u16) -> bool {
    (0xDC00..0xE000).contains(&n)
}
