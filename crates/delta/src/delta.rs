//! Delta module - Core implementation of the Quill Delta format
//!
//! This module provides the main [`Delta`] struct which represents either a complete
//! document or a set of changes to apply to a document. Deltas are composed of a
//! sequence of operations that describe how to build or modify content.
//!
//! # Key Concepts
//!
//! - **Document Delta**: A delta that represents a complete document, containing only
//!   insert operations
//! - **Change Delta**: A delta that represents modifications to a document, containing
//!   any combination of insert, delete, and retain operations
//! - **Operational Transformation**: The ability to transform concurrent edits to
//!   maintain consistency in collaborative editing scenarios
//!
//! # Examples
//!
//! Creating a document:
//! ```rust
//! use quillai_delta::Delta;
//!
//! let doc = Delta::new()
//!     .insert("Hello world", None)
//!     .insert("\n", None);
//! ```
//!
//! Creating a change:
//! ```rust
//! use quillai_delta::Delta;
//!
//! let change = Delta::new()
//!     .retain(6, None)  // Keep "Hello "
//!     .delete(5)        // Delete "world"
//!     .insert("Rust", None);  // Insert "Rust"
//! ```

use crate::attributes::{AttributeMap, AttributeMapOps};
use crate::diff::{diff_text, DiffType};
use crate::op::{EmbedData, Op};
use crate::op_iterator::OpIterator;
use serde_json::Value as JsonValue;

/// Represents a Quill Delta - either a complete document or a change to a document
///
/// A Delta is fundamentally a sequence of operations that describe how to create
/// or modify a rich text document. The operations are stored in a normalized form
/// where consecutive operations of the same type with the same attributes are merged.
///
/// # Examples
///
/// Creating a simple document:
/// ```rust
/// use quillai_delta::Delta;
///
/// let doc = Delta::new()
///     .insert("Hello ", None)
///     .insert("world!", None);
/// // Results in a single insert operation: "Hello world!"
/// ```
///
/// Creating a document with formatting:
/// ```rust
/// use quillai_delta::{Delta, AttributeMap, AttributeValue};
///
/// let mut bold = AttributeMap::new();
/// bold.insert("bold".to_string(), AttributeValue::Boolean(true));
///
/// let doc = Delta::new()
///     .insert("Normal text ", None)
///     .insert("bold text", Some(bold))
///     .insert(" more normal", None);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delta {
    ops: Vec<Op>,
}

impl Delta {
    /// Creates a new empty Delta
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::Delta;
    ///
    /// let delta = Delta::new();
    /// assert!(delta.ops().is_empty());
    /// ```
    pub fn new() -> Self {
        Self { ops: Vec::new() }
    }

    /// Creates a Delta from a vector of operations
    ///
    /// This constructor does not perform any optimization or merging of operations.
    /// Use the builder methods (insert, delete, retain) for automatic optimization.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::{Delta, Op};
    ///
    /// let ops = vec![
    ///     Op::Insert { text: "Hello".to_string(), attributes: None },
    ///     Op::Delete(5),
    /// ];
    /// let delta = Delta::from_ops(ops);
    /// assert_eq!(delta.ops().len(), 2);
    /// ```
    pub fn from_ops(ops: Vec<Op>) -> Self {
        Self { ops }
    }

    /// Returns a slice of the operations in this Delta
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::Delta;
    ///
    /// let delta = Delta::new().insert("Hello", None);
    /// assert_eq!(delta.ops().len(), 1);
    /// ```
    pub fn ops(&self) -> &[Op] {
        &self.ops
    }

    /// Returns a mutable reference to the operations vector
    ///
    /// # Warning
    ///
    /// Direct modification of operations can break Delta invariants.
    /// Use the builder methods when possible.
    pub fn ops_mut(&mut self) -> &mut Vec<Op> {
        &mut self.ops
    }

    /// Inserts text with optional formatting attributes
    ///
    /// This method automatically merges consecutive insert operations with
    /// identical attributes for optimal storage.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to insert
    /// * `attributes` - Optional formatting attributes (bold, italic, color, etc.)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::{Delta, AttributeMap, AttributeValue};
    ///
    /// // Simple text insertion
    /// let delta = Delta::new().insert("Hello world", None);
    ///
    /// // Text with formatting
    /// let mut attrs = AttributeMap::new();
    /// attrs.insert("bold".to_string(), AttributeValue::Boolean(true));
    /// let delta = Delta::new().insert("Bold text", Some(attrs));
    /// ```
    pub fn insert<T: Into<String>>(self, text: T, attributes: Option<AttributeMap>) -> Self {
        let text = text.into();
        if text.is_empty() {
            return self;
        }

        let op = Op::Insert {
            text,
            attributes: if attributes.as_ref().is_some_and(|a| !a.is_empty()) {
                attributes
            } else {
                None
            },
        };
        self.push(op)
    }

