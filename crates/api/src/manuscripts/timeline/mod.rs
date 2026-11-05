pub mod handlers;
pub mod models;
pub mod store;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::sqlite::SqlitePool;

pub fn routes(pool: SqlitePool) -> Router {
    Router::new()
        .route(
            "/universes/{id}/timeline",
            get(handlers::list_points).post(handlers::create_point),
        )
        .route(
            "/timeline/{id}",
            axum::routing::put(handlers::update_point).delete(handlers::delete_point),
        )
        .route("/timeline/{id}/restore", post(handlers::restore_point))
        .with_state(pool)
}
