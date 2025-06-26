//! # Delta
//!
//! A Rust implementation of the Quill Delta format for representing rich text documents and changes.
//! 
//! Deltas are a simple, yet expressive format that can be used to describe contents and changes.
//! The format is JSON based, and is human readable, yet easily parsible by machines. Deltas can
//! describe any rich text document, includes all text and formatting information, without the
//! ambiguity and complexity of HTML.

pub mod attributes;
pub mod delta;
pub mod diff;
pub mod op;
pub mod op_iterator;

pub use attributes::{AttributeMap, AttributeValue};
pub use delta::Delta;
pub use op::Op;
pub use op_iterator::OpIterator;

// Re-export for convenience
pub use serde_json::Value as JsonValue;