use crate::op::Op;

/// Iterator for efficiently traversing and slicing operations
#[derive(Debug, Clone)]
pub struct OpIterator {
    ops: Vec<Op>,
    index: usize,
    offset: usize,
}

impl OpIterator {
    /// Create a new OpIterator from a slice of operations
    pub fn new(ops: &[Op]) -> Self {
        Self {
            ops: ops.to_vec(),
            index: 0,
            offset: 0,
        }
    }

    /// Check if there are more operations to process
    pub fn has_next(&self) -> bool {
        self.peek_length() < usize::MAX
    }

    /// Get the next operation, optionally with a length limit
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

    /// Peek at the current operation without consuming it
    pub fn peek(&self) -> Option<&Op> {
        self.ops.get(self.index)
    }

    /// Get the remaining length of the current operation
    pub fn peek_length(&self) -> usize {
        if let Some(op) = self.ops.get(self.index) {
            op.length() - self.offset
        } else {
            usize::MAX
        }
    }

    /// Get the type of the current operation
    pub fn peek_type(&self) -> &'static str {
        if let Some(op) = self.ops.get(self.index) {
            op.op_type()
        } else {
            "retain"
        }
    }

    /// Get the remaining operations as a new vector
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

    /// Slice an operation to a specific length and offset
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

