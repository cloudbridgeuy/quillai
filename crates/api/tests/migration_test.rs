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
