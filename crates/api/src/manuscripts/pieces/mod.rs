pub mod handlers;
pub mod models;
pub mod moves;
pub mod scenes;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::sqlite::SqlitePool;

pub fn routes(pool: SqlitePool) -> Router {
    Router::new()
        .route(
            "/universes/{id}/pieces",
            get(handlers::list_loose_pieces).post(handlers::create_loose_piece),
        )
        .route(
            "/containers/{id}/pieces",
            get(handlers::list_container_pieces).post(handlers::create_container_piece),
        )
        .route("/pieces/{id}/children", post(handlers::create_child))
        .route("/pieces/{id}/placement", post(moves::place))
        .route("/pieces/{id}/split", post(moves::split))
        .route("/pieces/{id}/attach", post(moves::attach))
        .route(
            "/pieces/{id}",
            get(handlers::get_piece)
                .put(handlers::update_piece)
                .delete(handlers::delete_piece),
        )
        .route("/pieces/{id}/scenes", get(scenes::get_scenes))
        .route("/pieces/{id}/scenes/deltas", get(scenes::get_scene_deltas))
        .route(
            "/pieces/{id}/deltas",
            get(handlers::get_deltas).post(handlers::post_delta),
        )
        .with_state(pool)
}
