pub mod error;
pub mod prelude;
pub mod users;

use axum::Router;
use sqlx::sqlite::SqlitePool;

pub fn app(pool: SqlitePool) -> Router {
    Router::new().merge(users::routes(pool))
}
