use quillai_delta::{Delta, Insert, Op};
use serde::{Deserialize, Serialize};

pub const FIXTURE_FILE: &str = "book.json";

#[derive(Debug, Deserialize)]
pub struct ApiChapter {
    pub id: i64,
    pub title: String,
    pub position: i64,
    pub kind: String,
    pub content: Delta,
    pub head_rev: i64,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Fixture {
    pub series_id: i64,
    pub book_id: i64,
    pub chapters: Vec<FixtureChapter>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct FixtureChapter {
    pub id: i64,
    pub title: String,
    pub position: i64,
    pub kind: String,
    pub head_rev: i64,
    pub content: Delta,
    pub text: String,
}

impl Fixture {
    pub fn build(series_id: i64, book_id: i64, chapters: Vec<ApiChapter>) -> Self {
        let chapters = chapters
            .into_iter()
            .map(|chapter| FixtureChapter {
                text: plain_text(&chapter.content),
                id: chapter.id,
                title: chapter.title,
                position: chapter.position,
                kind: chapter.kind,
                head_rev: chapter.head_rev,
                content: chapter.content,
            })
            .collect();
        Self {
            series_id,
            book_id,
            chapters,
        }
    }

    pub fn chapter(&self, id: i64) -> Option<&FixtureChapter> {
        self.chapters.iter().find(|chapter| chapter.id == id)
    }
}

pub fn plain_text(delta: &Delta) -> String {
    delta
        .ops
        .iter()
        .filter_map(|op| match op {
            Op::Insert {
                insert: Insert::Text(text),
                ..
            } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_joins_text_inserts_and_skips_embeds() {
        let delta = Delta::from_json(
            r#"{"ops":[{"insert":"One "},{"insert":{"image":"x"}},{"insert":"two\n","attributes":{"bold":true}}]}"#,
        )
        .unwrap();
        assert_eq!(plain_text(&delta), "One two\n");
    }
}
