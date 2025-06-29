//! Advanced text manipulation and analysis operations.
//!
//! This module provides comprehensive text processing capabilities for Parchment's
//! document model, including selection management, search functionality, statistical
//! analysis, and text traversal utilities.
//!
//! # Core Components
//!
//! - **Selection Management**: [`TextSelection`] for representing text ranges
//! - **Search Operations**: [`TextMatch`] and [`TextSearcher`] for pattern matching
//! - **Position Tracking**: [`Position`] for document coordinates
//! - **Statistical Analysis**: [`TextStatistics`] for document metrics
//! - **Text Utilities**: [`TextUtils`] for character and word analysis
//! - **Visitor Pattern**: [`TextVisitor`] for document traversal
//!
//! # Usage Examples
//!
//! ## Text Selection
//!
//! ```rust
//! use quillai_parchment::text_operations::TextSelection;
//!
//! let selection = TextSelection::new(
//!     vec![0, 1],  // start path
//!     5,           // start offset
//!     vec![0, 2],  // end path
//!     10           // end offset
//! );
//! ```
//!
//! ## Text Search
//!
//! ```rust
//! use quillai_parchment::text_operations::TextSearcher;
//!
//! let mut searcher = TextSearcher::new("hello".to_string(), false);
//! // Use with visitor pattern to search through document
//! ```
//!
//! ## Text Statistics
//!
//! ```rust
//! use quillai_parchment::text_operations::TextUtils;
//!
//! let text = "Hello world! This is a test.";
//! let word_count = TextUtils::count_words(text);
//! let char_count = TextUtils::count_characters(text, true);
//! ```
//!
//! # WASM Integration
//!
//! All public types are marked with `#[wasm_bindgen]` for seamless JavaScript
//! integration, enabling rich text operations in web environments.

use wasm_bindgen::prelude::*;

/// Represents a text selection range within the document.
///
/// A text selection defines a range of text that can span multiple nodes
/// in the document tree. It uses path-based addressing to identify the
/// start and end positions within the document structure.
///
/// # Path-Based Addressing
///
/// Paths are arrays of child indices that describe how to navigate from
/// the document root to a specific node:
/// - `[0]` - First child of root
/// - `[0, 2]` - Third child of first child of root
/// - `[1, 0, 3]` - Fourth child of first child of second child of root
///
/// # WASM Integration
///
/// This struct is exposed to JavaScript and can be used in web applications:
///
/// ```javascript
/// const selection = new TextSelection([0, 1], 5, [0, 2], 10);
/// console.log(selection.is_collapsed); // false
/// console.log(selection.start_path); // [0, 1]
/// ```
#[wasm_bindgen]
#[derive(Clone)]
pub struct TextSelection {
    /// Character offset within the starting text node
    pub start_offset: u32,
    /// Character offset within the ending text node
    pub end_offset: u32,
    /// Whether the selection is collapsed (start position equals end position)
    pub is_collapsed: bool,
    /// Path to the starting node (array of child indices)
    start_path: Vec<u32>,
    /// Path to the ending node (array of child indices)
    end_path: Vec<u32>,
}

#[wasm_bindgen]
impl TextSelection {
    /// Create a new text selection.
    ///
    /// # Arguments
    ///
    /// * `start_path` - Path to the starting node
    /// * `start_offset` - Character offset within the starting node
    /// * `end_path` - Path to the ending node
    /// * `end_offset` - Character offset within the ending node
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_parchment::text_operations::TextSelection;
    ///
    /// // Create a selection spanning from position 5 in node [0,1] to position 10 in node [0,2]
    /// let selection = TextSelection::new(vec![0, 1], 5, vec![0, 2], 10);
    /// assert!(!selection.is_collapsed);
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(
        start_path: Vec<u32>,
        start_offset: u32,
        end_path: Vec<u32>,
        end_offset: u32,
    ) -> TextSelection {
        let is_collapsed = start_path == end_path && start_offset == end_offset;
        TextSelection {
            start_path,
            start_offset,
            end_path,
            end_offset,
            is_collapsed,
        }
    }