    /// Inserts an embed object (image, video, etc.) with optional attributes
    ///
    /// Embeds are non-text content that occupy a single character position
    /// in the document.
    ///
    /// # Arguments
    ///
    /// * `embed_type` - The type of embed (e.g., "image", "video", "formula")
    /// * `data` - JSON data associated with the embed
    /// * `attributes` - Optional formatting attributes
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::Delta;
    /// use serde_json::json;
    ///
    /// let delta = Delta::new()
    ///     .insert("Check out this image: ", None)
    ///     .insert_embed("image".to_string(), json!({
    ///         "url": "https://example.com/image.png",
    ///         "alt": "Example image"
    ///     }), None);
    /// ```
    pub fn insert_embed(
        self,
        embed_type: String,
        data: JsonValue,
        attributes: Option<AttributeMap>,
    ) -> Self {
        let embed = EmbedData::new(embed_type, data);
        let op = Op::InsertEmbed {
            embed,
            attributes: if attributes.as_ref().is_some_and(|a| !a.is_empty()) {
                attributes
            } else {
                None
            },
        };
        self.push(op)
    }

    /// Deletes a specified number of characters
    ///
    /// Consecutive delete operations are automatically merged.
    ///
    /// # Arguments
    ///
    /// * `length` - The number of characters to delete
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::Delta;
    ///
    /// // Delete 5 characters
    /// let delta = Delta::new()
    ///     .retain(10, None)  // Skip first 10 characters
    ///     .delete(5);        // Delete next 5 characters
    /// ```
    pub fn delete(self, length: usize) -> Self {
        if length == 0 {
            return self;
        }
        self.push(Op::Delete(length))
    }

    /// Retains a number of characters, optionally modifying their attributes
    ///
    /// Retain operations indicate that existing content should be kept,
    /// with optional attribute changes applied.
    ///
    /// # Arguments
    ///
    /// * `length` - The number of characters to retain
    /// * `attributes` - Optional attribute changes to apply
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::{Delta, AttributeMap, AttributeValue};
    ///
    /// // Retain without changes
    /// let delta = Delta::new().retain(10, None);
    ///
    /// // Apply formatting to existing text
    /// let mut attrs = AttributeMap::new();
    /// attrs.insert("italic".to_string(), AttributeValue::Boolean(true));
    /// let delta = Delta::new()
    ///     .retain(5, None)           // Keep first 5 chars unchanged
    ///     .retain(3, Some(attrs));   // Make next 3 chars italic
    /// ```
    pub fn retain(self, length: usize, attributes: Option<AttributeMap>) -> Self {
        if length == 0 {
            return self;
        }

        let op = Op::Retain {
            length,
            attributes: if attributes.as_ref().is_some_and(|a| !a.is_empty()) {
                attributes
            } else {
                None
            },
        };
        self.push(op)
    }

    /// Retains an embed object, optionally modifying its attributes
    ///
    /// This is used to modify attributes of existing embeds in a document.
    ///
    /// # Arguments
    ///
    /// * `embed_type` - The type of the embed to retain
    /// * `data` - The embed data (must match the existing embed)
    /// * `attributes` - Optional attribute changes to apply
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::{Delta, AttributeMap, AttributeValue};
    /// use serde_json::json;
    ///
    /// // Add a border to an existing image
    /// let mut attrs = AttributeMap::new();
    /// attrs.insert("border".to_string(), AttributeValue::String("1px solid black".to_string()));
    /// 
    /// let delta = Delta::new()
    ///     .retain_embed("image".to_string(), json!({
    ///         "url": "https://example.com/image.png"
    ///     }), Some(attrs));
    /// ```
    pub fn retain_embed(
        self,
        embed_type: String,
        data: JsonValue,
        attributes: Option<AttributeMap>,
    ) -> Self {
        let embed = EmbedData::new(embed_type, data);
        let op = Op::RetainEmbed {
            embed,
            attributes: if attributes.as_ref().is_some_and(|a| !a.is_empty()) {
                attributes
            } else {
                None
            },
        };
        self.push(op)
    }

