mod common;

#[tokio::test]
async fn entries_round_trip_with_defaults_and_all_kinds_and_levels() {
    let pool = common::test_pool().await;
    sqlx::raw_sql(
        "INSERT INTO series (id, title, position) VALUES (1, 'Series', 0);
         INSERT INTO lore_entries (series_id, kind, name) VALUES (1, 'character', 'Alice');",
    )
    .execute(&pool)
    .await
    .unwrap();
    let entry: (i64, i64, String, String, String, String, String, i64, String) = sqlx::query_as(
        "SELECT id, series_id, kind, name, keywords, visible_text, hidden_notes, pinned, level FROM lore_entries",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        entry,
        (
            1,
            1,
            "character".into(),
            "Alice".into(),
            "[]".into(),
            "".into(),
            "".into(),
            0,
            "include".into()
        )
    );

    let keywords = serde_json::json!(["Al", "旅人"]);
    sqlx::query(
        "UPDATE lore_entries SET keywords = ?, visible_text = 'A traveler', hidden_notes = 'Secret heir', pinned = 1",
    )
    .bind(keywords.to_string())
    .execute(&pool)
    .await
    .unwrap();
    let stored: (String, String, String, i64) =
        sqlx::query_as("SELECT keywords, visible_text, hidden_notes, pinned FROM lore_entries")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&stored.0).unwrap(),
        keywords
    );
    assert_eq!(stored.1, "A traveler");
    assert_eq!(stored.2, "Secret heir");
    assert_eq!(stored.3, 1);

    for kind in ["character", "place", "term", "note"] {
        for level in ["include", "known_dont_reveal", "leave_out"] {
            sqlx::query("UPDATE lore_entries SET kind = ?, level = ?")
                .bind(kind)
                .bind(level)
                .execute(&pool)
                .await
                .unwrap();
        }
    }
}

#[tokio::test]
async fn invalid_kinds_levels_names_keywords_and_pinned_flags_are_rejected() {
    let pool = common::test_pool().await;
    sqlx::raw_sql(
        "INSERT INTO series (id, title, position) VALUES (1, 'Series', 0);
         INSERT INTO lore_entries (series_id, kind, name) VALUES (1, 'character', 'Alice');",
    )
    .execute(&pool)
    .await
    .unwrap();
    for statement in [
        "UPDATE lore_entries SET kind = 'unknown'",
        "UPDATE lore_entries SET level = 'unknown'",
        "UPDATE lore_entries SET name = ''",
        "UPDATE lore_entries SET name = '   '",
        "UPDATE lore_entries SET name = char(9, 10, 11, 12, 13, 32)",
        "UPDATE lore_entries SET keywords = 'not json'",
        "UPDATE lore_entries SET keywords = '{}'",
        "UPDATE lore_entries SET keywords = 'null'",
        "UPDATE lore_entries SET pinned = -1",
        "UPDATE lore_entries SET pinned = 2",
    ] {
        let error = sqlx::query(statement).execute(&pool).await.unwrap_err();
        assert!(error.as_database_error().unwrap().is_check_violation());
    }
    for column in [
        "series_id",
        "kind",
        "name",
        "keywords",
        "visible_text",
        "hidden_notes",
        "pinned",
        "level",
    ] {
        let error = sqlx::query(&format!("UPDATE lore_entries SET {column} = NULL"))
            .execute(&pool)
            .await
            .unwrap_err();
        assert_eq!(
            error.as_database_error().unwrap().kind(),
            sqlx::error::ErrorKind::NotNullViolation
        );
    }
}

#[tokio::test]
async fn missing_series_is_rejected_and_deleting_series_cascades_only_to_its_entries() {
    let pool = common::test_pool().await;
    let error = sqlx::query(
        "INSERT INTO lore_entries (series_id, kind, name) VALUES (999, 'character', 'Orphan')",
    )
    .execute(&pool)
    .await
    .unwrap_err();
    assert!(error
        .as_database_error()
        .unwrap()
        .is_foreign_key_violation());

    sqlx::raw_sql(
        "INSERT INTO series (id, title, position) VALUES (1, 'First', 0), (2, 'Second', 1);
         INSERT INTO lore_entries (series_id, kind, name)
         VALUES (1, 'character', 'Alice'), (1, 'place', 'Home'), (2, 'term', 'Magic');
         DELETE FROM series WHERE id = 1;",
    )
    .execute(&pool)
    .await
    .unwrap();
    let remaining: Vec<(i64, String)> = sqlx::query_as("SELECT series_id, name FROM lore_entries")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(remaining, [(2, "Magic".into())]);
}

#[tokio::test]
async fn down_migration_preserves_other_tables_and_can_be_reapplied() {
    let pool = common::test_pool().await;
    sqlx::raw_sql(
        "INSERT INTO users (name, email) VALUES ('Alice', 'alice@example.com');
         INSERT INTO series (id, title, position) VALUES (1, 'Series', 0);
         INSERT INTO books (id, series_id, title, position) VALUES (1, 1, 'Book', 0);
         INSERT INTO chapters (id, book_id, title, position, kind) VALUES (1, 1, 'Chapter', 0, 'chapter');
         INSERT INTO lore_entries (series_id, kind, name) VALUES (1, 'character', 'Alice');",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::migrate!("../../migrations")
        .undo(&pool, 20261019000002)
        .await
        .unwrap();
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'lore_entries'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 0);
    for table in ["users", "series", "books", "chapters"] {
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }
    quillai_api::database::migrate(&pool).await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM lore_entries")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    sqlx::query("INSERT INTO lore_entries (series_id, kind, name) VALUES (1, 'note', 'Restored')")
        .execute(&pool)
        .await
        .unwrap();
}
