pub mod handlers;
pub mod models;
pub mod prompt;
pub mod provider;
pub mod section_overrides;
pub mod segments;
pub mod tagged_text;

use axum::{routing::post, Router};

pub fn routes() -> Router {
    Router::new().route("/ai/generate", post(handlers::generate))
}
