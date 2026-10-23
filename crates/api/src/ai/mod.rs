pub mod catalog;
pub mod handlers;
pub mod lore;
pub mod models;
pub mod prompt;
pub mod provider;
pub mod replacement;
pub mod section_overrides;
pub mod segments;
pub mod tagged_text;

use axum::{
    routing::{get, post},
    Router,
};

pub fn routes(pool: sqlx::SqlitePool) -> Router {
    Router::new()
        .route("/ai/models", get(handlers::models))
        .route("/ai/generate", post(handlers::generate))
        .with_state(pool)
}
