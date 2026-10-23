pub const OPEN_TAG: &str = "<replacement>";
pub const CLOSE_TAG: &str = "</replacement>";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplacementExtractor {
    BeforeOpen { partial_tag: String },
    Inside { held_back: String },
    Closed,
}

impl Default for ReplacementExtractor {
    fn default() -> Self {
        Self::BeforeOpen {
            partial_tag: String::new(),
        }
    }
}

impl ReplacementExtractor {
    pub fn push(&mut self, chunk: &str) -> String {
        match self {
            Self::BeforeOpen { partial_tag } => {
                let buffered = format!("{partial_tag}{chunk}");
                match buffered.find(OPEN_TAG) {
                    Some(start) => {
                        *self = Self::Inside {
                            held_back: String::new(),
                        };
                        self.push(&buffered[start + OPEN_TAG.len()..])
                    }
                    None => {
                        *partial_tag = tag_prefix_suffix(&buffered, OPEN_TAG).to_string();
                        String::new()
                    }
                }
            }
            Self::Inside { held_back } => {
                let buffered = format!("{held_back}{chunk}");
                match buffered.find(CLOSE_TAG) {
                    Some(end) => {
                        *self = Self::Closed;
                        buffered[..end].to_string()
                    }
                    None => {
                        let kept = tag_prefix_suffix(&buffered, CLOSE_TAG);
                        let emitted = buffered[..buffered.len() - kept.len()].to_string();
                        *held_back = kept.to_string();
                        emitted
                    }
                }
            }
            Self::Closed => String::new(),
        }
    }

    pub fn finish(&self) -> Result<(), String> {
        match self {
            Self::Closed => Ok(()),
            Self::BeforeOpen { .. } => {
                Err("AI edit response did not include a <replacement> tag.".into())
            }
            Self::Inside { .. } => {
                Err("AI edit response ended before the closing </replacement> tag.".into())
            }
        }
    }
}

fn tag_prefix_suffix<'a>(text: &'a str, tag: &str) -> &'a str {
    (1..tag.len())
        .rev()
        .find(|&length| text.ends_with(&tag[..length]))
        .map_or("", |length| &text[text.len() - length..])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extract(chunks: &[&str]) -> (String, Result<(), String>) {
        let mut extractor = ReplacementExtractor::default();
        let text = chunks.iter().map(|chunk| extractor.push(chunk)).collect();
        (text, extractor.finish())
    }

    fn split_at_every_byte(response: &str) -> Vec<(String, String)> {
        (0..=response.len())
            .filter(|&index| response.is_char_boundary(index))
            .map(|index| (response[..index].to_string(), response[index..].to_string()))
            .collect()
    }

    #[test]
    fn keeps_only_text_inside_the_tags() {
        assert_eq!(
            extract(&["Sure! <replacement>He tried.</replacement> This fixes the agreement."]),
            ("He tried.".into(), Ok(()))
        );
    }

    #[test]
    fn open_and_close_tags_split_at_every_byte_across_two_chunks() {
        let response =
            "Note: <replacement>Él intentó 😀 <b> distraerse.</replacement>\n\nA new paragraph.";
        for (first, second) in split_at_every_byte(response) {
            assert_eq!(
                extract(&[&first, &second]),
                ("Él intentó 😀 <b> distraerse.".into(), Ok(())),
                "split {first:?} | {second:?}"
            );
        }
    }

    #[test]
    fn one_character_chunks_stream_the_replacement() {
        let response = "x<replacement>a</replacemenb</replacement>y";
        let chars: Vec<String> = response.chars().map(String::from).collect();
        let chunks: Vec<&str> = chars.iter().map(String::as_str).collect();
        assert_eq!(extract(&chunks), ("a</replacemenb".into(), Ok(())));
    }

    #[test]
    fn emits_inside_text_before_the_close_tag_arrives() {
        let mut extractor = ReplacementExtractor::default();
        assert_eq!(extractor.push("intro <replace"), "");
        assert_eq!(extractor.push("ment>He tried </rep"), "He tried ");
        assert_eq!(extractor.push("ly hard"), "</reply hard");
        assert_eq!(extractor.push("</replacement> more"), "");
        assert_eq!(extractor.push("<replacement>again</replacement>"), "");
        assert_eq!(extractor.finish(), Ok(()));
    }

    #[test]
    fn missing_open_tag_is_an_error() {
        let (text, result) = extract(&["He tried. This fixes the agreement."]);
        assert_eq!(text, "");
        assert!(result.unwrap_err().contains("did not include"));
    }

    #[test]
    fn unclosed_tag_is_an_error_after_streaming_inside_text() {
        let (text, result) = extract(&["<replacement>He tried", " hard</replace"]);
        assert_eq!(text, "He tried hard");
        assert!(result.unwrap_err().contains("closing"));
    }

    #[test]
    fn empty_replacement_is_accepted() {
        assert_eq!(
            extract(&["<replacement></replacement>"]),
            (String::new(), Ok(()))
        );
    }
}
