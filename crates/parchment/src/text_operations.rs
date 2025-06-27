use wasm_bindgen::prelude::*;

/// Represents a text selection range within the document
#[wasm_bindgen]
#[derive(Clone)]
pub struct TextSelection {
    /// Starting offset within the text node
    pub start_offset: u32,
    /// Ending offset within the text node
    pub end_offset: u32,
    /// Whether the selection is collapsed (start == end)
    pub is_collapsed: bool,
    /// Starting position path (array of child indices) - private field
    start_path: Vec<u32>,
    /// Ending position path (array of child indices) - private field
    end_path: Vec<u32>,
}

#[wasm_bindgen]
impl TextSelection {
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

    /// Get the start path as a JavaScript array
    #[wasm_bindgen(getter)]
    pub fn start_path(&self) -> Vec<u32> {
        self.start_path.clone()
    }

    /// Get the end path as a JavaScript array
    #[wasm_bindgen(getter)]
    pub fn end_path(&self) -> Vec<u32> {
        self.end_path.clone()
    }
}

/// Represents a text match found during search operations
#[wasm_bindgen]
#[derive(Clone)]
pub struct TextMatch {
    /// Starting offset within the text node
    pub start_offset: u32,
    /// Ending offset within the text node
    pub end_offset: u32,
    /// Starting position path - private field
    start_path: Vec<u32>,
    /// Ending position path - private field
    end_path: Vec<u32>,
    /// The matched text content - private field
    matched_text: String,
}

#[wasm_bindgen]
impl TextMatch {
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

    /// Get the start path as a JavaScript array
    #[wasm_bindgen(getter)]
    pub fn start_path(&self) -> Vec<u32> {
        self.start_path.clone()
    }

    /// Get the end path as a JavaScript array
    #[wasm_bindgen(getter)]
    pub fn end_path(&self) -> Vec<u32> {
        self.end_path.clone()
    }

    /// Get the matched text content
    #[wasm_bindgen(getter)]
    pub fn matched_text(&self) -> String {
        self.matched_text.clone()
    }
}

/// Represents a position within the document
#[wasm_bindgen]
#[derive(Clone)]
pub struct Position {
    /// Offset within the node
    pub offset: u32,
    /// Path to the node (array of child indices) - private field
    path: Vec<u32>,
}

#[wasm_bindgen]
impl Position {
    #[wasm_bindgen(constructor)]
    pub fn new(path: Vec<u32>, offset: u32) -> Position {
        Position { path, offset }
    }

    /// Get the path as a JavaScript array
    #[wasm_bindgen(getter)]
    pub fn path(&self) -> Vec<u32> {
        self.path.clone()
    }
}

/// Comprehensive text statistics for the document
#[wasm_bindgen]
pub struct TextStatistics {
    /// Total word count
    pub words: u32,
    /// Total character count including spaces
    pub characters: u32,
    /// Character count excluding spaces
    pub characters_no_spaces: u32,
    /// Number of paragraphs (block-level elements)
    pub paragraphs: u32,
    /// Number of lines (approximate)
    pub lines: u32,
    /// Number of sentences (approximate)
    pub sentences: u32,
}

#[wasm_bindgen]
impl TextStatistics {
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

/// Utility functions for text analysis
pub struct TextUtils;

impl TextUtils {
    /// Check if a character is a word boundary
    pub fn is_word_boundary(ch: char) -> bool {
        ch.is_whitespace() || ch.is_ascii_punctuation() || !ch.is_alphanumeric()
    }

    /// Check if a position in text is a sentence boundary
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

    /// Count words in text using Unicode-aware word boundaries
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

    /// Count characters with option to include/exclude spaces
    pub fn count_characters(text: &str, include_spaces: bool) -> u32 {
        if include_spaces {
            text.chars().count() as u32
        } else {
            text.chars().filter(|ch| !ch.is_whitespace()).count() as u32
        }
    }

    /// Count sentences in text
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

    /// Estimate line count based on text length and average line length
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

/// Trait for visiting text nodes during traversal
pub trait TextVisitor {
    /// Called for each text node encountered during traversal
    fn visit_text(&mut self, text: &str, path: &[u32]);
}

/// A visitor that collects all text content
#[derive(Default)]
pub struct TextCollector {
    pub collected_text: String,
}

impl TextCollector {
    pub fn new() -> Self {
        TextCollector {
            collected_text: String::new(),
        }
    }
}

impl TextVisitor for TextCollector {
    fn visit_text(&mut self, text: &str, _path: &[u32]) {
        self.collected_text.push_str(text);
    }
}

/// A visitor that searches for text patterns
pub struct TextSearcher {
    pub pattern: String,
    pub case_sensitive: bool,
    pub matches: Vec<TextMatch>,
    current_offset: u32,
}

impl TextSearcher {
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

