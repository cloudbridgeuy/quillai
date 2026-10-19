mod common;

#[tokio::test]
async fn migrations_preserve_data_on_rerun_and_test_pools_are_isolated() {
    let pool = common::test_pool().await;
    let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(foreign_keys, 1);

    sqlx::query("INSERT INTO users (name, email) VALUES ('Alice', 'alice@example.com')")
        .execute(&pool)
        .await
        .unwrap();
    quillai_api::database::migrate(&pool).await.unwrap();

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);

    let isolated_pool = common::test_pool().await;
    let isolated_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&isolated_pool)
        .await
        .unwrap();
    assert_eq!(isolated_count, 0);
}

#[tokio::test]
async fn migrate_baselines_legacy_database_without_sqlx_bookkeeping() {
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(":memory:")
        .foreign_keys(true);
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    sqlx::raw_sql(include_str!(
        "../../../migrations/20250407204144_init.up.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO users (name, email) VALUES ('Legacy', 'legacy@example.com')")
        .execute(&pool)
        .await
        .unwrap();

    quillai_api::database::migrate(&pool).await.unwrap();
    quillai_api::database::migrate(&pool).await.unwrap();

    let users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(users, 1);
    let applied: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations WHERE success = TRUE")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        applied,
        quillai_api::database::MIGRATOR
            .iter()
            .filter(|m| m.migration_type.is_up_migration())
            .count() as i64
    );
}