    /// Adds an operation to the Delta with automatic optimization
    ///
    /// This method handles:
    /// - Merging consecutive operations of the same type with identical attributes
    /// - Maintaining proper operation ordering (inserts before deletes)
    /// - Eliminating redundant operations
    ///
    /// # Arguments
    ///
    /// * `new_op` - The operation to add
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::{Delta, Op};
    ///
    /// // Operations are automatically optimized
    /// let delta = Delta::new()
    ///     .push(Op::Insert { text: "Hello".to_string(), attributes: None })
    ///     .push(Op::Insert { text: " World".to_string(), attributes: None });
    /// // Results in a single insert: "Hello World"
    /// ```
    pub fn push(mut self, new_op: Op) -> Self {
        if self.ops.is_empty() {
            self.ops.push(new_op);
            return self;
        }

        let last_index = self.ops.len() - 1;

        // Clone the last operation to avoid borrowing issues
        let last_op = self.ops[last_index].clone();

        // Try to merge delete operations
        if let (Op::Delete(_), Op::Delete(new_len)) = (&last_op, &new_op) {
            if let Op::Delete(ref mut last_len) = self.ops[last_index] {
                *last_len += new_len;
                return self;
            }
        }

        // Try to merge compatible operations
        if Self::can_merge_ops(&last_op, &new_op) {
            Self::merge_ops(&mut self.ops[last_index], new_op);
            return self;
        }

        // Insert before delete for consistent ordering
        if matches!(last_op, Op::Delete(_)) && new_op.is_insert() {
            if last_index > 0 {
                // Check if we can merge with the operation before the delete
                let prev_op = self.ops[last_index - 1].clone();
                if Self::can_merge_ops(&prev_op, &new_op) {
                    Self::merge_ops(&mut self.ops[last_index - 1], new_op);
                    return self;
                }
            }
            // Insert before the delete
            self.ops.insert(last_index, new_op);
            return self;
        }

        // Just append the operation
        self.ops.push(new_op);
        self
    }

    /// Determines if two operations can be merged together
    ///
    /// Operations can be merged if they are of the same type and have
    /// identical attributes.
    fn can_merge_ops(op1: &Op, op2: &Op) -> bool {
        match (op1, op2) {
            // Merge text inserts with same attributes
            (
                Op::Insert {
                    text: _,
                    attributes: attr1,
                },
                Op::Insert {
                    text: _,
                    attributes: attr2,
                },
            ) => attr1 == attr2,
            // Merge retains with same attributes
            (
                Op::Retain {
                    length: _,
                    attributes: attr1,
                },
                Op::Retain {
                    length: _,
                    attributes: attr2,
                },
            ) => attr1 == attr2,
            // Merge deletes
            (Op::Delete(_), Op::Delete(_)) => true,
            _ => false,
        }
    }

    /// Merges two compatible operations into the first one
    ///
    /// This assumes `can_merge_ops` has already returned true for these operations.
    fn merge_ops(op1: &mut Op, op2: Op) {
        match (op1, op2) {
            (
                Op::Insert {
                    text: text1,
                    attributes: _,
                },
                Op::Insert {
                    text: text2,
                    attributes: _,
                },
            ) => {
                text1.push_str(&text2);
            }
            (
                Op::Retain {
                    length: len1,
                    attributes: _,
                },
                Op::Retain {
                    length: len2,
                    attributes: _,
                },
            ) => {
                *len1 += len2;
            }
            (Op::Delete(len1), Op::Delete(len2)) => {
                *len1 += len2;
            }
            _ => {} // Should not happen if can_merge_ops is correct
        }
    }

    /// Removes a trailing retain operation without attributes
    ///
    /// This is commonly used after compose or transform operations to ensure
    /// the Delta doesn't end with a meaningless retain.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::Delta;
    ///
    /// let delta = Delta::new()
    ///     .insert("Hello", None)
    ///     .retain(10, None)
    ///     .chop();
    /// // The trailing retain(10) is removed
    /// ```
    pub fn chop(mut self) -> Self {
        match self.ops.last() {
            Some(Op::Retain {
                attributes: None, ..
            }) => {
                self.ops.pop();
            }
            _ => {
                // No trailing retain to remove
            }
        }
        self
    }

    /// Calculates the total length of content affected by this Delta
    ///
    /// This includes the length of all operations (inserts, deletes, and retains).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::Delta;
    ///
    /// let delta = Delta::new()
    ///     .insert("Hello", None)    // 5
    ///     .retain(3, None)          // 3
    ///     .delete(2);               // 2
    /// assert_eq!(delta.length(), 10);
    /// ```
    pub fn length(&self) -> usize {
        self.ops.iter().map(|op| op.length()).sum()
    }

