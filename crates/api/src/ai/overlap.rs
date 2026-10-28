const MAX_TAIL: usize = 512;
const MAX_BUFFER: usize = 4096;
const MIN_OVERLAP: usize = 12;

fn normalized(text: &str) -> Vec<(char, usize)> {
    let mut result = Vec::new();
    for (index, ch) in text.char_indices() {
        let end = index + ch.len_utf8();
        if ch.is_whitespace() {
            if result.last().is_some_and(|(last, _)| *last == ' ') {
                result.last_mut().unwrap().1 = end;
            } else {
                result.push((' ', end));
            }
        } else {
            result.extend(ch.to_lowercase().map(|lower| (lower, end)));
        }
    }
    result
}

pub struct Overlap {
    tail: Vec<char>,
    buffer: String,
    decided: bool,
}

impl Overlap {
    pub fn new(before: &str) -> Self {
        let text = normalized(before);
        Self {
            tail: text
                .iter()
                .skip(text.len().saturating_sub(MAX_TAIL))
                .map(|(ch, _)| *ch)
                .collect(),
            buffer: String::new(),
            decided: false,
        }
    }

    pub fn push(&mut self, text: &str, terminal: bool) -> String {
        if self.decided {
            return text.to_owned();
        }
        self.buffer.push_str(text);
        let normalized = normalized(&self.buffer);
        let generated: Vec<_> = normalized.iter().map(|(ch, _)| *ch).collect();
        let mut longest = 0;
        let mut pending = false;
        for start in 0..self.tail.len() {
            let suffix = &self.tail[start..];
            if suffix.len() < MIN_OVERLAP {
                continue;
            }
            if generated.starts_with(suffix) {
                longest = longest.max(suffix.len());
            } else if suffix.starts_with(&generated) {
                pending = true;
            }
        }
        if pending && !terminal && self.buffer.len() <= MAX_BUFFER {
            return String::new();
        }
        self.decided = true;
        let start = if longest == 0 {
            0
        } else {
            normalized[longest - 1].1
        };
        self.buffer.split_off(start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_only_complete_long_overlaps_across_fragments() {
        let mut overlap = Overlap::new("Before The lantern flickered at the window.");
        assert_eq!(overlap.push("the lantern", false), "");
        assert_eq!(
            overlap.push("  FLICKERED at the window. New", false),
            " New"
        );
        assert_eq!(overlap.push(" text", false), " text");
    }

    #[test]
    fn trims_character_chunks_with_split_whitespace() {
        let mut overlap = Overlap::new("The lantern flickered at the window.");
        let mut output = String::new();
        for ch in "THE lantern  \n flickered at the window. New".chars() {
            output.push_str(&overlap.push(&ch.to_string(), false));
        }
        output.push_str(&overlap.push("", true));
        assert_eq!(output, " New");
    }

    #[test]
    fn preserves_nonmatching_short_and_partial_output() {
        for (before, output) in [
            ("abcdefghijklmnop", "abcdefghijklX"),
            ("The lantern flickered.", "New text"),
            ("The lantern flickered.", "flickered."),
            ("The lantern flickered.", "The lantern"),
        ] {
            let mut overlap = Overlap::new(before);
            let first = overlap.push(output, false);
            assert_eq!(first + &overlap.push("", true), output);
        }
    }

    #[test]
    fn waits_for_longer_shared_prefix_before_deciding() {
        let mut overlap = Overlap::new("abcdefghijklmnop abcdefghijkl");
        assert_eq!(overlap.push("abcdefghijkl", false), "");
        assert_eq!(overlap.push("mnop!", false), "mnop!");
    }

    #[test]
    fn terminal_trims_complete_overlap_and_preserves_short_partial() {
        let mut overlap = Overlap::new("Before a repeated ending.");
        assert_eq!(overlap.push("a repeated ending.", false), "");
        assert_eq!(overlap.push("", true), "");
        let mut partial = Overlap::new("Before a repeated ending.");
        assert_eq!(partial.push("a repeated", false), "");
        assert_eq!(partial.push("", true), "a repeated");
    }

    #[test]
    fn bounded_buffer_flushes_unmatched_whitespace() {
        let mut overlap = Overlap::new("Some long matching tail");
        let input = " ".repeat(MAX_BUFFER + 1);
        assert_eq!(overlap.push(&input, false), input);
    }
}
