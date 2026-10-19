pub mod books;
pub mod series;

use axum::Router;
use sqlx::sqlite::SqlitePool;

pub fn routes(pool: SqlitePool) -> Router {
    series::routes(pool.clone()).merge(books::routes(pool))
}