    /// Get the path to the starting node.
    ///
    /// Returns a copy of the start path for use in JavaScript environments.
    ///
    /// # Returns
    ///
    /// Vector of child indices representing the path to the start node
    #[wasm_bindgen(getter)]
    pub fn start_path(&self) -> Vec<u32> {
        self.start_path.clone()
    }

    /// Get the path to the ending node.
    ///
    /// Returns a copy of the end path for use in JavaScript environments.
    ///
    /// # Returns
    ///
    /// Vector of child indices representing the path to the end node
    #[wasm_bindgen(getter)]
    pub fn end_path(&self) -> Vec<u32> {
        self.end_path.clone()
    }
}

/// Represents a text match found during search operations.
///
/// A text match contains information about a pattern that was found within
/// the document, including its location, extent, and the actual matched text.
/// This is typically used as the result of search operations.
///
/// # Usage
///
/// Text matches are usually created by search operations and contain both
/// the location information (paths and offsets) and the matched content.
///
/// # WASM Integration
///
/// This struct is exposed to JavaScript for web-based text search:
///
/// ```javascript
/// // Assuming a search operation returned matches
/// for (const match of searchResults) {
///     console.log(`Found "${match.matched_text}" at offset ${match.start_offset}`);
/// }
/// ```
#[wasm_bindgen]
#[derive(Clone)]
pub struct TextMatch {
    /// Character offset where the match begins
    pub start_offset: u32,
    /// Character offset where the match ends
    pub end_offset: u32,
    /// Path to the node where the match starts
    start_path: Vec<u32>,
    /// Path to the node where the match ends
    end_path: Vec<u32>,
    /// The actual text content that was matched
    matched_text: String,
}

#[wasm_bindgen]
impl TextMatch {
    /// Create a new text match.
    ///
    /// # Arguments
    ///
    /// * `start_path` - Path to the node where the match begins
    /// * `start_offset` - Character offset within the starting node
    /// * `end_path` - Path to the node where the match ends
    /// * `end_offset` - Character offset within the ending node
    /// * `matched_text` - The actual text content that was matched
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_parchment::text_operations::TextMatch;
    ///
    /// let text_match = TextMatch::new(
    ///     vec![0, 1],
    ///     5,
    ///     vec![0, 1],
    ///     10,
    ///     "hello".to_string()
    /// );
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(
        start_path: Vec<u32>,
        start_offset: u32,
        end_path: Vec<u32>,
        end_offset: u32,
        matched_text: String,
    ) -> TextMatch {
        TextMatch {
            start_path,
            start_offset,
            end_path,
            end_offset,
            matched_text,
        }
    }

    /// Get the path to the starting node.
    ///
    /// Returns a copy of the start path for use in JavaScript environments.
    ///
    /// # Returns
    ///
    /// Vector of child indices representing the path to the start node
    #[wasm_bindgen(getter)]
    pub fn start_path(&self) -> Vec<u32> {
        self.start_path.clone()
    }

    /// Get the path to the ending node.
    ///
    /// Returns a copy of the end path for use in JavaScript environments.
    ///
    /// # Returns
    ///
    /// Vector of child indices representing the path to the end node
    #[wasm_bindgen(getter)]
    pub fn end_path(&self) -> Vec<u32> {
        self.end_path.clone()
    }

    /// Get the matched text content.
    ///
    /// Returns a copy of the matched text for use in JavaScript environments.
    ///
    /// # Returns
    ///
    /// The actual text content that was matched during the search
    #[wasm_bindgen(getter)]
    pub fn matched_text(&self) -> String {
        self.matched_text.clone()
    }
}

