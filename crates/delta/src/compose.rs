//! `compose`, `transform` and `transform_position`, ported from quill-delta 5.1.0.

use crate::error::{DeltaError, SPLIT_SURROGATE_MSG};
use crate::op::{Attributes, Insert, Op, Retain};
use crate::Delta;

#[derive(PartialEq, Clone, Copy)]
enum Kind {
    Insert,
    Delete,
    Retain,
}

/// Port of quill-delta's `OpIterator`. An exhausted iterator yields endless retains.
struct OpIter<'a> {
    ops: &'a [Op],
    index: usize,
    offset: usize,
}

/// UTF-16 slice `[from, to)` of `s`; `None` if a bound cuts a surrogate pair.
fn slice_utf16(s: &str, from: usize, to: usize) -> Option<&str> {
    let (mut pos, mut start, mut end) = (0, None, None);
    for (i, c) in s.char_indices() {
        if pos == from {
            start = Some(i);
        }
        if pos == to {
            end = Some(i);
        }
        pos += c.len_utf16();
    }
    if pos == from {
        start = Some(s.len());
    }
    if pos == to {
        end = Some(s.len());
    }
    Some(&s[start?..end?])
}

impl<'a> OpIter<'a> {
    fn new(ops: &'a [Op]) -> Self {
        OpIter {
            ops,
            index: 0,
            offset: 0,
        }
    }

    fn has_next(&self) -> bool {
        self.index < self.ops.len()
    }

    /// Remaining length of the current op; `usize::MAX` (infinity) when exhausted.
    fn peek_len(&self) -> usize {
        self.ops
            .get(self.index)
            .map_or(usize::MAX, |o| o.len() - self.offset)
    }

    fn peek_kind(&self) -> Kind {
        match self.ops.get(self.index) {
            Some(Op::Insert { .. }) => Kind::Insert,
            Some(Op::Delete(_)) => Kind::Delete,
            _ => Kind::Retain,
        }
    }

    /// Take up to `max` units of the current op. Like quill-delta, `max == 0` means
    /// "no limit" (so a zero-length op is consumed whole, and `next(0)` takes the
    /// whole current op).
    fn next(&mut self, max: usize) -> Result<Op, DeltaError> {
        let max = if max == 0 { usize::MAX } else { max };
        let Some(op) = self.ops.get(self.index) else {
            return Ok(Op::Retain {
                retain: Retain::Count(max),
                attributes: None,
            });
        };
        let remaining = op.len() - self.offset;
        let take = max.min(remaining);
        let from = self.offset;
        if take >= remaining {
            self.index += 1;
            self.offset = 0;
        } else {
            self.offset += take;
        }
        Ok(match op {
            Op::Delete(_) => Op::Delete(take),
            Op::Retain {
                retain: Retain::Count(_),
                attributes,
            } => Op::Retain {
                retain: Retain::Count(take),
                attributes: attributes.clone(),
            },
            Op::Insert {
                insert: Insert::Text(t),
                attributes,
            } => {
                let part = slice_utf16(t, from, from + take)
                    .ok_or_else(|| DeltaError::SplitSurrogate(SPLIT_SURROGATE_MSG.to_string()))?;
                Op::Insert {
                    insert: Insert::Text(part.to_string()),
                    attributes: attributes.clone(),
                }
            }
            other => other.clone(),
        })
    }

    fn rest(&mut self) -> Result<Vec<Op>, DeltaError> {
        if !self.has_next() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        if self.offset > 0 {
            out.push(self.next(usize::MAX)?);
        }
        out.extend_from_slice(&self.ops[self.index..]);
        self.index = self.ops.len();
        Ok(out)
    }
}

fn compose_attrs(
    a: Option<&Attributes>,
    b: Option<&Attributes>,
    keep_null: bool,
) -> Option<Attributes> {
    let mut out = Attributes::new();
    if let Some(b) = b {
        for (k, v) in b {
            if keep_null || !v.is_null() {
                out.insert(k.clone(), v.clone());
            }
        }
    }
    if let Some(a) = a {
        for (k, v) in a {
            if !b.is_some_and(|b| b.contains_key(k)) {
                out.insert(k.clone(), v.clone());
            }
        }
    }
    (!out.is_empty()).then_some(out)
}

fn transform_attrs(
    a: Option<&Attributes>,
    b: Option<&Attributes>,
    priority: bool,
) -> Option<Attributes> {
    let Some(a) = a else {
        return b.cloned();
    };
    let b = b?;
    if !priority {
        return Some(b.clone());
    }
    Some(
        b.iter()
            .filter(|(k, _)| !a.contains_key(*k))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
    )
}

impl Delta {
    /// Remove a trailing attribute-less count retain.
    fn chop(mut self) -> Delta {
        if matches!(
            self.ops.last(),
            Some(Op::Retain {
                retain: Retain::Count(_),
                attributes: None
            })
        ) {
            self.ops.pop();
        }
        self
    }

