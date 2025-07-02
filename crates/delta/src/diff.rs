//! Text diffing functionality for the Delta format
//!
//! This module provides text diffing algorithms used to generate Delta
//! operations that transform one document into another. The diff algorithm
//! identifies the minimal set of changes (insertions, deletions) needed
//! to transform the source text into the target text.
//!
//! # Algorithm
//!
//! The current implementation uses a simple algorithm that:
//! 1. Identifies common prefix between texts
//! 2. Identifies common suffix between texts
//! 3. Treats the middle portion as delete + insert
//!
//! This approach is optimized for correctness and simplicity rather than
//! finding the absolute minimal edit distance.

/// Types of operations in a text diff
///
/// These correspond to the fundamental ways text can differ between
/// two versions:
/// - **Equal**: Text that is the same in both versions
/// - **Insert**: Text that appears in the target but not the source
/// - **Delete**: Text that appears in the source but not the target
#[derive(Debug, Clone, PartialEq)]
pub enum DiffType {
    /// Text that is unchanged between versions
    Equal,
    /// Text that needs to be inserted
    Insert,
    /// Text that needs to be deleted
    Delete,
}

/// Represents a single operation in a text diff
///
/// A diff operation combines a type (Equal, Insert, Delete) with the
/// text content affected by that operation. A sequence of DiffOps
/// describes how to transform one text into another.
///
/// # Examples
///
/// ```rust
/// use quillai_delta::diff::{DiffOp, DiffType};
///
/// // Represent deleting "Hello" and inserting "Hi"
/// let delete = DiffOp::new(DiffType::Delete, "Hello".to_string());
/// let insert = DiffOp::new(DiffType::Insert, "Hi".to_string());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct DiffOp {
    /// The type of diff operation
    pub operation: DiffType,
    /// The text content affected by this operation
    pub text: String,
}

impl DiffOp {
    /// Creates a new diff operation
    ///
    /// # Arguments
    ///
    /// * `operation` - The type of diff operation
    /// * `text` - The text content for this operation
    pub fn new(operation: DiffType, text: String) -> Self {
        Self { operation, text }
    }

    /// Returns the length of text affected by this operation
    ///
    /// Length is measured in Unicode characters, not bytes.
    pub fn length(&self) -> usize {
        self.text.chars().count()
    }
}