/// Represents a single position within the document.
///
/// A position defines a specific location in the document tree using
/// path-based addressing and a character offset within the target node.
/// This is commonly used for cursor positioning and range boundaries.
///
/// # Usage
///
/// Positions are used throughout Parchment for:
/// - Cursor positioning
/// - Selection boundaries
/// - Insert/delete operations
/// - Navigation operations
///
/// # WASM Integration
///
/// This struct is exposed to JavaScript for web-based document manipulation:
///
/// ```javascript
/// const position = new Position([0, 1, 2], 15);
/// console.log(position.offset); // 15
/// console.log(position.path); // [0, 1, 2]
/// ```
#[wasm_bindgen]
#[derive(Clone)]
pub struct Position {
    /// Character offset within the target node
    pub offset: u32,
    /// Path to the target node (array of child indices)
    path: Vec<u32>,
}

#[wasm_bindgen]
impl Position {
    /// Create a new position.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the target node
    /// * `offset` - Character offset within the node
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_parchment::text_operations::Position;
    ///
    /// // Create a position at character 15 in the node at path [0, 1, 2]
    /// let position = Position::new(vec![0, 1, 2], 15);
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(path: Vec<u32>, offset: u32) -> Position {
        Position { path, offset }
    }

    /// Get the path to the target node.
    ///
    /// Returns a copy of the path for use in JavaScript environments.
    ///
    /// # Returns
    ///
    /// Vector of child indices representing the path to the target node
    #[wasm_bindgen(getter)]
    pub fn path(&self) -> Vec<u32> {
        self.path.clone()
    }
}

/// Comprehensive text statistics for document analysis.
///
/// This struct provides detailed metrics about the text content within
/// a document, including counts for words, characters, paragraphs, lines,
/// and sentences. These statistics are useful for document analysis,
/// word processing features, and content validation.
///
/// # Calculation Methods
///
/// - **Words**: Unicode-aware word boundary detection
/// - **Characters**: Full Unicode character counting with/without spaces
/// - **Paragraphs**: Count of block-level elements
/// - **Lines**: Estimated based on content length and line breaks
/// - **Sentences**: Heuristic detection using punctuation patterns
///
/// # WASM Integration
///
/// This struct is exposed to JavaScript for web-based document analysis:
///
/// ```javascript
/// const stats = new TextStatistics(150, 750, 650, 5, 12, 8);
/// console.log(`Document has ${stats.words} words and ${stats.sentences} sentences`);
/// ```
#[wasm_bindgen]
pub struct TextStatistics {
    /// Total number of words in the document
    pub words: u32,
    /// Total character count including whitespace
    pub characters: u32,
    /// Character count excluding whitespace characters
    pub characters_no_spaces: u32,
    /// Number of paragraph-level block elements
    pub paragraphs: u32,
    /// Estimated number of lines (including wrapped text)
    pub lines: u32,
    /// Estimated number of sentences based on punctuation
    pub sentences: u32,
}

#[wasm_bindgen]
impl TextStatistics {
    /// Create a new text statistics object.
    ///
    /// # Arguments
    ///
    /// * `words` - Total word count
    /// * `characters` - Total character count including spaces
    /// * `characters_no_spaces` - Character count excluding spaces
    /// * `paragraphs` - Number of paragraph elements
    /// * `lines` - Estimated line count
    /// * `sentences` - Estimated sentence count
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_parchment::text_operations::TextStatistics;
    ///
    /// let stats = TextStatistics::new(150, 750, 650, 5, 12, 8);
    /// assert_eq!(stats.words, 150);
    /// assert_eq!(stats.characters, 750);
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(
        words: u32,
        characters: u32,
        characters_no_spaces: u32,
        paragraphs: u32,
        lines: u32,
        sentences: u32,
    ) -> TextStatistics {
        TextStatistics {
            words,
            characters,
            characters_no_spaces,
            paragraphs,
            lines,
            sentences,
        }
    }
}