    /// `self` then `other` as one Delta (quill-delta `compose`).
    ///
    /// Errors if an op would be cut inside a surrogate pair, or on an embed retain
    /// composed with an embed (needs a quill-delta embed handler).
    pub fn compose(&self, other: &Delta) -> Result<Delta, DeltaError> {
        let mut this = OpIter::new(&self.ops);
        let mut oth = OpIter::new(&other.ops);
        let mut delta = Delta::new();

        if let Some(Op::Retain {
            retain: Retain::Count(n),
            attributes: None,
        }) = other.ops.first()
        {
            let mut left = *n;
            while this.peek_kind() == Kind::Insert && this.peek_len() <= left {
                left -= this.peek_len();
                // quill-delta pushes these onto a plain array: no normalization.
                delta.ops.push(this.next(usize::MAX)?);
            }
            if *n - left > 0 {
                oth.next(*n - left)?;
            }
        }

        while this.has_next() || oth.has_next() {
            if oth.peek_kind() == Kind::Insert {
                delta.push(oth.next(usize::MAX)?);
            } else if this.peek_kind() == Kind::Delete {
                delta.push(this.next(usize::MAX)?);
            } else {
                let length = this.peek_len().min(oth.peek_len());
                let this_op = this.next(length)?;
                let other_op = oth.next(length)?;
                match other_op {
                    // A zero retain is falsy in quill-delta: the pair is dropped.
                    Op::Retain {
                        retain: Retain::Count(0),
                        ..
                    } => {}
                    Op::Retain {
                        retain: oret,
                        attributes: oattrs,
                    } => {
                        let (new_op, keep_null) = match (this_op, oret) {
                            (
                                Op::Retain {
                                    retain: Retain::Count(_),
                                    attributes,
                                },
                                oret,
                            ) => (
                                Op::Retain {
                                    // `length` is 0 when `this` is a zero retain, even
                                    // though `other` was consumed whole.
                                    retain: match oret {
                                        Retain::Count(_) => Retain::Count(length),
                                        embed => embed,
                                    },
                                    attributes: None,
                                },
                                (attributes, true),
                            ),
                            (
                                Op::Retain {
                                    retain: tret,
                                    attributes,
                                },
                                Retain::Count(_),
                            ) => (
                                Op::Retain {
                                    retain: tret,
                                    attributes: None,
                                },
                                (attributes, false),
                            ),
                            (Op::Insert { insert, attributes }, Retain::Count(_)) => (
                                Op::Insert {
                                    insert,
                                    attributes: None,
                                },
                                (attributes, false),
                            ),
                            _ => {
                                return Err(DeltaError::Unsupported(
                                    "composing an embed retain with an embed".into(),
                                ))
                            }
                        };
                        let (tattrs, keep_null) = keep_null;
                        let attrs = compose_attrs(tattrs.as_ref(), oattrs.as_ref(), keep_null);
                        let new_op = match new_op {
                            Op::Retain { retain, .. } => Op::Retain {
                                retain,
                                attributes: attrs,
                            },
                            Op::Insert { insert, .. } => Op::Insert {
                                insert,
                                attributes: attrs,
                            },
                            d => d,
                        };
                        delta.push(new_op.clone());
                        if !oth.has_next() && delta.ops.last() == Some(&new_op) {
                            let mut rest = this.rest()?.into_iter();
                            if let Some(first) = rest.next() {
                                delta.push(first);
                                delta.ops.extend(rest);
                            }
                            return Ok(delta.chop());
                        }
                    }
                    Op::Delete(_) => {
                        if matches!(this_op, Op::Retain { .. }) {
                            delta.push(other_op);
                        }
                    }
                    Op::Insert { .. } => unreachable!("other inserts are consumed above"),
                }
            }
        }
        Ok(delta.chop())
    }

    /// Transform `other` against `self`, which happened first (quill-delta `transform`).
    /// `priority` true means `self` wins ties (its inserts come first).
    pub fn transform(&self, other: &Delta, priority: bool) -> Delta {
        let mut this = OpIter::new(&self.ops);
        let mut oth = OpIter::new(&other.ops);
        let mut delta = Delta::new();
        while this.has_next() || oth.has_next() {
            if this.peek_kind() == Kind::Insert && (priority || oth.peek_kind() != Kind::Insert) {
                let n = this.peek_len();
                this.index += 1;
                this.offset = 0;
                // quill-delta's `retain()` builder drops a zero length.
                if let Some(op) = Op::retain(n, None) {
                    delta.push(op);
                }
            } else if oth.peek_kind() == Kind::Insert {
                // Inserts are consumed whole, so no text is ever sliced here.
                delta.push(oth.next(usize::MAX).expect("whole op never splits"));
            } else {
                let length = this.peek_len().min(oth.peek_len());
                let this_op = this.next(length).expect("retain/delete never splits");
                let other_op = oth.next(length).expect("retain/delete never splits");
                match (this_op, other_op) {
                    // Zero deletes are falsy in quill-delta and fall through to the
                    // retain branch below.
                    (Op::Delete(n), _) if n > 0 => {}
                    (_, d @ Op::Delete(1..)) => {
                        delta.push(d);
                    }
                    (this_op, other_op) => {
                        let retain = match &other_op {
                            Op::Retain {
                                retain: Retain::Embed(e),
                                ..
                            } => Retain::Embed(e.clone()),
                            _ => Retain::Count(length),
                        };
                        let attrs =
                            transform_attrs(this_op.attributes(), other_op.attributes(), priority)
                                .filter(|a| !a.is_empty());
                        // quill-delta's `retain()` builder drops a zero count.
                        if !matches!(retain, Retain::Count(0)) {
                            delta.push(Op::Retain {
                                retain,
                                attributes: attrs,
                            });
                        }
                    }
                }
            }
        }
        delta.chop()
    }

    /// Where an `index` in the document before `self` ends up after applying `self`.
    pub fn transform_position(&self, mut index: usize, priority: bool) -> usize {
        let mut offset = 0;
        for op in &self.ops {
            if offset > index {
                break;
            }
            let length = op.len();
            match op {
                Op::Delete(_) => {
                    index -= length.min(index - offset);
                    continue;
                }
                Op::Insert { .. } if offset < index || !priority => index += length,
                _ => {}
            }
            offset += length;
        }
        index
    }
}
