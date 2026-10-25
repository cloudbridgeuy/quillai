mod common;

use quillai_api::database::MIGRATOR;
use sqlx::{migrate::MigrateError, SqlitePool};

const BEFORE_STORY_MODEL: i64 = 20261020000000;

const OLD_TABLES: [(&str, &str); 6] = [
    ("series", "SELECT json_array(id, title, position) FROM series ORDER BY id"),
    ("books", "SELECT json_array(id, series_id, title, position) FROM books ORDER BY id"),
    (
        "chapters",
        "SELECT json_array(id, book_id, title, position, kind, content, head_rev, snapshot_rev) FROM chapters ORDER BY id",
    ),
    (
        "chapter_revisions",
        "SELECT json_array(chapter_id, rev, delta, created_at) FROM chapter_revisions ORDER BY chapter_id, rev",
    ),
    (
        "lore_entries",
        "SELECT json_array(id, series_id, kind, name, keywords, aliases, visible_text, hidden_notes, pinned, level) FROM lore_entries ORDER BY id",
    ),
    (
        "sqlite_sequence",
        "SELECT json_array(name, seq) FROM sqlite_sequence WHERE name IN ('series', 'books', 'chapters', 'lore_entries') ORDER BY name",
    ),
];

const NEW_TABLES: [(&str, &str); 7] = [
    ("universes", "SELECT json_array(id, title, position) FROM universes ORDER BY id"),
    (
        "containers",
        "SELECT json_array(id, universe_id, title, position, kind) FROM containers ORDER BY id",
    ),
    (
        "pieces",
        "SELECT json_array(id, universe_id, container_id, parent_id, title, position, label, format, shape, content, head_rev, snapshot_rev, legacy_book_id, legacy_chapter_id) FROM pieces ORDER BY id",
    ),
    (
        "piece_revisions",
        "SELECT json_array(piece_id, rev, delta, created_at) FROM piece_revisions ORDER BY piece_id, rev",
    ),
    (
        "lore_entries",
        "SELECT json_array(id, universe_id, kind, name, keywords, aliases, visible_text, hidden_notes, pinned, level) FROM lore_entries ORDER BY id",
    ),
    (
        "sqlite_sequence",
        "SELECT json_array(name, seq) FROM sqlite_sequence WHERE name IN ('universes', 'containers', 'pieces', 'lore_entries') ORDER BY name",
    ),
    (
        "legacy_ids",
        "SELECT json_array(kind, seq) FROM legacy_ids ORDER BY kind",
    ),
];

const SEED: &str = r#"
INSERT INTO series (id, title, position) VALUES (1, 'Saga', 0), (2, 'Duology', 1), (3, 'Deleted', 2);
DELETE FROM series WHERE id = 3;
INSERT INTO books (id, series_id, title, position) VALUES (1, 1, 'First', 0), (2, 1, 'Empty', 1), (3, 2, 'Other', 0);
INSERT INTO chapters (id, book_id, title, position, kind, content, head_rev, snapshot_rev) VALUES
    (1, 1, 'Before', 0, 'prologue', '{"ops":[{"insert":"Once\n"}]}', 2, 0),
    (2, 1, 'One', 1, 'chapter', '{"ops":[]}', 0, 0),
    (3, 3, 'After', 0, 'epilogue', '{"ops":[{"insert":"End\n"}]}', 1, 1);
INSERT INTO chapter_revisions (chapter_id, rev, delta, created_at) VALUES
    (1, 1, '{"ops":[{"insert":"A"}]}', '2026-10-01 10:00:00'),
    (1, 2, '{"ops":[{"retain":1},{"insert":"B"}]}', '2026-10-01 10:01:00'),
    (3, 1, '{"ops":[{"insert":"End\n"}]}', '2026-10-02 09:00:00');
INSERT INTO lore_entries (id, series_id, kind, name, keywords, aliases, visible_text, hidden_notes, pinned, level) VALUES
    (1, 1, 'character', 'Alice', '["heir"]', '["Al"]', 'Visible', 'Hidden', 1, 'known_dont_reveal'),
    (2, 2, 'place', 'Harbor', '[]', '[]', '', '', 0, 'include'),
    (4, 2, 'note', 'Gone', '[]', '[]', '', '', 0, 'leave_out');
DELETE FROM lore_entries WHERE id = 4;
"#;

async fn up(pool: &SqlitePool) -> Result<(), MigrateError> {
    MIGRATOR.run(pool).await
}

async fn down(pool: &SqlitePool) -> Result<(), MigrateError> {
    MIGRATOR.undo(pool, BEFORE_STORY_MODEL).await
}

async fn dump(pool: &SqlitePool, tables: &[(&str, &str)]) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    for (name, query) in tables {
        let rows: Vec<String> = sqlx::query_scalar(query).fetch_all(pool).await.unwrap();
        out.push((name.to_string(), rows));
    }
    out
}