/// Utility functions for advanced text analysis and processing.
///
/// This struct provides static methods for various text analysis operations
/// including word boundary detection, sentence parsing, character counting,
/// and statistical analysis. These utilities support the higher-level text
/// operations used throughout Parchment.
///
/// # Features
///
/// - **Unicode-aware processing**: Proper handling of international text
/// - **Word boundary detection**: Intelligent word segmentation
/// - **Sentence parsing**: Heuristic sentence boundary detection
/// - **Statistical analysis**: Comprehensive text metrics
/// - **Performance optimized**: Efficient algorithms for large documents
pub struct TextUtils;

impl TextUtils {
    /// Check if a character represents a word boundary.
    ///
    /// A word boundary is defined as whitespace, ASCII punctuation, or any
    /// non-alphanumeric character. This is used for word counting and
    /// text segmentation operations.
    ///
    /// # Arguments
    ///
    /// * `ch` - The character to test
    ///
    /// # Returns
    ///
    /// `true` if the character is a word boundary, `false` otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_parchment::text_operations::TextUtils;
    ///
    /// assert!(TextUtils::is_word_boundary(' '));  // whitespace
    /// assert!(TextUtils::is_word_boundary('.'));  // punctuation
    /// assert!(!TextUtils::is_word_boundary('a')); // alphanumeric
    /// ```
    pub fn is_word_boundary(ch: char) -> bool {
        ch.is_whitespace() || ch.is_ascii_punctuation() || !ch.is_alphanumeric()
    }

    /// Check if a position in text represents a sentence boundary.
    ///
    /// A sentence boundary is detected by looking for sentence-ending punctuation
    /// (period, exclamation mark, question mark) followed by whitespace or end of text.
    /// This is a heuristic approach that works well for most text.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to analyze
    /// * `pos` - The character position to check
    ///
    /// # Returns
    ///
    /// `true` if the position represents a sentence boundary, `false` otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_parchment::text_operations::TextUtils;
    ///
    /// let text = "Hello world. This is a test!";
    /// assert!(TextUtils::is_sentence_boundary(text, 11)); // after "world."
    /// assert!(!TextUtils::is_sentence_boundary(text, 5)); // middle of "world"
    /// ```
    pub fn is_sentence_boundary(text: &str, pos: usize) -> bool {
        if pos == 0 || pos >= text.len() {
            return false;
        }

        let chars: Vec<char> = text.chars().collect();
        if pos >= chars.len() {
            return false;
        }

        let current_char = chars[pos];

        // Check for sentence-ending punctuation
        matches!(current_char, '.' | '!' | '?') &&
        // Followed by whitespace or end of text
        (pos + 1 >= chars.len() || chars[pos + 1].is_whitespace())
    }

    /// Count words in text using Unicode-aware word boundaries.
    ///
    /// This method provides accurate word counting that handles Unicode text
    /// properly, using the word boundary detection logic to segment text into
    /// individual words.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to analyze
    ///
    /// # Returns
    ///
    /// The number of words found in the text
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_parchment::text_operations::TextUtils;
    ///
    /// assert_eq!(TextUtils::count_words("Hello world"), 2);
    /// assert_eq!(TextUtils::count_words(""), 0);
    /// assert_eq!(TextUtils::count_words("One-word"), 2); // hyphenated counts as two
    /// ```
    pub fn count_words(text: &str) -> u32 {
        if text.is_empty() {
            return 0;
        }

        let mut word_count = 0;
        let mut in_word = false;

        for ch in text.chars() {
            if Self::is_word_boundary(ch) {
                if in_word {
                    word_count += 1;
                    in_word = false;
                }
            } else {
                in_word = true;
            }
        }

        // Count the last word if text doesn't end with a boundary
        if in_word {
            word_count += 1;
        }

        word_count
    }

