pub mod books;
pub mod chapters;
pub mod leaf_sync;
pub mod lore;
pub mod series;

use axum::Router;
use sqlx::sqlite::SqlitePool;

pub fn routes(pool: SqlitePool) -> Router {
    series::routes(pool.clone())
        .merge(books::routes(pool.clone()))
        .merge(lore::routes(pool.clone()))
        .merge(chapters::routes(pool))
}
