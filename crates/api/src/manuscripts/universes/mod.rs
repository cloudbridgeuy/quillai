pub mod handlers;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::sqlite::SqlitePool;

pub fn routes(pool: SqlitePool) -> Router {
    Router::new()
        .route(
            "/universes",
            get(handlers::list_universes).post(handlers::create_universe),
        )
        .route(
            "/universes/{id}",
            get(handlers::get_universe)
                .put(handlers::update_universe)
                .delete(handlers::delete_universe),
        )
        .route(
            "/universes/{id}/lore",
            get(handlers::list_lore).post(handlers::create_lore),
        )
        .route(
            "/universes/{id}/lore/{lore_id}",
            get(handlers::get_lore)
                .put(handlers::update_lore)
                .delete(handlers::delete_lore),
        )
        .route(
            "/universes/{id}/lore/{lore_id}/restore",
            post(handlers::restore_lore),
        )
        .with_state(pool)
}
