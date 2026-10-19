pub mod handlers;
pub mod models;

use axum::{routing::get, Router};
use sqlx::sqlite::SqlitePool;

pub fn routes(pool: SqlitePool) -> Router {
    Router::new()
        .route(
            "/series/{id}/books",
            get(handlers::list_books).post(handlers::create_book),
        )
        .route(
            "/series/{id}/books/{book_id}",
            get(handlers::get_book)
                .put(handlers::update_book)
                .delete(handlers::delete_book),
        )
        .with_state(pool)
}
