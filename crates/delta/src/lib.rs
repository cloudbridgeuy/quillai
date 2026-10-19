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
//! quill-delta never validates ops, so any JSON object is an op here too: odd ones
//! (negative or fractional counts, several or none of `insert`/`retain`/`delete`,
//! values of other types such as `{"insert":5}` or `{"retain":"3"}`, unknown keys,
//! non-object or array `attributes`) parse into [`Op::Raw`] and are handled exactly
//! like JS; `ops` entries that are not objects (`5`, `"s"`) are [`Op::NonObject`], and
//! keys next to `ops` are ignored, like `new Delta(obj)`. Numbers are JS numbers:
//! [`Delta::from_json`] reads `1.0` as `1` and `1e400` as Infinity, and
//! [`Delta::to_json`] prints floats like `JSON.stringify` (Infinity as `null`).
//! Every function named after a JS one works in JS numbers (`f64`): [`Op::len`],
//! [`Delta::len`] and [`Delta::transform_position`] return exactly what JS returns.
//! Known limits: object key order is not preserved (serde_json sorts keys), and where
//! quill-delta throws (composing an embed retain with an embed) `compose` returns
//! [`DeltaError::Unsupported`]; `transform` of two embed retains keeps the other side's.
//! Merging is not applied on deserialize; use [`Delta::push`].

mod compose;
mod delta;
mod error;
mod js;
mod number;
mod op;

pub use delta::Delta;
pub use error::DeltaError;
pub use op::{Attributes, Embed, Insert, Op, Retain};
