pub mod ai;
pub mod chapter_revisions;
pub mod database;
pub mod error;
pub mod manuscripts;
pub mod prelude;
pub mod settings;
pub mod users;

use axum::http::{header, HeaderValue, Method};
use axum::Router;
use sqlx::sqlite::SqlitePool;
use tower_http::cors::{Any, CorsLayer};

fn local_dev_cors() -> CorsLayer {
    CorsLayer::new()
        .allow_origin([
            HeaderValue::from_static("http://localhost:3001"),
            HeaderValue::from_static("http://127.0.0.1:3001"),
        ])
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
    let user_routes = users::routes(pool.clone());

    Router::new()
        .merge(user_routes)
        .merge(manuscripts::routes(pool.clone()))
        .merge(settings::routes(pool.clone()))
        .merge(ai::routes(pool))
        .layer(local_dev_cors())
}
