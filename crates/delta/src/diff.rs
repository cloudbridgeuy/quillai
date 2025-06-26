/// Represents a diff operation type
#[derive(Debug, Clone, PartialEq)]
pub enum DiffType {
    Equal,
    Insert,
    Delete,
}

/// Represents a single diff operation
#[derive(Debug, Clone, PartialEq)]
pub struct DiffOp {
    pub operation: DiffType,
    pub text: String,
}

impl DiffOp {
    pub fn new(operation: DiffType, text: String) -> Self {
        Self { operation, text }
    }

    pub fn length(&self) -> usize {
        self.text.chars().count()
    }
}

/// Simple text diffing implementation
/// This is a basic implementation focused on correctness over performance
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