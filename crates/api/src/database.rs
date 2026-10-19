use sqlx::{migrate::MigrateError, SqlitePool};

pub async fn migrate(pool: &SqlitePool) -> Result<(), MigrateError> {
    sqlx::migrate!("../../migrations").run(pool).await
}
