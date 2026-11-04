use axum::{
    extract::{rejection::QueryRejection, Query, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::error::Error;

#[derive(Deserialize)]
struct ActivityQuery {
    limit: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Activity {
    piece_id: i64,
    universe_id: i64,
    title: String,
    label: String,
    format: String,
    parent_id: Option<i64>,
    changed_at: String,
    chapter: Option<Chapter>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Chapter {
    series_id: i64,
    book_id: i64,
    chapter_id: i64,
}

fn limit(value: Option<&str>) -> Result<i64, Error> {
    match value {
        None => Ok(20),
        Some(value) => value
            .parse::<i64>()
            .ok()
            .filter(|n| (1..=50).contains(n))
            .ok_or_else(|| Error::Validation("limit must be an integer from 1 to 50".into())),
    }
}

fn timestamp(value: &str) -> Result<String, Error> {
    let (date, time) = value
        .split_once(' ')
        .ok_or_else(|| Error::Internal("invalid revision timestamp".into()))?;
    Ok(format!("{date}T{time}Z"))
}

async fn get_activity(
    State(pool): State<SqlitePool>,
    query: Result<Query<ActivityQuery>, QueryRejection>,
) -> Result<Json<Vec<Activity>>, Error> {
    let Query(query) = query.map_err(|error| Error::Validation(error.body_text()))?;
    let limit = limit(query.limit.as_deref())?;
    let rows = sqlx::query!(r#"SELECT p.id AS "piece_id!", p.universe_id, p.title, p.label, p.format, p.parent_id, datetime(r.created_at) AS "changed_at!", c.series_id, c.book_id, c.id AS chapter_id FROM active_pieces p JOIN piece_revisions r ON r.piece_id = p.id AND r.rev = (SELECT MAX(rev) FROM piece_revisions WHERE piece_id = p.id) LEFT JOIN legacy_chapters c ON c.piece_id = p.id WHERE p.shape = 'leaf' ORDER BY datetime(r.created_at) DESC, p.id DESC LIMIT ?"#, limit).fetch_all(&pool).await?;
    let activities = rows
        .into_iter()
        .map(|row| {
            Ok(Activity {
                piece_id: row.piece_id,
                universe_id: row.universe_id,
                title: row.title,
                label: row.label,
                format: row.format,
                parent_id: row.parent_id,
                changed_at: timestamp(&row.changed_at)?,
                chapter: row.series_id.zip(row.book_id).zip(row.chapter_id).map(
                    |((series_id, book_id), chapter_id)| Chapter {
                        series_id,
                        book_id,
                        chapter_id,
                    },
                ),
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(Json(activities))
}

pub fn routes(pool: SqlitePool) -> Router {
    Router::new()
        .route("/activity", get(get_activity))
        .with_state(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits() {
        assert_eq!(limit(None).unwrap(), 20);
        assert_eq!(limit(Some("1")).unwrap(), 1);
        assert_eq!(limit(Some("50")).unwrap(), 50);
        for value in ["0", "51", "-1", "", "bad"] {
            assert!(limit(Some(value)).is_err());
        }
    }

    #[test]
    fn formats_timestamp() {
        assert_eq!(
            timestamp("2026-11-04 17:00:00").unwrap(),
            "2026-11-04T17:00:00Z"
        );
        assert!(timestamp("invalid").is_err());
    }
}
