use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};

pub async fn test_pool() -> SqlitePool {
    let options = SqliteConnectOptions::new()
        .filename(":memory:")
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .expect("failed to create in-memory SQLite pool");
    quillai_api::database::migrate(&pool)
        .await
        .expect("failed to run migrations");
    pool
}
