//! # QuillAI Delta
//!
//! A Rust implementation of the [Quill Delta](https://quilljs.com/docs/delta/) format for 
//! representing rich text documents and their changes.
//!
//! ## Overview
//!
//! Deltas are a simple, yet expressive format that can be used to describe contents and changes.
//! The format is JSON-based, human readable, and easily parsable by machines. Deltas can
//! describe any rich text document, including all text and formatting information, without the
//! ambiguity and complexity of HTML.
//!
//! This library provides a complete implementation of the Delta format with support for:
//! - Creating and manipulating rich text documents
//! - Operational transformation for real-time collaboration
//! - Diffing documents to generate change sets
//! - Composing and transforming deltas
//! - Applying deltas to documents
//!
//! ## Basic Usage
//!
//! ```rust
//! use quillai_delta::{Delta, AttributeMap};
//!
//! // Create a new document
//! let mut doc = Delta::new()
//!     .insert("Hello ", None)
//!     .insert("world", Some([("bold".to_string(), true.into())].into_iter().collect()))
//!     .insert("!\n", None);
//!
//! // Create a change that modifies the document
//! let change = Delta::new()
//!     .retain(6, None)  // Keep "Hello "
//!     .delete(5)        // Delete "world"
//!     .insert("Rust", Some([("italic".to_string(), true.into())].into_iter().collect()));
//!
//! // Apply the change to the document
//! let new_doc = doc.compose(&change);
//! ```
//!
//! ## Core Concepts
//!
//! ### Operations
//!
//! Deltas are composed of operations that describe how to build or modify a document:
//! - **Insert**: Add text or embeds with optional formatting attributes
//! - **Delete**: Remove a specified number of characters
//! - **Retain**: Keep existing content, optionally modifying its attributes
//!
//! ### Attributes
//!
//! Attributes represent formatting and metadata associated with text:
//! - Text can have multiple attributes (bold, italic, color, etc.)
//! - Attributes are stored in a `BTreeMap` for consistent ordering
//! - Null attributes indicate removal of formatting
//!
//! ### Operational Transformation
//!
//! This library supports operational transformation (OT) for collaborative editing:
//! - **Compose**: Combine two deltas sequentially
//! - **Transform**: Adjust deltas for concurrent editing
//! - **Invert**: Generate the inverse of a delta
//!
//! ## Examples
//!
//! ### Creating a formatted document
//!
//! ```rust
//! use quillai_delta::{Delta, AttributeMap, AttributeValue};
//!
//! let doc = Delta::new()
//!     .insert("Title\n", Some([
//!         ("header".to_string(), AttributeValue::Number(1)),
//!         ("bold".to_string(), AttributeValue::Boolean(true))
//!     ].into_iter().collect()))
//!     .insert("This is a ", None)
//!     .insert("formatted", Some([
//!         ("italic".to_string(), AttributeValue::Boolean(true)),
//!         ("color".to_string(), AttributeValue::String("#ff0000".to_string()))
//!     ].into_iter().collect()))
//!     .insert(" document.\n", None);
//! ```
//!
//! ### Diffing documents
//!
//! ```rust
//! use quillai_delta::Delta;
//!
//! let old_doc = Delta::new().insert("Hello world", None);
//! let new_doc = Delta::new().insert("Hello Rust", None);
//!
//! let diff = old_doc.diff(&new_doc);
//! // Results in: retain(6), delete(5), insert("Rust")
//! ```
//!
//! ## Design Goals
//!
//! This implementation aims to:
//! - Maintain compatibility with the JavaScript Quill Delta format
//! - Provide a safe, idiomatic Rust API
//! - Support all Delta operations and transformations
//! - Enable efficient document manipulation and storage
//!
//! ## See Also
//!
//! - [Quill Delta Documentation](https://quilljs.com/docs/delta/)
//! - [Designing the Delta Format](https://quilljs.com/guides/designing-the-delta-format/)
//! - [Original JavaScript Implementation](https://github.com/quilljs/delta)

#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

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