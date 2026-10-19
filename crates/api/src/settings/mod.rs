pub mod models;

use axum::{
    extract::{rejection::JsonRejection, State},
    routing::get,
    Json, Router,
};
use sqlx::SqlitePool;

use crate::error::Error;
use models::AuthorGuidance;

pub fn routes(pool: SqlitePool) -> Router {
    Router::new()
        .route(
            "/settings/author-guidance",
            get(get_guidance).put(put_guidance),
        )
        .with_state(pool)
}

pub async fn load_guidance(pool: &SqlitePool) -> Result<AuthorGuidance, Error> {
    let text = sqlx::query_scalar!("SELECT text FROM author_guidance WHERE id = 1")
        .fetch_optional(pool)
        .await?
        .unwrap_or_default();
    Ok(AuthorGuidance { text })
}

async fn get_guidance(State(pool): State<SqlitePool>) -> Result<Json<AuthorGuidance>, Error> {
    Ok(Json(load_guidance(&pool).await?))
}

async fn put_guidance(
    State(pool): State<SqlitePool>,
    payload: Result<Json<AuthorGuidance>, JsonRejection>,
) -> Result<Json<AuthorGuidance>, Error> {
    let Json(input) = payload.map_err(Error::from_json_rejection)?;
    input.validate()?;
    sqlx::query!(
        "INSERT INTO author_guidance (id, text) VALUES (1, ?) ON CONFLICT(id) DO UPDATE SET text = excluded.text",
        input.text
    )
    .execute(&pool)
    .await?;
    Ok(Json(input))
}