    /// Calculates the net change in document length if this Delta is applied
    ///
    /// - Insert operations increase length
    /// - Delete operations decrease length
    /// - Retain operations don't change length
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::Delta;
    ///
    /// let delta = Delta::new()
    ///     .insert("Hello", None)    // +5
    ///     .delete(3)                // -3
    ///     .retain(10, None);        // +0
    /// assert_eq!(delta.change_length(), 2);
    /// ```
    pub fn change_length(&self) -> i64 {
        self.ops
            .iter()
            .map(|op| match op {
                Op::Insert { .. } | Op::InsertEmbed { .. } => op.length() as i64,
                Op::Delete(len) => -(*len as i64),
                _ => 0,
            })
            .sum()
    }

    /// Filters operations based on a predicate function
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::Delta;
    ///
    /// let delta = Delta::new()
    ///     .insert("Hello", None)
    ///     .delete(5)
    ///     .retain(10, None);
    ///
    /// let inserts = delta.filter(|op| op.is_insert());
    /// assert_eq!(inserts.len(), 1);
    /// ```
    pub fn filter<F>(&self, predicate: F) -> Vec<&Op>
    where
        F: Fn(&Op) -> bool,
    {
        self.ops.iter().filter(|op| predicate(op)).collect()
    }

    /// Applies a function to each operation in the Delta
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::Delta;
    ///
    /// let delta = Delta::new()
    ///     .insert("Hello", None)
    ///     .delete(5);
    ///
    /// delta.for_each(|op| {
    ///     println!("Operation: {:?}", op);
    /// });
    /// ```
    pub fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(&Op),
    {
        for op in &self.ops {
            f(op);
        }
    }

    /// Maps each operation to a value of type T
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::Delta;
    ///
    /// let delta = Delta::new()
    ///     .insert("Hello", None)
    ///     .delete(5)
    ///     .retain(10, None);
    ///
    /// let lengths: Vec<usize> = delta.map(|op| op.length());
    /// assert_eq!(lengths, vec![5, 5, 10]);
    /// ```
    pub fn map<T, F>(&self, f: F) -> Vec<T>
    where
        F: Fn(&Op) -> T,
    {
        self.ops.iter().map(f).collect()
    }

    /// Partitions operations into two groups based on a predicate
    ///
    /// Returns a tuple where the first vector contains operations that
    /// satisfy the predicate, and the second contains those that don't.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::Delta;
    ///
    /// let delta = Delta::new()
    ///     .insert("Hello", None)
    ///     .delete(5)
    ///     .retain(10, None);
    ///
    /// let (modifying, preserving) = delta.partition(|op| op.is_insert() || op.is_delete());
    /// assert_eq!(modifying.len(), 2);
    /// assert_eq!(preserving.len(), 1);
    /// ```
    pub fn partition<F>(&self, predicate: F) -> (Vec<Op>, Vec<Op>)
    where
        F: Fn(&Op) -> bool,
    {
        let mut passed = Vec::new();
        let mut failed = Vec::new();

        for op in &self.ops {
            if predicate(op) {
                passed.push(op.clone());
            } else {
                failed.push(op.clone());
            }
        }

        (passed, failed)
    }

    /// Reduces operations to a single value using an accumulator function
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::Delta;
    ///
    /// let delta = Delta::new()
    ///     .insert("Hello", None)
    ///     .insert(" World", None);
    ///
    /// let total_text = delta.reduce(String::new(), |mut acc, op| {
    ///     if let quillai_delta::Op::Insert { text, .. } = op {
    ///         acc.push_str(text);
    ///     }
    ///     acc
    /// });
    /// assert_eq!(total_text, "Hello World");
    /// ```
    pub fn reduce<T, F>(&self, initial: T, f: F) -> T
    where
        F: Fn(T, &Op) -> T,
    {
        self.ops.iter().fold(initial, f)
    }

    /// Extracts a slice of operations from the Delta
    ///
    /// This creates a new Delta containing only the operations that affect
    /// the specified character range.
    ///
    /// # Arguments
    ///
    /// * `start` - The starting character index (inclusive)
    /// * `end` - The ending character index (exclusive), or None for the end
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::Delta;
    ///
    /// let delta = Delta::new().insert("Hello World", None);
    /// 
    /// let slice = delta.slice(0, Some(5));
    /// // Contains: insert("Hello")
    ///
    /// let slice = delta.slice(6, None);
    /// // Contains: insert("World")
    /// ```
    pub fn slice(&self, start: usize, end: Option<usize>) -> Delta {
        let end = end.unwrap_or(usize::MAX);
        let mut ops = Vec::new();
        let mut iter = OpIterator::new(&self.ops);
        let mut index = 0;

        while index < end && iter.has_next() {
            let next_op = if index < start {
                iter.next(Some(start - index))
            } else {
                let op = iter.next(Some(end - index));
                ops.push(op.clone());
                op
            };
            index += next_op.length();
        }

        Delta::from_ops(ops)
    }