    /// Count characters in text with option to include or exclude whitespace.
    ///
    /// This method provides Unicode-aware character counting, which is important
    /// for international text that may contain multi-byte characters.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to analyze
    /// * `include_spaces` - Whether to include whitespace characters in the count
    ///
    /// # Returns
    ///
    /// The number of characters found in the text
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_parchment::text_operations::TextUtils;
    ///
    /// let text = "Hello world";
    /// assert_eq!(TextUtils::count_characters(text, true), 11);  // with space
    /// assert_eq!(TextUtils::count_characters(text, false), 10); // without space
    /// ```
    pub fn count_characters(text: &str, include_spaces: bool) -> u32 {
        if include_spaces {
            text.chars().count() as u32
        } else {
            text.chars().filter(|ch| !ch.is_whitespace()).count() as u32
        }
    }

    /// Count sentences in text using heuristic boundary detection.
    ///
    /// This method uses the sentence boundary detection logic to count the
    /// number of sentences in the given text. The detection is heuristic-based
    /// and works well for most standard text.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to analyze
    ///
    /// # Returns
    ///
    /// The estimated number of sentences in the text
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_parchment::text_operations::TextUtils;
    ///
    /// let text = "Hello world. This is a test! How are you?";
    /// assert_eq!(TextUtils::count_sentences(text), 3);
    /// ```
    pub fn count_sentences(text: &str) -> u32 {
        if text.is_empty() {
            return 0;
        }

        let mut sentence_count = 0;
        let chars: Vec<char> = text.chars().collect();

        for i in 0..chars.len() {
            if Self::is_sentence_boundary(text, i) {
                sentence_count += 1;
            }
        }

        sentence_count
    }

    /// Estimate line count based on text length and line breaks.
    ///
    /// This method estimates the number of lines by counting explicit line breaks
    /// and estimating wrapped lines based on an assumed average line length of
    /// 80 characters. This provides a reasonable approximation for display purposes.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to analyze
    ///
    /// # Returns
    ///
    /// The estimated number of lines in the text
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_parchment::text_operations::TextUtils;
    ///
    /// let text = "Short line\nAnother line";
    /// assert_eq!(TextUtils::estimate_lines(text), 2);
    /// ```
    pub fn estimate_lines(text: &str) -> u32 {
        if text.is_empty() {
            return 0;
        }

        // Count explicit line breaks
        let explicit_lines = text.matches('\n').count() as u32 + 1;

        // Estimate wrapped lines (assuming ~80 characters per line)
        let char_count = text.chars().count() as u32;
        let estimated_wrapped_lines = (char_count / 80).max(1);

        explicit_lines.max(estimated_wrapped_lines)
    }
}

/// Trait for implementing the visitor pattern on text nodes.
///
/// This trait enables traversal of document text content using the visitor pattern.
/// Implementations can perform various operations on text nodes such as collection,
/// search, analysis, or transformation.
///
/// # Usage
///
/// The visitor pattern is used throughout Parchment for text processing operations
/// that need to traverse the document tree and operate on text content.
///
/// # Examples
///
/// ```rust
/// use quillai_parchment::text_operations::{TextVisitor, TextCollector};
///
/// let mut collector = TextCollector::new();
/// // Use with document traversal to collect all text
/// ```
pub trait TextVisitor {
    /// Called for each text node encountered during document traversal.
    ///
    /// # Arguments
    ///
    /// * `text` - The text content of the current node
    /// * `path` - The path to the current node in the document tree
    fn visit_text(&mut self, text: &str, path: &[u32]);
}

/// A visitor implementation that collects all text content.
///
/// This visitor traverses the document and concatenates all text content
/// into a single string. This is useful for operations like full-text
/// search, word counting, or exporting plain text.
///
/// # Examples
///
/// ```rust
/// use quillai_parchment::text_operations::TextCollector;
///
/// let mut collector = TextCollector::new();
/// // After traversal, collector.collected_text contains all text
/// ```
#[derive(Default)]
pub struct TextCollector {
    /// The accumulated text content from all visited nodes
    pub collected_text: String,
}

