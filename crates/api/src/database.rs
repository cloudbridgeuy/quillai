use sqlx::{
    migrate::{Migrate, MigrateError, Migrator},
    SqlitePool,
};

pub static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");

const LEGACY_INIT_VERSION: i64 = 20250407204144;

pub async fn migrate(pool: &SqlitePool) -> Result<(), MigrateError> {
    baseline_legacy_schema(pool).await?;
    MIGRATOR.run(pool).await
}

async fn table_exists(pool: &SqlitePool, name: &str) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
    )
    .bind(name)
    .fetch_one(pool)
    .await
    .map(|count| count > 0)
}

async fn baseline_legacy_schema(pool: &SqlitePool) -> Result<(), MigrateError> {
    if table_exists(pool, "_sqlx_migrations").await? || !table_exists(pool, "users").await? {
        return Ok(());
    }
    let Some(init) = MIGRATOR
        .iter()
        .find(|m| m.version == LEGACY_INIT_VERSION && m.migration_type.is_up_migration())
    else {
        return Ok(());
    };
    let mut conn = pool.acquire().await?;
    conn.ensure_migrations_table().await?;
    sqlx::query(
        "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time) VALUES (?1, ?2, TRUE, ?3, -1)",
    )
    .bind(init.version)
    .bind(&*init.description)
    .bind(&*init.checksum)
    .execute(&mut *conn)
    .await?;
    Ok(())
}
