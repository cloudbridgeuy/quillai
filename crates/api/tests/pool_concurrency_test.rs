use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use clap::Parser;
use sqlx::sqlite::SqliteConnectOptions;
use std::{
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tower::ServiceExt;

#[path = "../src/cli.rs"]
mod cli;

fn database_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "quillai-pool-{}-{}.db",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

async fn pool(max_connections: u32, acquire_timeout: Duration) -> (sqlx::SqlitePool, PathBuf) {
    let path = database_path();
    let options = SqliteConnectOptions::new()
        .filename(&path)
        .create_if_missing(true);
    let pool = quillai_api::database::connect(options, max_connections, acquire_timeout)
        .await
        .unwrap();
    quillai_api::database::migrate(&pool).await.unwrap();
    (pool, path)
}

#[tokio::test]
async fn activity_succeeds_while_another_connection_is_held() {
    let args = cli::App::parse_from(["quillai-api"]);
    let (pool, path) = pool(
        args.db_max_connections,
        Duration::from_secs(args.db_acquire_timeout),
    )
    .await;
    let connection = pool.acquire().await.unwrap();
    let response = quillai_api::app(pool.clone())
        .oneshot(
            Request::builder()
                .uri("/activity")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    drop(connection);
    pool.close().await;
    std::fs::remove_file(path).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn write_succeeds_while_read_transaction_is_open() {
    let (pool, path) = pool(2, Duration::from_secs(3)).await;
    let mut read = pool.begin().await.unwrap();
    sqlx::query("SELECT * FROM users")
        .fetch_all(&mut *read)
        .await
        .unwrap();
    let mut write = pool.acquire().await.unwrap();
    let journal: String = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(&mut *write)
        .await
        .unwrap();
    let timeout: i64 = sqlx::query_scalar("PRAGMA busy_timeout")
        .fetch_one(&mut *write)
        .await
        .unwrap();
    let result =
        sqlx::query("INSERT INTO users (name, email) VALUES ('Alice', 'alice@example.com')")
            .execute(&mut *write)
            .await;
    read.rollback().await.unwrap();
    drop(write);
    pool.close().await;
    std::fs::remove_file(path).unwrap();
    result.unwrap();
    assert_eq!(journal, "wal");
    assert_eq!(timeout, 5000);
}
