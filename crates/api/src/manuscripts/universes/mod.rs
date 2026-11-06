pub mod handlers;

use axum::{
    routing::{get, post, put},
    Router,
};
use sqlx::sqlite::SqlitePool;

use crate::manuscripts::lore::{images, states};

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
        .route(
            "/universes/{id}/lore/{lore_id}/states",
            get(states::list_states).post(states::create_state),
        )
        .route(
            "/universes/{id}/lore/{lore_id}/states/{state_id}",
            put(states::update_state).delete(states::delete_state),
        )
        .route(
            "/universes/{id}/lore/{lore_id}/states/{state_id}/restore",
            post(states::restore_state),
        )
        .route(
            "/universes/{id}/lore/{lore_id}/images",
            get(images::list_images).post(images::create_image),
        )
        .route(
            "/universes/{id}/lore/{lore_id}/images/{image_id}",
            get(images::get_image),
        )
        .route("/universes/{id}/restore", post(handlers::restore_universe))
        .with_state(pool)
}
