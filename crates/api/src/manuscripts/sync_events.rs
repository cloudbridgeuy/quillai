use std::{collections::HashSet, convert::Infallible, time::Duration};

use axum::{
    extract::{rejection::QueryRejection, Path, Query, RawQuery, State},
    http::{header, HeaderValue},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::get,
    Extension, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::sync::broadcast;

use super::chapters::sync::chapter_piece;
use crate::error::Error;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub piece_id: i64,
    pub rev: i64,
}

pub type Sender = broadcast::Sender<Change>;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PieceIds {
    piece_ids: String,
}

pub fn routes(pool: SqlitePool) -> Router {
    Router::new()
        .route("/sync/events", get(events))
        .route(
            "/series/{id}/books/{book_id}/chapters/{chapter_id}/events",
            get(chapter_events),
        )
        .with_state(pool)
}

async fn events(
    State(pool): State<SqlitePool>,
    Extension(sender): Extension<Sender>,
    RawQuery(raw): RawQuery,
    query: Result<Query<PieceIds>, QueryRejection>,
) -> Result<Response, Error> {
    if raw.as_ref().is_none_or(|raw| raw.len() > 4096) {
        return Err(Error::Validation("invalid pieceIds".into()));
    }
    let Query(query) = query.map_err(|error| Error::Validation(error.body_text()))?;
    let mut seen = HashSet::new();
    let ids = query
        .piece_ids
        .split(',')
        .map(|id| {
            if id.is_empty() || !id.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(Error::Validation("invalid pieceIds".into()));
            }
            let id = id
                .parse::<i64>()
                .map_err(|_| Error::Validation("invalid pieceIds".into()))?;
            if !seen.insert(id) || seen.len() > 256 {
                return Err(Error::Validation("invalid pieceIds".into()));
            }
            Ok(id)
        })
        .collect::<Result<Vec<_>, _>>()?;
    stream(&pool, &sender, ids).await
}

async fn chapter_events(
    State(pool): State<SqlitePool>,
    Extension(sender): Extension<Sender>,
    Path(path): Path<(i64, i64, i64)>,
) -> Result<Response, Error> {
    let id = chapter_piece(&pool, path).await?;
    stream(&pool, &sender, vec![id]).await
}

async fn stream(pool: &SqlitePool, sender: &Sender, ids: Vec<i64>) -> Result<Response, Error> {
    for id in &ids {
        sqlx::query_scalar::<_, i64>(
            "SELECT id FROM active_pieces WHERE id = ? AND shape = 'leaf'",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(Error::NotFound)?;
    }
    let mut receiver = sender.subscribe();
    let selected: HashSet<_> = ids.iter().copied().collect();
    let stream = async_stream::stream! {
        yield Ok::<_, Infallible>(Event::default().event("ready").data(serde_json::json!({"pieceIds": ids}).to_string()).retry(Duration::from_secs(1)));
        loop {
            match receiver.recv().await {
                Ok(change) if selected.contains(&change.piece_id) => {
                    yield Ok(Event::default().event("changed").data(serde_json::to_string(&change).expect("serializing change")));
                }
                Ok(_) => {}
                Err(broadcast::error::RecvError::Lagged(_)) => yield Ok(Event::default().event("resync").data("{}")),
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    };
    let mut response = Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
        .into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    response
        .headers_mut()
        .insert("x-accel-buffering", HeaderValue::from_static("no"));
    Ok(response)
}