/// Computes the diff between two text strings
///
/// This function identifies the differences between two strings and returns
/// a sequence of operations that would transform `text1` into `text2`.
///
/// The algorithm used is a simple approach that:
/// 1. Finds the longest common prefix
/// 2. Finds the longest common suffix (in the remaining text)
/// 3. Treats everything in between as a delete (from text1) followed by an insert (from text2)
///
/// While this doesn't always produce the minimal edit sequence, it's efficient
/// and produces reasonable results for most text editing scenarios.
///
/// # Arguments
///
/// * `text1` - The source text
/// * `text2` - The target text
///
/// # Returns
///
/// A vector of `DiffOp` operations that transform `text1` into `text2`.
///
/// # Examples
///
/// ```rust
/// use quillai_delta::diff::{diff_text, DiffType};
///
/// // Simple replacement
/// let ops = diff_text("Hello World", "Hello Rust");
/// // Results in: Equal("Hello "), Delete("World"), Insert("Rust")
///
/// // Insertion
/// let ops = diff_text("Hello", "Hello World");
/// // Results in: Equal("Hello"), Insert(" World")
///
/// // Deletion
/// let ops = diff_text("Hello World", "Hello");
/// // Results in: Equal("Hello"), Delete(" World")
/// ```
pub fn diff_text(text1: &str, text2: &str) -> Vec<DiffOp> {
    if text1 == text2 {
        if text1.is_empty() {
            return Vec::new();
        }
        return vec![DiffOp::new(DiffType::Equal, text1.to_string())];
    }

    if text1.is_empty() {
        return vec![DiffOp::new(DiffType::Insert, text2.to_string())];
    }

    if text2.is_empty() {
        return vec![DiffOp::new(DiffType::Delete, text1.to_string())];
    }

    // Find common prefix
    let chars1: Vec<char> = text1.chars().collect();
    let chars2: Vec<char> = text2.chars().collect();
    
    let mut prefix_len = 0;
    while prefix_len < chars1.len() 
        && prefix_len < chars2.len() 
        && chars1[prefix_len] == chars2[prefix_len] 
    {
        prefix_len += 1;
    }

    // Find common suffix
    let mut suffix_len = 0;
    while suffix_len < (chars1.len() - prefix_len)
        && suffix_len < (chars2.len() - prefix_len)
        && chars1[chars1.len() - 1 - suffix_len] == chars2[chars2.len() - 1 - suffix_len]
    {
        suffix_len += 1;
    }

    let mut result = Vec::new();

    // Add common prefix
    if prefix_len > 0 {
        let prefix: String = chars1[..prefix_len].iter().collect();
        result.push(DiffOp::new(DiffType::Equal, prefix));
    }

    // Add middle differences
    let middle1_start = prefix_len;
    let middle1_end = chars1.len() - suffix_len;
    let middle2_start = prefix_len;
    let middle2_end = chars2.len() - suffix_len;

    if middle1_start < middle1_end {
        let deleted: String = chars1[middle1_start..middle1_end].iter().collect();
        result.push(DiffOp::new(DiffType::Delete, deleted));
    }

    if middle2_start < middle2_end {
        let inserted: String = chars2[middle2_start..middle2_end].iter().collect();
        result.push(DiffOp::new(DiffType::Insert, inserted));
    }

    // Add common suffix
    if suffix_len > 0 {
        let suffix_start = chars1.len() - suffix_len;
        let suffix: String = chars1[suffix_start..].iter().collect();
        result.push(DiffOp::new(DiffType::Equal, suffix));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_identical() {
        let result = diff_text("hello", "hello");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].operation, DiffType::Equal);
        assert_eq!(result[0].text, "hello");
    }

    #[test]
    fn test_diff_empty() {
        let result = diff_text("", "");
        assert_eq!(result.len(), 0);

        let result = diff_text("hello", "");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].operation, DiffType::Delete);
        assert_eq!(result[0].text, "hello");

        let result = diff_text("", "hello");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].operation, DiffType::Insert);
        assert_eq!(result[0].text, "hello");
    }

    #[test]
    fn test_diff_replacement() {
        let result = diff_text("abc", "axc");
        // Debug print to see what we're actually getting
        for (i, op) in result.iter().enumerate() {
            println!("Op {}: {:?} '{}'", i, op.operation, op.text);
        }
        
        // My algorithm creates: Equal("a"), Delete("b"), Insert("x"), Equal("c")
        assert_eq!(result.len(), 4);
        assert_eq!(result[0].operation, DiffType::Equal);
        assert_eq!(result[0].text, "a");
        assert_eq!(result[1].operation, DiffType::Delete);
        assert_eq!(result[1].text, "b");
        assert_eq!(result[2].operation, DiffType::Insert);
        assert_eq!(result[2].text, "x");
        assert_eq!(result[3].operation, DiffType::Equal);
        assert_eq!(result[3].text, "c");
    }

    #[test]
    fn test_diff_insertion() {
        let result = diff_text("ac", "abc");
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].operation, DiffType::Equal);
        assert_eq!(result[0].text, "a");
        assert_eq!(result[1].operation, DiffType::Insert);
        assert_eq!(result[1].text, "b");
        assert_eq!(result[2].operation, DiffType::Equal);
        assert_eq!(result[2].text, "c");
    }

    #[test]
    fn test_diff_deletion() {
        let result = diff_text("abc", "ac");
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].operation, DiffType::Equal);
        assert_eq!(result[0].text, "a");
        assert_eq!(result[1].operation, DiffType::Delete);
        assert_eq!(result[1].text, "b");
        assert_eq!(result[2].operation, DiffType::Equal);
        assert_eq!(result[2].text, "c");
    }
}