pub mod handlers;
pub mod models;
pub(super) mod sync;

use axum::{routing::get, Router};
use sqlx::sqlite::SqlitePool;

pub fn routes(pool: SqlitePool) -> Router {
    Router::new()
        .route(
            "/series/{id}/books/{book_id}/chapters",
            get(handlers::list_chapters).post(handlers::create_chapter),
        )
        .route(
            "/series/{id}/books/{book_id}/chapters/{chapter_id}/restore",
            axum::routing::post(handlers::restore_chapter),
        )
        .route(
            "/series/{id}/books/{book_id}/chapters/{chapter_id}/deltas",
            get(sync::get_deltas).post(sync::post_delta),
        )
        .route(
            "/series/{id}/books/{book_id}/chapters/{chapter_id}",
            get(handlers::get_chapter)
                .put(handlers::update_chapter)
                .delete(handlers::delete_chapter),
        )
        .with_state(pool)
}
