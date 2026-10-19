//! Quill-compatible Delta core types.
//!
//! JSON wire format matches `quill-delta` 5.1.0:
//! `{"ops":[{"insert":"..","attributes":{..}},{"retain":n},{"delete":n}]}`.
//! Embeds are `{"insert":{..}}`; retain may also be an embed object.
//! All lengths are UTF-16 code units, like quill-delta.
//!
//! Parsing is strict (parse, don't validate): unknown keys are rejected and text
//! can never contain a split surrogate pair. Zero-length ops (`{"insert":""}`,
//! `{"retain":0}`, `{"delete":0}`) are accepted and dropped as no-ops. Other
//! normalization (merging) is not applied on deserialize; use [`Delta::push`].

mod delta;
mod error;
mod op;

pub use delta::Delta;
pub use error::DeltaError;
pub use op::{Attributes, Embed, Insert, Op, Retain};