async fn table_names(pool: &SqlitePool) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT IN ('sqlite_sequence', '_sqlx_migrations') ORDER BY name",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

async fn seeded_pool() -> SqlitePool {
    let pool = common::test_pool().await;
    down(&pool).await.unwrap();
    sqlx::raw_sql(SEED).execute(&pool).await.unwrap();
    pool
}

fn rows(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[tokio::test]
async fn up_maps_series_books_chapters_lore_and_revisions() {
    let pool = seeded_pool().await;
    up(&pool).await.unwrap();

    assert_eq!(
        table_names(&pool).await,
        rows(&[
            "author_guidance",
            "containers",
            "legacy_ids",
            "lore_entries",
            "piece_revisions",
            "pieces",
            "universes",
            "users"
        ])
    );
    let expected = vec![
        (
            "universes",
            rows(&[r#"[1,"Saga",0]"#, r#"[2,"Duology",1]"#]),
        ),
        (
            "containers",
            rows(&[
                r#"[1,1,"Saga",0,"book_series"]"#,
                r#"[2,2,"Duology",1,"book_series"]"#,
            ]),
        ),
        (
            "pieces",
            rows(&[
                r#"[1,1,1,null,"First",0,"novel","prose","group",null,null,null,1,null]"#,
                r#"[2,1,1,null,"Empty",1,"novel","prose","group",null,null,null,2,null]"#,
                r#"[3,2,2,null,"Other",0,"novel","prose","group",null,null,null,3,null]"#,
                r#"[4,1,null,1,"Before",0,"prologue","prose","leaf","{\"ops\":[{\"insert\":\"Once\\n\"}]}",2,0,null,1]"#,
                r#"[5,1,null,1,"One",1,"chapter","prose","leaf","{\"ops\":[]}",0,0,null,2]"#,
                r#"[6,2,null,3,"After",0,"epilogue","prose","leaf","{\"ops\":[{\"insert\":\"End\\n\"}]}",1,1,null,3]"#,
            ]),
        ),
        (
            "piece_revisions",
            rows(&[
                r#"[4,1,"{\"ops\":[{\"insert\":\"A\"}]}","2026-10-01 10:00:00"]"#,
                r#"[4,2,"{\"ops\":[{\"retain\":1},{\"insert\":\"B\"}]}","2026-10-01 10:01:00"]"#,
                r#"[6,1,"{\"ops\":[{\"insert\":\"End\\n\"}]}","2026-10-02 09:00:00"]"#,
            ]),
        ),
        (
            "lore_entries",
            rows(&[
                r#"[1,1,"character","Alice","[\"heir\"]","[\"Al\"]","Visible","Hidden",1,"known_dont_reveal"]"#,
                r#"[2,2,"place","Harbor","[]","[]","","",0,"include"]"#,
            ]),
        ),
        (
            "sqlite_sequence",
            rows(&[
                r#"["containers",3]"#,
                r#"["lore_entries",4]"#,
                r#"["pieces",6]"#,
                r#"["universes",2]"#,
            ]),
        ),
        ("legacy_ids", rows(&[r#"["book",3]"#, r#"["chapter",3]"#])),
    ];
    let expected: Vec<(String, Vec<String>)> = expected
        .into_iter()
        .map(|(name, values)| (name.to_string(), values))
        .collect();
    assert_eq!(dump(&pool, &NEW_TABLES).await, expected);
}

#[tokio::test]
async fn down_restores_exact_old_rows_and_older_downs_still_run() {
    let pool = seeded_pool().await;
    let before = dump(&pool, &OLD_TABLES).await;

    up(&pool).await.unwrap();
    down(&pool).await.unwrap();

    assert_eq!(dump(&pool, &OLD_TABLES).await, before);
    assert_eq!(
        table_names(&pool).await,
        rows(&[
            "author_guidance",
            "books",
            "chapter_revisions",
            "chapters",
            "lore_entries",
            "series",
            "users"
        ])
    );
    MIGRATOR.undo(&pool, 0).await.unwrap();
    assert!(table_names(&pool).await.is_empty());
}

#[tokio::test]
async fn down_aborts_without_writes_when_a_loose_piece_exists() {
    let pool = seeded_pool().await;
    up(&pool).await.unwrap();
    sqlx::raw_sql(
        r#"INSERT INTO pieces (universe_id, title, position, label, format, shape, content, head_rev, snapshot_rev)
           VALUES (1, 'Loose', 0, 'short_story', 'prose', 'leaf', '{"ops":[]}', 0, 0)"#,
    )
    .execute(&pool)
    .await
    .unwrap();
    let before = dump(&pool, &NEW_TABLES).await;

    let error = down(&pool).await.unwrap_err();

    assert!(
        error
            .to_string()
            .contains("story model rows do not fit series, books and chapters"),
        "{error}"
    );
    assert_eq!(dump(&pool, &NEW_TABLES).await, before);
    assert_eq!(
        table_names(&pool).await,
        rows(&[
            "author_guidance",
            "containers",
            "legacy_ids",
            "lore_entries",
            "piece_revisions",
            "pieces",
            "universes",
            "users"
        ])
    );
}

#[tokio::test]
async fn up_after_down_gives_the_same_rows() {
    let pool = seeded_pool().await;
    up(&pool).await.unwrap();
    let first = dump(&pool, &NEW_TABLES).await;

    down(&pool).await.unwrap();
    up(&pool).await.unwrap();

    assert_eq!(dump(&pool, &NEW_TABLES).await, first);
}

#[tokio::test]
async fn triggers_reject_rows_that_break_shape_rules() {
    let pool = seeded_pool().await;
    up(&pool).await.unwrap();
    sqlx::raw_sql(
        "INSERT INTO universes (id, title, position) VALUES (9, 'Elsewhere', 0);
         INSERT INTO containers (id, universe_id, title, position, kind) VALUES (9, 9, 'Shorts', 0, 'collection');",
    )
    .execute(&pool)
    .await
    .unwrap();

    let cases = [
        (
            "INSERT INTO pieces (universe_id, container_id, title, position, label, format, shape) VALUES (1, 1, 'Short', 0, 'short_story', 'prose', 'group')",
            "container kind does not allow this label",
        ),
        (
            "INSERT INTO pieces (universe_id, container_id, title, position, label, format, shape) VALUES (1, 9, 'Novel', 0, 'novel', 'prose', 'group')",
            "piece container must be in the same universe",
        ),
        (
            r#"INSERT INTO pieces (universe_id, parent_id, title, position, label, format, shape, content, head_rev, snapshot_rev) VALUES (1, 1, 'Scene', 0, 'scene', 'script', 'leaf', '{"ops":[]}', 0, 0)"#,
            "piece parent must be a group in the same universe and format",
        ),
        (
            r#"INSERT INTO pieces (universe_id, parent_id, title, position, label, format, shape, content, head_rev, snapshot_rev) VALUES (1, 4, 'Nested', 0, 'chapter', 'prose', 'leaf', '{"ops":[]}', 0, 0)"#,
            "piece parent must be a group in the same universe and format",
        ),
        (
            r#"INSERT INTO piece_revisions (piece_id, rev, delta) VALUES (1, 1, '{"ops":[]}')"#,
            "a group has no revisions",
        ),
        (
            "UPDATE containers SET kind = 'tv_series' WHERE id = 1",
            "container change does not fit its pieces",
        ),
    ];
    for (sql, message) in cases {
        let error = sqlx::raw_sql(sql).execute(&pool).await.unwrap_err();
        assert!(error.to_string().contains(message), "{sql}: {error}");
    }

    sqlx::raw_sql("UPDATE pieces SET format = 'script' WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();
    let child_formats: Vec<String> =
        sqlx::query_scalar("SELECT format FROM pieces WHERE parent_id = 1 ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(child_formats, rows(&["script", "script"]));
}

#[tokio::test]
async fn new_pieces_take_the_next_legacy_ids_and_show_in_facade_views() {
    let pool = seeded_pool().await;
    up(&pool).await.unwrap();
    sqlx::raw_sql(
        r#"INSERT INTO pieces (universe_id, container_id, title, position, label, format, shape) VALUES (1, 1, 'Next', 2, 'novel', 'prose', 'group');
           INSERT INTO pieces (universe_id, parent_id, title, position, label, format, shape, content, head_rev, snapshot_rev) VALUES (1, 7, 'Fresh', 0, 'chapter', 'prose', 'leaf', '{"ops":[]}', 0, 0);"#,
    )
    .execute(&pool)
    .await
    .unwrap();

    let books: Vec<String> = sqlx::query_scalar(
        "SELECT json_array(id, piece_id, series_id, title) FROM legacy_books ORDER BY id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        books,
        rows(&[
            r#"[1,1,1,"First"]"#,
            r#"[2,2,1,"Empty"]"#,
            r#"[3,3,2,"Other"]"#,
            r#"[4,7,1,"Next"]"#,
        ])
    );
    let chapters: Vec<String> = sqlx::query_scalar(
        "SELECT json_array(id, piece_id, book_id, series_id, kind) FROM legacy_chapters ORDER BY id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        chapters,
        rows(&[
            r#"[1,4,1,1,"prologue"]"#,
            r#"[2,5,1,1,"chapter"]"#,
            r#"[3,6,3,2,"epilogue"]"#,
            r#"[4,8,4,1,"chapter"]"#,
        ])
    );
}