    /// Concatenates this Delta with another Delta
    ///
    /// The operations from the other Delta are appended, with automatic
    /// merging of compatible operations at the boundary.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::Delta;
    ///
    /// let delta1 = Delta::new().insert("Hello", None);
    /// let delta2 = Delta::new().insert(" World", None);
    ///
    /// let combined = delta1.concat(&delta2);
    /// // Results in: insert("Hello World")
    /// ```
    pub fn concat(&self, other: &Delta) -> Delta {
        let mut result = self.clone();
        if !other.ops.is_empty() {
            result = result.push(other.ops[0].clone());
            for op in &other.ops[1..] {
                result.ops.push(op.clone());
            }
        }
        result
    }

    /// Composes this Delta with another Delta
    ///
    /// Composition creates a new Delta that represents the result of applying
    /// this Delta followed by the other Delta. This is the fundamental operation
    /// for combining sequential changes.
    ///
    /// # Arguments
    ///
    /// * `other` - The Delta to compose with this one
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::Delta;
    ///
    /// // Original document
    /// let doc = Delta::new().insert("Hello World", None);
    ///
    /// // First change: replace "World" with "Rust"
    /// let change1 = Delta::new()
    ///     .retain(6, None)
    ///     .delete(5)
    ///     .insert("Rust", None);
    ///
    /// // Second change: make "Rust" bold
    /// let mut bold = std::collections::BTreeMap::new();
    /// bold.insert("bold".to_string(), quillai_delta::AttributeValue::Boolean(true));
    /// let change2 = Delta::new()
    ///     .retain(6, None)
    ///     .retain(4, Some(bold));
    ///
    /// // Compose changes
    /// let combined = change1.compose(&change2);
    /// // Results in: retain(6), delete(5), insert("Rust", bold)
    /// ```
    pub fn compose(&self, other: &Delta) -> Delta {
        let mut this_iter = OpIterator::new(&self.ops);
        let mut other_iter = OpIterator::new(&other.ops);
        let mut result = Delta::new();

        // Handle leading retain in other
        match other_iter.peek() {
            Some(Op::Retain {
                length,
                attributes: None,
            }) => {
                let mut first_left = *length;
                while this_iter.peek_type() == "insert" && this_iter.peek_length() <= first_left {
                    first_left -= this_iter.peek_length();
                    result = result.push(this_iter.next(None));
                }
                if length - first_left > 0 {
                    other_iter.next(Some(length - first_left));
                }
            }
            _ => {
                // No leading retain, just start with this
            }
        }

        while this_iter.has_next() || other_iter.has_next() {
            if other_iter.peek_type() == "insert" {
                result = result.push(other_iter.next(None));
            } else if this_iter.peek_type() == "delete" {
                result = result.push(this_iter.next(None));
            } else {
                let length = std::cmp::min(this_iter.peek_length(), other_iter.peek_length());
                let this_op = this_iter.next(Some(length));
                let other_op = other_iter.next(Some(length));

                match (&this_op, &other_op) {
                    (
                        _,
                        Op::Retain {
                            attributes: other_attrs,
                            ..
                        },
                    ) => {
                        let new_op = match &this_op {
                            Op::Retain {
                                attributes: this_attrs,
                                ..
                            } => Op::Retain {
                                length,
                                attributes: AttributeMapOps::compose(
                                    this_attrs.as_ref(),
                                    other_attrs.as_ref(),
                                    true,
                                ),
                            },
                            Op::Insert {
                                text,
                                attributes: this_attrs,
                            } => Op::Insert {
                                text: text.clone(),
                                attributes: AttributeMapOps::compose(
                                    this_attrs.as_ref(),
                                    other_attrs.as_ref(),
                                    false,
                                ),
                            },
                            _ => this_op, // Should not happen in valid compose
                        };
                        let new_op_clone = new_op.clone();
                        result = result.push(new_op);

                        // Optimization: if rest of other is just retain, concatenate rest of this
                        if !other_iter.has_next()
                            && matches!(result.ops.last(), Some(last) if *last == new_op_clone)
                        {
                            let rest = Delta::from_ops(this_iter.rest());
                            return result.concat(&rest).chop();
                        }
                    }
                    (Op::Retain { .. } | Op::Insert { .. }, Op::Delete(_)) => {
                        result = result.push(other_op);
                    }
                    _ => {} // Insert + delete cancels out
                }
            }
        }

        result.chop()
    }

