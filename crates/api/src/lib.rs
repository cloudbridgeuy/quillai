pub mod ai;
pub mod chapter_revisions;
pub mod database;
pub mod error;
pub mod eval;
pub mod manuscripts;
pub mod prelude;
pub mod settings;
pub mod users;

use axum::http::{header, HeaderValue, Method};
use axum::Router;
use sqlx::sqlite::SqlitePool;
use tower_http::cors::{Any, CorsLayer};

fn cors(origins: Vec<HeaderValue>) -> CorsLayer {
    let origins = if origins.is_empty() {
        vec![
            HeaderValue::from_static("http://localhost:3001"),
            HeaderValue::from_static("http://127.0.0.1:3001"),
        ]
    } else {
        origins
    };
    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::CONTENT_TYPE])
        .expose_headers(Any)
}

pub fn app(pool: SqlitePool) -> Router {
    app_with_cors(pool, Vec::new())
}

pub fn app_with_cors(pool: SqlitePool, origins: Vec<HeaderValue>) -> Router {
    let user_routes = users::routes(pool.clone());
    let (events, _) = tokio::sync::broadcast::channel::<manuscripts::sync_events::Change>(256);

    Router::new()
        .merge(user_routes)
        .merge(manuscripts::routes(pool.clone()).layer(axum::Extension(events)))
        .merge(settings::routes(pool.clone()))
        .merge(ai::routes(pool))
        .layer(cors(origins))
}
