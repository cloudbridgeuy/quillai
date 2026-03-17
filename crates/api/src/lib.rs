pub mod error;
pub mod prelude;
pub mod users;

use axum::{extract::State, response::Html, routing::get, Router};
use sqlx::sqlite::SqlitePool;

use crate::error::Error;

pub fn app(pool: SqlitePool) -> Router {
    Router::new().route("/", get(handler)).with_state(pool)
}

async fn handler(State(pool): State<SqlitePool>) -> std::result::Result<Html<String>, Error> {
    let value: String = sqlx::query_scalar!("SELECT 'hello world from sqlite'")
        .fetch_one(&pool)
        .await?;

    Ok(Html(format!("<h1>Hello, {}!</h1>", value)))
}