impl TextCollector {
    /// Create a new text collector.
    ///
    /// # Returns
    ///
    /// A new `TextCollector` with an empty text buffer
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_parchment::text_operations::TextCollector;
    ///
    /// let collector = TextCollector::new();
    /// assert!(collector.collected_text.is_empty());
    /// ```
    pub fn new() -> Self {
        TextCollector {
            collected_text: String::new(),
        }
    }
}

impl TextVisitor for TextCollector {
    /// Visit a text node and append its content to the collected text.
    ///
    /// # Arguments
    ///
    /// * `text` - The text content to append
    /// * `_path` - The path to the node (unused in this implementation)
    fn visit_text(&mut self, text: &str, _path: &[u32]) {
        self.collected_text.push_str(text);
    }
}

/// A visitor implementation that searches for text patterns.
///
/// This visitor traverses the document looking for occurrences of a specific
/// text pattern. It supports both case-sensitive and case-insensitive search
/// and collects all matches with their location information.
///
/// # Examples
///
/// ```rust
/// use quillai_parchment::text_operations::TextSearcher;
///
/// let mut searcher = TextSearcher::new("hello".to_string(), false);
/// // After traversal, searcher.matches contains all found occurrences
/// ```
pub struct TextSearcher {
    /// The pattern to search for
    pub pattern: String,
    /// Whether the search is case-sensitive
    pub case_sensitive: bool,
    /// Collection of all matches found during traversal
    pub matches: Vec<TextMatch>,
    /// Current character offset in the document
    current_offset: u32,
}

impl TextSearcher {
    /// Create a new text searcher.
    ///
    /// # Arguments
    ///
    /// * `pattern` - The text pattern to search for
    /// * `case_sensitive` - Whether the search should be case-sensitive
    ///
    /// # Returns
    ///
    /// A new `TextSearcher` configured with the specified pattern and sensitivity
    ///
    /// # Examples
    ///
    /// ```rust
    /// use quillai_parchment::text_operations::TextSearcher;
    ///
    /// let searcher = TextSearcher::new("hello".to_string(), false);
    /// assert_eq!(searcher.pattern, "hello");
    /// assert!(!searcher.case_sensitive);
    /// ```
    pub fn new(pattern: String, case_sensitive: bool) -> Self {
        TextSearcher {
            pattern,
            case_sensitive,
            matches: Vec::new(),
            current_offset: 0,
        }
    }
}

impl TextVisitor for TextSearcher {
    /// Visit a text node and search for pattern matches.
    ///
    /// This method searches the provided text for occurrences of the configured
    /// pattern, respecting the case sensitivity setting. All matches are recorded
    /// with their location information.
    ///
    /// # Arguments
    ///
    /// * `text` - The text content to search within
    /// * `path` - The path to the current node in the document tree
    fn visit_text(&mut self, text: &str, path: &[u32]) {
        let search_text = if self.case_sensitive {
            text.to_string()
        } else {
            text.to_lowercase()
        };

        let search_pattern = if self.case_sensitive {
            self.pattern.clone()
        } else {
            self.pattern.to_lowercase()
        };

        let mut start_pos = 0;
        while let Some(match_pos) = search_text[start_pos..].find(&search_pattern) {
            let absolute_pos = start_pos + match_pos;
            let match_start = self.current_offset + absolute_pos as u32;
            let _match_end = match_start + search_pattern.len() as u32;

            let text_match = TextMatch::new(
                path.to_vec(),
                absolute_pos as u32,
                path.to_vec(),
                (absolute_pos + search_pattern.len()) as u32,
                text[absolute_pos..absolute_pos + search_pattern.len()].to_string(),
            );

            self.matches.push(text_match);
            start_pos = absolute_pos + search_pattern.len();
        }

        self.current_offset += text.len() as u32;
    }
}
