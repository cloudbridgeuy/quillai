use crate::attributes::{AttributeMap, AttributeMapOps};
use crate::diff::{diff_text, DiffType};
use crate::op::{EmbedData, Op};
use crate::op_iterator::OpIterator;
use serde_json::Value as JsonValue;

/// Main Delta struct representing a document or change
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delta {
    ops: Vec<Op>,
}

impl Delta {
    /// Create a new empty Delta
    pub fn new() -> Self {
        Self { ops: Vec::new() }
    }

    /// Create a Delta from a vector of operations
    pub fn from_ops(ops: Vec<Op>) -> Self {
        Self { ops }
    }

    /// Get the operations as a slice
    pub fn ops(&self) -> &[Op] {
        &self.ops
    }

    /// Get the operations as a mutable slice
    pub fn ops_mut(&mut self) -> &mut Vec<Op> {
        &mut self.ops
    }

    /// Insert text with optional attributes
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

    /// Insert an embed with optional attributes
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

    /// Delete a number of characters
    pub fn delete(self, length: usize) -> Self {
        if length == 0 {
            return self;
        }
        self.push(Op::Delete(length))
    }

    /// Retain a number of characters with optional attribute changes
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

    /// Retain an embed with optional attribute changes
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

    /// Push an operation onto the Delta, optimizing when possible
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

    /// Check if two operations can be merged
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

    /// Merge two compatible operations
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

    /// Remove trailing retain without attributes (chop)
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

    /// Calculate the length of the Delta
    pub fn length(&self) -> usize {
        self.ops.iter().map(|op| op.length()).sum()
    }

    /// Calculate the change in length this Delta would cause
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

    /// Filter operations based on a predicate
    pub fn filter<F>(&self, predicate: F) -> Vec<&Op>
    where
        F: Fn(&Op) -> bool,
    {
        self.ops.iter().filter(|op| predicate(op)).collect()
    }

    /// Apply a function to each operation
    pub fn for_each<F>(&self, mut f: F)
    where
        F: FnMut(&Op),
    {
        for op in &self.ops {
            f(op);
        }
    }

    /// Map operations to another type
    pub fn map<T, F>(&self, f: F) -> Vec<T>
    where
        F: Fn(&Op) -> T,
    {
        self.ops.iter().map(f).collect()
    }

    /// Partition operations into two groups
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

    /// Reduce operations to a single value
    pub fn reduce<T, F>(&self, initial: T, f: F) -> T
    where
        F: Fn(T, &Op) -> T,
    {
        self.ops.iter().fold(initial, f)
    }

    /// Get a slice of the Delta
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

    /// Concatenate with another Delta
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

    /// Compose this Delta with another Delta
    /// Returns a Delta that represents applying this Delta followed by the other Delta
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

    /// Transform this Delta against another Delta for operational transformation
    /// If priority is true, this Delta takes precedence in case of conflicts
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

    /// Transform a position index against this Delta
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

    /// Create a diff between this Delta and another Delta (both must be documents)
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

    /// Create an inverted Delta that undoes this Delta when applied to the given base document
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

    /// Convert Delta to plain text (for documents only)
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
