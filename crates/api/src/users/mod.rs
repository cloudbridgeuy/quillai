pub mod handlers;
pub mod models;

use axum::{routing::get, Router};
use sqlx::sqlite::SqlitePool;

pub fn routes(pool: SqlitePool) -> Router {
    Router::new()
        .route("/users", get(handlers::list_users).post(handlers::create_user))
        .route("/users/{id}", get(handlers::get_user).delete(handlers::delete_user))
        .with_state(pool)
}
