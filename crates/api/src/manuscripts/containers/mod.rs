pub mod handlers;
pub mod models;

use axum::{routing::get, Router};
use sqlx::sqlite::SqlitePool;

pub fn routes(pool: SqlitePool) -> Router {
    Router::new()
        .route(
            "/universes/{id}/containers",
            get(handlers::list_containers).post(handlers::create_container),
        )
        .route(
            "/containers/{id}",
            get(handlers::get_container)
                .put(handlers::update_container)
                .delete(handlers::delete_container),
        )
        .with_state(pool)
}
