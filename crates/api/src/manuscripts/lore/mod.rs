pub mod handlers;
pub mod models;
pub mod states;
pub mod store;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::sqlite::SqlitePool;

pub fn routes(pool: SqlitePool) -> Router {
    Router::new()
        .route(
            "/series/{id}/lore",
            get(handlers::list_lore).post(handlers::create_lore),
        )
        .route(
            "/series/{id}/lore/{lore_id}",
            get(handlers::get_lore)
                .put(handlers::update_lore)
                .delete(handlers::delete_lore),
        )
        .route(
            "/series/{id}/lore/{lore_id}/restore",
            post(handlers::restore_lore),
        )
        .with_state(pool)
}