    /// Transforms this Delta against another Delta for operational transformation
    ///
    /// Transform is used to adjust a Delta so it can be applied after another
    /// concurrent Delta. This is essential for real-time collaborative editing.
    ///
    /// # Arguments
    ///
    /// * `other` - The Delta to transform against
    /// * `priority` - If true, this Delta takes precedence in conflicts
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::Delta;
    ///
    /// // Two users editing concurrently
    /// let user1 = Delta::new()
    ///     .retain(5, None)
    ///     .insert(" there", None);
    ///
    /// let user2 = Delta::new()
    ///     .retain(10, None)
    ///     .insert("!", None);
    ///
    /// // Transform user2's changes to apply after user1's
    /// let user2_transformed = user2.transform(&user1, false);
    /// // Results in: retain(11), insert("!") - adjusted for the inserted " there"
    /// ```
    pub fn transform(&self, other: &Delta, priority: bool) -> Delta {
        let mut this_iter = OpIterator::new(&self.ops);
        let mut other_iter = OpIterator::new(&other.ops);
        let mut result = Delta::new();

        while this_iter.has_next() || other_iter.has_next() {
            if this_iter.peek_type() == "insert" && (priority || other_iter.peek_type() != "insert")
            {
                result = result.retain(this_iter.next(None).length(), None);
            } else if other_iter.peek_type() == "insert" {
                result = result.push(other_iter.next(None));
            } else {
                let length = std::cmp::min(this_iter.peek_length(), other_iter.peek_length());
                let this_op = this_iter.next(Some(length));
                let other_op = other_iter.next(Some(length));

                match (&this_op, &other_op) {
                    (Op::Delete(_), _) => {
                        // Our delete makes their operation irrelevant
                        continue;
                    }
                    (_, Op::Delete(_)) => {
                        result = result.push(other_op);
                    }
                    (
                        Op::Retain {
                            attributes: this_attrs,
                            ..
                        },
                        Op::Retain {
                            attributes: other_attrs,
                            ..
                        },
                    ) => {
                        result = result.retain(
                            length,
                            AttributeMapOps::transform(
                                this_attrs.as_ref(),
                                other_attrs.as_ref(),
                                priority,
                            ),
                        );
                    }
                    _ => {
                        // Both are retains or inserts, transform attributes
                        let transformed_attrs = match (&this_op, &other_op) {
                            (
                                Op::Insert {
                                    attributes: this_attrs,
                                    ..
                                },
                                Op::Retain {
                                    attributes: other_attrs,
                                    ..
                                },
                            )
                            | (
                                Op::Retain {
                                    attributes: this_attrs,
                                    ..
                                },
                                Op::Insert {
                                    attributes: other_attrs,
                                    ..
                                },
                            ) => AttributeMapOps::transform(
                                this_attrs.as_ref(),
                                other_attrs.as_ref(),
                                priority,
                            ),
                            _ => None,
                        };

                        match &other_op {
                            Op::Insert { text, .. } => {
                                result = result.insert(text.clone(), transformed_attrs);
                            }
                            Op::Retain { .. } => {
                                result = result.retain(length, transformed_attrs);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        result.chop()
    }

    /// Transforms a position index to account for this Delta's operations
    ///
    /// This is useful for updating cursor positions or selections after
    /// applying a Delta to a document.
    ///
    /// # Arguments
    ///
    /// * `index` - The original position index
    /// * `priority` - If true, positions at operation boundaries favor this Delta
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::Delta;
    ///
    /// // Delta that inserts "Hello " at the beginning
    /// let delta = Delta::new().insert("Hello ", None);
    ///
    /// // Transform cursor position
    /// let new_position = delta.transform_position(5, false);
    /// assert_eq!(new_position, 11); // 5 + 6 (length of "Hello ")
    /// ```
    pub fn transform_position(&self, index: usize, priority: bool) -> usize {
        let mut iter = OpIterator::new(&self.ops);
        let mut offset = 0;
        let mut transformed_index = index;

        while iter.has_next() && offset <= index {
            let length = iter.peek_length();
            let op_type = iter.peek_type();
            iter.next(None);

            match op_type {
                "delete" => {
                    transformed_index =
                        transformed_index.saturating_sub(std::cmp::min(length, index - offset));
                }
                "insert" => {
                    if offset < index || !priority {
                        transformed_index += length;
                    }
                }
                _ => {} // retain doesn't affect position
            }

            offset += length;
        }

        transformed_index
    }

    /// Creates a diff Delta that transforms this document into another document
    ///
    /// Both Deltas must be documents (contain only insert operations).
    /// The resulting Delta contains the operations needed to transform
    /// this document into the other document.
    ///
    /// # Arguments
    ///
    /// * `other` - The target document to diff against
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::Delta;
    ///
    /// let doc1 = Delta::new().insert("Hello World", None);
    /// let doc2 = Delta::new().insert("Hello Rust", None);
    ///
    /// let diff = doc1.diff(&doc2);
    /// // Results in: retain(6), delete(5), insert("Rust")
    ///
    /// // Applying the diff to doc1 produces doc2
    /// let result = doc1.compose(&diff);
    /// ```
    pub fn diff(&self, other: &Delta) -> Delta {
        if self.ops == other.ops {
            return Delta::new();
        }

        // Convert both deltas to strings for diffing
        let self_text = self.to_text();
        let other_text = other.to_text();

        let diff_ops = diff_text(&self_text, &other_text);
        let mut result = Delta::new();
        let mut this_iter = OpIterator::new(&self.ops);
        let mut other_iter = OpIterator::new(&other.ops);

        for diff_op in diff_ops {
            let mut length = diff_op.length();

            while length > 0 {
                match diff_op.operation {
                    DiffType::Insert => {
                        let op_length = std::cmp::min(other_iter.peek_length(), length);
                        result = result.push(other_iter.next(Some(op_length)));
                        length -= op_length;
                    }
                    DiffType::Delete => {
                        let op_length = std::cmp::min(length, this_iter.peek_length());
                        this_iter.next(Some(op_length));
                        result = result.delete(op_length);
                        length -= op_length;
                    }
                    DiffType::Equal => {
                        let op_length = std::cmp::min(
                            std::cmp::min(this_iter.peek_length(), other_iter.peek_length()),
                            length,
                        );
                        let this_op = this_iter.next(Some(op_length));
                        let other_op = other_iter.next(Some(op_length));

                        // Check if the content is actually equal
                        let content_equal = match (&this_op, &other_op) {
                            (Op::Insert { text: t1, .. }, Op::Insert { text: t2, .. }) => t1 == t2,
                            (
                                Op::InsertEmbed { embed: e1, .. },
                                Op::InsertEmbed { embed: e2, .. },
                            ) => e1 == e2,
                            _ => false,
                        };

                        if content_equal {
                            let attr_diff =
                                AttributeMapOps::diff(this_op.attributes(), other_op.attributes());
                            result = result.retain(op_length, attr_diff);
                        } else {
                            result = result.push(other_op).delete(op_length);
                        }
                        length -= op_length;
                    }
                }
            }
        }

        result.chop()
    }

    /// Creates an inverted Delta that undoes this Delta's changes
    ///
    /// The inverted Delta, when applied to the result of applying this Delta
    /// to the base document, will restore the base document.
    ///
    /// # Arguments
    ///
    /// * `base` - The document state before this Delta was applied
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::Delta;
    ///
    /// let base = Delta::new().insert("Hello World", None);
    /// 
    /// let change = Delta::new()
    ///     .retain(6, None)
    ///     .delete(5)
    ///     .insert("Rust", None);
    ///
    /// let inverted = change.invert(&base);
    /// // Results in: retain(6), delete(4), insert("World")
    ///
    /// // Applying change then inverted restores the original
    /// let modified = base.compose(&change);
    /// let restored = modified.compose(&inverted);
    /// // restored equals base
    /// ```
    pub fn invert(&self, base: &Delta) -> Delta {
        let mut inverted = Delta::new();
        let mut base_index = 0;

        for op in &self.ops {
            match op {
                Op::Insert { .. } | Op::InsertEmbed { .. } => {
                    inverted = inverted.delete(op.length());
                }
                Op::Delete(length) => {
                    let slice = base.slice(base_index, Some(base_index + length));
                    for base_op in slice.ops() {
                        inverted = inverted.push(base_op.clone());
                    }
                    base_index += length;
                }
                Op::Retain { length, attributes } => {
                    if attributes.is_none() {
                        inverted = inverted.retain(*length, None);
                    } else {
                        let slice = base.slice(base_index, Some(base_index + length));
                        for base_op in slice.ops() {
                            let inverted_attrs =
                                AttributeMapOps::invert(attributes.as_ref(), base_op.attributes());
                            inverted = inverted.retain(base_op.length(), inverted_attrs);
                        }
                    }
                    base_index += length;
                }
                Op::RetainEmbed { .. } => {
                    // For embed retains, we need to handle the base operation
                    let slice = base.slice(base_index, Some(base_index + 1));
                    if let Some(base_op) = slice.ops().first() {
                        let inverted_attrs =
                            AttributeMapOps::invert(op.attributes(), base_op.attributes());
                        match base_op {
                            Op::InsertEmbed { embed, .. } => {
                                inverted = inverted.retain_embed(
                                    embed.embed_type.clone(),
                                    embed.data.clone(),
                                    inverted_attrs,
                                );
                            }
                            _ => {
                                inverted = inverted.retain(1, inverted_attrs);
                            }
                        }
                    }
                    base_index += 1;
                }
            }
        }

        inverted.chop()
    }

    /// Converts a document Delta to plain text
    ///
    /// This method extracts only the text content from insert operations,
    /// ignoring all formatting. Embeds are represented as null characters.
    ///
    /// Note: This only works correctly for document Deltas (containing only inserts).
    fn to_text(&self) -> String {
        let mut result = String::new();
        for op in &self.ops {
            match op {
                Op::Insert { text, .. } => result.push_str(text),
                Op::InsertEmbed { .. } => result.push('\0'), // Use null character for embeds
                _ => {} // Only insert operations contribute to document text
            }
        }
        result
    }
}

impl Default for Delta {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attributes::AttributeValue;
    use std::collections::BTreeMap;

    #[test]
    fn test_delta_construction() {
        let delta = Delta::new();
        assert!(delta.ops().is_empty());

        let delta = Delta::new()
            .insert("Hello", None)
            .insert(" ", None)
            .insert("World", None);

        assert_eq!(delta.ops().len(), 1); // Should merge into single insert
        if let Op::Insert { text, .. } = &delta.ops()[0] {
            assert_eq!(text, "Hello World");
        } else {
            panic!("Expected insert operation");
        }
    }

    #[test]
    fn test_delta_attributes() {
        let mut attrs = BTreeMap::new();
        attrs.insert("bold".to_string(), AttributeValue::Boolean(true));

        let delta = Delta::new()
            .insert("Hello", Some(attrs.clone()))
            .insert("World", Some(attrs));

        assert_eq!(delta.ops().len(), 1); // Should merge
        if let Op::Insert { text, attributes } = &delta.ops()[0] {
            assert_eq!(text, "HelloWorld");
            assert!(attributes.is_some());
        } else {
            panic!("Expected insert operation");
        }
    }

    #[test]
    fn test_delta_different_attributes() {
        let mut bold = BTreeMap::new();
        bold.insert("bold".to_string(), AttributeValue::Boolean(true));

        let mut italic = BTreeMap::new();
        italic.insert("italic".to_string(), AttributeValue::Boolean(true));

        let delta = Delta::new()
            .insert("Hello", Some(bold))
            .insert("World", Some(italic));

        assert_eq!(delta.ops().len(), 2); // Should not merge different attributes
    }

    #[test]
    fn test_delta_delete_merge() {
        let delta = Delta::new().delete(5).delete(3);

        assert_eq!(delta.ops().len(), 1);
        if let Op::Delete(len) = delta.ops()[0] {
            assert_eq!(len, 8);
        } else {
            panic!("Expected delete operation");
        }
    }

    #[test]
    fn test_delta_insert_before_delete() {
        let delta = Delta::new().delete(5).insert("Hello", None);

        assert_eq!(delta.ops().len(), 2);
        assert!(delta.ops()[0].is_insert());
        assert!(delta.ops()[1].is_delete());
    }

    #[test]
    fn test_delta_length() {
        let delta = Delta::new().insert("Hello", None).retain(5, None).delete(3);

        assert_eq!(delta.length(), 13); // 5 + 5 + 3
    }

    #[test]
    fn test_delta_change_length() {
        let delta = Delta::new()
            .insert("Hello", None) // +5
            .retain(5, None) // +0
            .delete(3); // -3

        assert_eq!(delta.change_length(), 2); // 5 - 3
    }

    #[test]
    fn test_delta_slice() {
        let delta = Delta::new().insert("Hello World", None);

        let sliced = delta.slice(0, Some(5));
        assert_eq!(sliced.ops().len(), 1);
        if let Op::Insert { text, .. } = &sliced.ops()[0] {
            assert_eq!(text, "Hello");
        } else {
            panic!("Expected insert operation");
        }

        let sliced = delta.slice(6, None);
        assert_eq!(sliced.ops().len(), 1);
        if let Op::Insert { text, .. } = &sliced.ops()[0] {
            assert_eq!(text, "World");
        } else {
            panic!("Expected insert operation");
        }
    }

    #[test]
    fn test_delta_chop() {
        let delta = Delta::new().insert("Hello", None).retain(5, None).chop();

        assert_eq!(delta.ops().len(), 1); // Retain should be removed
        assert!(delta.ops()[0].is_insert());
    }
}
