//! Operation iterator for efficient Delta traversal
//!
//! This module provides the [`OpIterator`] struct, which enables efficient
//! traversal and slicing of Delta operations. The iterator is particularly
//! useful for implementing compose, transform, and diff operations where
//! you need to process operations in parallel while potentially splitting
//! them into smaller chunks.
//!
//! # Key Features
//!
//! - **Partial consumption**: Operations can be consumed partially, allowing
//!   precise control over how much of each operation to process
//! - **Peeking**: Look ahead at the next operation without consuming it
//! - **Slicing**: Extract specific portions of text operations
//! - **Infinite retain**: Returns infinite retain when exhausted, simplifying
//!   compose/transform algorithms

use crate::op::Op;

/// An iterator for traversing and slicing Delta operations
///
/// `OpIterator` maintains internal state to track position within operations,
/// allowing partial consumption of operations. This is essential for algorithms
/// like compose and transform that need to process operations in lockstep.
///
/// # Design
///
/// The iterator tracks:
/// - `ops`: The vector of operations to iterate over
/// - `index`: The current operation index
/// - `offset`: The offset within the current operation (for partial consumption)
///
/// When an operation is partially consumed, the iterator remembers the offset
/// and returns the remaining portion on the next call.
///
/// # Examples
///
/// ```rust
/// use quillai_delta::{Op, OpIterator};
///
/// let ops = vec![
///     Op::Insert { text: "Hello World".to_string(), attributes: None },
///     Op::Retain { length: 5, attributes: None },
/// ];
///
/// let mut iter = OpIterator::new(&ops);
///
/// // Consume first 5 characters of the insert
/// let partial = iter.next(Some(5));
/// // Returns: Insert { text: "Hello", attributes: None }
///
/// // Consume the rest
/// let rest = iter.next(None);
/// // Returns: Insert { text: " World", attributes: None }
/// ```
#[derive(Debug, Clone)]
pub struct OpIterator {
    ops: Vec<Op>,
    index: usize,
    offset: usize,
}

impl OpIterator {
    /// Creates a new iterator from a slice of operations
    ///
    /// # Arguments
    ///
    /// * `ops` - The operations to iterate over
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::{Op, OpIterator};
    ///
    /// let ops = vec![
    ///     Op::Insert { text: "Hello".to_string(), attributes: None },
    ///     Op::Delete(5),
    /// ];
    /// let iter = OpIterator::new(&ops);
    /// ```
    pub fn new(ops: &[Op]) -> Self {
        Self {
            ops: ops.to_vec(),
            index: 0,
            offset: 0,
        }
    }

    /// Checks if there are more operations to process
    ///
    /// Returns `false` when all operations have been consumed.
    /// Note that the iterator returns infinite retain operations
    /// when exhausted, so `next()` will always return something.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::{Op, OpIterator};
    ///
    /// let ops = vec![Op::Delete(5)];
    /// let mut iter = OpIterator::new(&ops);
    ///
    /// assert!(iter.has_next());
    /// iter.next(None);
    /// assert!(!iter.has_next());
    /// ```
    pub fn has_next(&self) -> bool {
        self.peek_length() < usize::MAX
    }

    /// Consumes and returns the next operation or portion thereof
    ///
    /// If a length is specified and the current operation is longer,
    /// only that length is consumed and the remainder is saved for
    /// the next call. When the iterator is exhausted, it returns
    /// infinite retain operations.
    ///
    /// # Arguments
    ///
    /// * `length` - Optional maximum length to consume. If None, consumes the entire operation.
    ///
    /// # Returns
    ///
    /// The next operation or operation slice. Returns an infinite retain
    /// when no more operations are available.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::{Op, OpIterator};
    ///
    /// let ops = vec![Op::Insert { text: "Hello".to_string(), attributes: None }];
    /// let mut iter = OpIterator::new(&ops);
    ///
    /// // Consume only 2 characters
    /// let partial = iter.next(Some(2));
    /// if let Op::Insert { text, .. } = partial {
    ///     assert_eq!(text, "He");
    /// }
    ///
    /// // Consume the rest
    /// let rest = iter.next(None);
    /// if let Op::Insert { text, .. } = rest {
    ///     assert_eq!(text, "llo");
    /// }
    /// ```
    pub fn next(&mut self, length: Option<usize>) -> Op {
        let length = length.unwrap_or(usize::MAX);

        if let Some(next_op) = self.ops.get(self.index) {
            let offset = self.offset;
            let op_length = next_op.length();

            if length >= op_length - offset {
                // Consume the entire operation
                let consumed_length = op_length - offset;
                self.index += 1;
                self.offset = 0;
                self.slice_op(next_op, offset, consumed_length)
            } else {
                // Partially consume the operation
                self.offset += length;
                self.slice_op(next_op, offset, length)
            }
        } else {
            // Return infinite retain when no more operations
            Op::Retain {
                length: usize::MAX,
                attributes: None,
            }
        }
    }

