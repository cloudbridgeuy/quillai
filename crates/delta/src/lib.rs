//! Quill-compatible Delta core types.
//!
//! JSON wire format matches `quill-delta` 5.1.0:
//! `{"ops":[{"insert":"..","attributes":{..}},{"retain":n},{"delete":n}]}`.
//! Embeds are `{"insert":{..}}`; retain may also be an embed object.
//! All lengths are UTF-16 code units, like quill-delta.
//!
//! [`Delta::compose`], [`Delta::transform`] and [`Delta::transform_position`] follow
//! `quill-delta` 5.1.0 (no embed handlers: composing an embed retain with an embed
//! is [`DeltaError::Unsupported`]). `compose` fails with [`DeltaError::SplitSurrogate`]
//! if it would have to cut an emoji in half.
//!
//! # Parity rule with `quill-delta`
//!
//! This crate is a superset of the JS `quill-delta` the browser runs: it may add
//! functions, but it must never drop or change JS behavior. For every Delta that
//! `quill-delta` accepts and that holds valid Unicode, JSON parse/serialize,
//! `compose`, `transform` and `transform_position` give exactly the JS result.
//! That includes quirks: zero-length ops (`{"insert":""}`, `{"retain":0}`,
//! `{"delete":0}`) are kept on parse and by [`Delta::push`] (only the JS
//! `insert()/retain()/delete()` builders drop them, as do [`Op::insert_text`],
//! [`Op::retain`] and [`Op::delete`]), and `compose` reproduces JS's treatment of them.
//!
//! The single exception, as a fixed list: a Delta, or a compose result, that holds
//! half of a UTF-16 surrogate pair (half an emoji) cannot be a Rust `String`, so it
//! returns [`DeltaError::SplitSurrogate`]. `tests/fixtures.rs` pins the known cases
//! (parse-level half-emoji fixtures, compose fixtures with half-emoji input, and
//! compose results that would split a pair); any other mismatch fails the tests.
//!
//! Parsing is otherwise strict (parse, don't validate): unknown keys are rejected and
//! counts must be non-negative integers. Merging is not applied on deserialize; use
//! [`Delta::push`].

mod compose;
mod delta;
mod error;
mod op;

pub use delta::Delta;
pub use error::DeltaError;
pub use op::{Attributes, Embed, Insert, Op, Retain};
