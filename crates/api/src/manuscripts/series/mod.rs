pub mod handlers;
pub mod models;

use axum::{routing::get, Router};
use sqlx::sqlite::SqlitePool;

pub fn routes(pool: SqlitePool) -> Router {
    Router::new()
        .route(
            "/series",
            get(handlers::list_series).post(handlers::create_series),
        )
        .route(
            "/series/{id}",
            get(handlers::get_series)
                .put(handlers::update_series)
                .delete(handlers::delete_series),
        )
        .with_state(pool)
}