    /// Peeks at the current operation without consuming it
    ///
    /// Returns `None` if the iterator is exhausted.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::{Op, OpIterator};
    ///
    /// let ops = vec![Op::Delete(5)];
    /// let iter = OpIterator::new(&ops);
    ///
    /// // Peek doesn't consume
    /// assert!(iter.peek().is_some());
    /// assert!(iter.peek().is_some());
    /// ```
    pub fn peek(&self) -> Option<&Op> {
        self.ops.get(self.index)
    }

    /// Returns the remaining length of the current operation
    ///
    /// If the iterator is exhausted, returns `usize::MAX` to indicate
    /// infinite length (used for compose/transform algorithms).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::{Op, OpIterator};
    ///
    /// let ops = vec![Op::Insert { text: "Hello".to_string(), attributes: None }];
    /// let mut iter = OpIterator::new(&ops);
    ///
    /// assert_eq!(iter.peek_length(), 5);
    /// iter.next(Some(2));
    /// assert_eq!(iter.peek_length(), 3); // "llo" remaining
    /// ```
    pub fn peek_length(&self) -> usize {
        if let Some(op) = self.ops.get(self.index) {
            op.length() - self.offset
        } else {
            usize::MAX
        }
    }

    /// Returns the type of the current operation as a string
    ///
    /// Returns "retain" if the iterator is exhausted (matching the
    /// behavior of returning infinite retains).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::{Op, OpIterator};
    ///
    /// let ops = vec![
    ///     Op::Insert { text: "Hi".to_string(), attributes: None },
    ///     Op::Delete(3),
    /// ];
    /// let mut iter = OpIterator::new(&ops);
    ///
    /// assert_eq!(iter.peek_type(), "insert");
    /// iter.next(None);
    /// assert_eq!(iter.peek_type(), "delete");
    /// iter.next(None);
    /// assert_eq!(iter.peek_type(), "retain"); // exhausted
    /// ```
    pub fn peek_type(&self) -> &'static str {
        if let Some(op) = self.ops.get(self.index) {
            op.op_type()
        } else {
            "retain"
        }
    }

    /// Consumes the iterator and returns all remaining operations
    ///
    /// If the current operation has been partially consumed, the
    /// remaining portion is included as the first element.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_delta::{Op, OpIterator};
    ///
    /// let ops = vec![
    ///     Op::Insert { text: "Hello".to_string(), attributes: None },
    ///     Op::Delete(5),
    /// ];
    /// let mut iter = OpIterator::new(&ops);
    ///
    /// // Partially consume first op
    /// iter.next(Some(2));
    ///
    /// let rest = iter.rest();
    /// assert_eq!(rest.len(), 2); // "llo" + Delete(5)
    /// ```
    pub fn rest(&mut self) -> Vec<Op> {
        if !self.has_next() {
            return Vec::new();
        }

        if self.offset == 0 {
            // No partial operation, return the rest directly
            self.ops[self.index..].to_vec()
        } else {
            // There's a partial operation, need to include it
            let mut result = Vec::new();
            let current = self.next(None);
            result.push(current);
            result.extend_from_slice(&self.ops[self.index..]);
            result
        }
    }

    /// Slices an operation to extract a specific portion
    ///
    /// This method handles the complexity of extracting a substring from
    /// text operations while preserving attributes. For atomic operations
    /// like embeds, it returns the whole operation.
    ///
    /// # Arguments
    ///
    /// * `op` - The operation to slice
    /// * `offset` - Starting position within the operation
    /// * `length` - Number of characters to extract
    ///
    /// # Returns
    ///
    /// A new operation containing the specified slice with attributes preserved.
    fn slice_op(&self, op: &Op, offset: usize, length: usize) -> Op {
        match op {
            Op::Insert { text, attributes } => {
                let chars: Vec<char> = text.chars().collect();
                let sliced_text: String = chars[offset..offset + length].iter().collect();
                Op::Insert {
                    text: sliced_text,
                    attributes: attributes.clone(),
                }
            }
            Op::InsertEmbed { embed, attributes } => {
                // Embeds are atomic, can't be sliced
                Op::InsertEmbed {
                    embed: embed.clone(),
                    attributes: attributes.clone(),
                }
            }
            Op::Delete(_) => Op::Delete(length),
            Op::Retain { attributes, .. } => Op::Retain {
                length,
                attributes: attributes.clone(),
            },
            Op::RetainEmbed { embed, attributes } => {
                // Embed retains are atomic
                Op::RetainEmbed {
                    embed: embed.clone(),
                    attributes: attributes.clone(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_op_iterator_basic() {
        let ops = vec![
            Op::Insert {
                text: "Hello".to_string(),
                attributes: None,
            },
            Op::Retain {
                length: 3,
                attributes: None,
            },
            Op::Delete(2),
        ];

        let mut iter = OpIterator::new(&ops);

        assert!(iter.has_next());
        assert_eq!(iter.peek_type(), "insert");
        assert_eq!(iter.peek_length(), 5);

        let first = iter.next(None);
        if let Op::Insert { text, .. } = first {
            assert_eq!(text, "Hello");
        } else {
            panic!("Expected insert operation");
        }

        assert_eq!(iter.peek_type(), "retain");
        assert_eq!(iter.peek_length(), 3);

        let second = iter.next(None);
        if let Op::Retain { length, .. } = second {
            assert_eq!(length, 3);
        } else {
            panic!("Expected retain operation");
        }

        assert_eq!(iter.peek_type(), "delete");
        let third = iter.next(None);
        if let Op::Delete(len) = third {
            assert_eq!(len, 2);
        } else {
            panic!("Expected delete operation");
        }

        assert!(!iter.has_next());
    }

    #[test]
    fn test_op_iterator_slicing() {
        let ops = vec![Op::Insert {
            text: "Hello World".to_string(),
            attributes: None,
        }];

        let mut iter = OpIterator::new(&ops);

        // Take first 5 characters
        let first = iter.next(Some(5));
        if let Op::Insert { text, .. } = first {
            assert_eq!(text, "Hello");
        } else {
            panic!("Expected insert operation");
        }

        assert_eq!(iter.peek_length(), 6); // " World" remaining

        // Take the rest
        let second = iter.next(None);
        if let Op::Insert { text, .. } = second {
            assert_eq!(text, " World");
        } else {
            panic!("Expected insert operation");
        }

        assert!(!iter.has_next());
    }

    #[test]
    fn test_op_iterator_rest() {
        let ops = vec![
            Op::Insert {
                text: "Hello".to_string(),
                attributes: None,
            },
            Op::Retain {
                length: 3,
                attributes: None,
            },
        ];

        let mut iter = OpIterator::new(&ops);
        iter.next(None); // Consume first operation

        let rest = iter.rest();
        assert_eq!(rest.len(), 1);
        if let Op::Retain { length, .. } = &rest[0] {
            assert_eq!(*length, 3);
        } else {
            panic!("Expected retain operation");
        }
    }

    #[test]
    fn test_op_iterator_partial_rest() {
        let ops = vec![Op::Insert {
            text: "Hello World".to_string(),
            attributes: None,
        }];

        let mut iter = OpIterator::new(&ops);
        iter.next(Some(5)); // Partially consume "Hello"

        let rest = iter.rest();
        assert_eq!(rest.len(), 1);
        if let Op::Insert { text, .. } = &rest[0] {
            assert_eq!(text, " World");
        } else {
            panic!("Expected insert operation");
        }
    }
}

