mod common;

#[tokio::test]
async fn deleting_a_book_or_series_cascades_only_to_its_descendants() {
    let pool = common::test_pool().await;
    sqlx::raw_sql(
        "INSERT INTO series (id, title, position) VALUES (1, 'First', 0), (2, 'Second', 1);
         INSERT INTO books (id, series_id, title, position)
         VALUES (1, 1, 'One', 0), (2, 1, 'Two', 1), (3, 2, 'Three', 0);
         INSERT INTO chapters (book_id, title, position, kind)
         VALUES (1, 'One', 0, 'chapter'), (2, 'Two', 0, 'chapter'), (3, 'Three', 0, 'chapter');
         DELETE FROM books WHERE id = 1;",
    )
    .execute(&pool)
    .await
    .unwrap();
    let remaining: Vec<i64> = sqlx::query_scalar("SELECT book_id FROM chapters ORDER BY book_id")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(remaining, [2, 3]);

    sqlx::query("DELETE FROM series WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();
    let books: Vec<i64> = sqlx::query_scalar("SELECT id FROM books")
        .fetch_all(&pool)
        .await
        .unwrap();
    let chapters: Vec<i64> = sqlx::query_scalar("SELECT book_id FROM chapters")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(books, [3]);
    assert_eq!(chapters, [3]);

    for statement in [
        "INSERT INTO books (series_id, title, position) VALUES (999, 'Orphan', 0)",
        "INSERT INTO chapters (book_id, title, position, kind) VALUES (999, 'Orphan', 0, 'chapter')",
    ] {
        let error = sqlx::query(statement).execute(&pool).await.unwrap_err();
        assert!(error.as_database_error().unwrap().is_foreign_key_violation());
    }
}

#[tokio::test]
async fn positions_kinds_and_delta_json_round_trip() {
    let pool = common::test_pool().await;
    sqlx::raw_sql(
        "INSERT INTO series (id, title, position) VALUES (1, 'Later', 20), (2, 'Earlier', 10);
         INSERT INTO books (id, series_id, title, position)
         VALUES (1, 1, 'Later', 20), (2, 1, 'Earlier', 10);
         INSERT INTO chapters (book_id, title, position, kind)
         VALUES (1, 'End', 30, 'epilogue'), (1, 'Start', 10, 'prologue'), (1, 'Middle', 20, 'chapter');",
    )
    .execute(&pool)
    .await
    .unwrap();

    for statement in [
        "SELECT title FROM series ORDER BY position",
        "SELECT title FROM books WHERE series_id = 1 ORDER BY position",
    ] {
        let titles: Vec<String> = sqlx::query_scalar(statement)
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(titles, ["Earlier", "Later"]);
    }
    let chapters: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT title, kind, content FROM chapters WHERE book_id = 1 ORDER BY position",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        chapters,
        [
            ("Start".into(), "prologue".into(), "{\"ops\":[]}".into()),
            ("Middle".into(), "chapter".into(), "{\"ops\":[]}".into()),
            ("End".into(), "epilogue".into(), "{\"ops\":[]}".into()),
        ]
    );

    let delta =
        serde_json::json!({"ops": [{"insert": "Hello 🌍\n", "attributes": {"bold": true}}]});
    sqlx::query("UPDATE chapters SET content = ? WHERE kind = 'chapter'")
        .bind(delta.to_string())
        .execute(&pool)
        .await
        .unwrap();
    let stored: String = sqlx::query_scalar("SELECT content FROM chapters WHERE kind = 'chapter'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&stored).unwrap(),
        delta
    );

    for statement in [
        "UPDATE chapters SET kind = 'appendix'",
        "UPDATE chapters SET content = 'not json'",
    ] {
        let error = sqlx::query(statement).execute(&pool).await.unwrap_err();
        assert!(error.as_database_error().unwrap().is_check_violation());
    }
}

#[tokio::test]
async fn down_migration_preserves_users_and_can_be_reapplied() {
    let pool = common::test_pool().await;
    sqlx::raw_sql(
        "INSERT INTO users (name, email) VALUES ('Alice', 'alice@example.com');
         INSERT INTO series (title, position) VALUES ('Series', 0);
         INSERT INTO books (series_id, title, position) VALUES (1, 'Book', 0);
         INSERT INTO chapters (book_id, title, position, kind) VALUES (1, 'Chapter', 0, 'chapter');",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::migrate!("../../migrations")
        .undo(&pool, 20250407204144)
        .await
        .unwrap();
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('series', 'books', 'chapters')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 0);
    let name: String = sqlx::query_scalar("SELECT name FROM users")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(name, "Alice");
    quillai_api::database::migrate(&pool).await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chapters")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}
