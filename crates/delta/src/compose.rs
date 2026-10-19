//! `compose`, `transform` and `transform_position`, ported from quill-delta 5.1.0
//! (the port itself lives in `js.rs`).

use crate::error::DeltaError;
use crate::js::{self, JOp};
use crate::op::Op;
use crate::Delta;

fn to_js(d: &Delta) -> Vec<JOp> {
    d.ops.iter().map(Op::to_js).collect()
}

fn from_js(ops: Vec<JOp>) -> Delta {
    Delta {
        ops: ops.into_iter().map(Op::from_js).collect(),
    }
}

impl Delta {
    /// `self` then `other` as one Delta (quill-delta `compose`).
    ///
    /// Errors if an op would be cut inside a surrogate pair, or on an embed retain
    /// composed with an embed (quill-delta throws there too: no embed handler).
    pub fn compose(&self, other: &Delta) -> Result<Delta, DeltaError> {
        js::compose(&to_js(self), &to_js(other)).map(from_js)
    }

    /// Transform `other` against `self`, which happened first (quill-delta `transform`).
    /// `priority` true means `self` wins ties (its inserts come first).
    pub fn transform(&self, other: &Delta, priority: bool) -> Delta {
        from_js(js::transform(&to_js(self), &to_js(other), priority))
    }

    /// Where an `index` in the document before `self` ends up after applying `self`
    /// (quill-delta `transformPosition`, in JS numbers: negative or fractional counts
    /// give exactly the JS result).
    pub fn transform_position(&self, index: f64, priority: bool) -> f64 {
        js::transform_position(&to_js(self), index, priority)
    }
}
