mod common;

use std::num::NonZeroU64;

use quillai_api::chapter_revisions::{rebuild, RevisionError, RevisionStore};
use quillai_delta::Delta;
use sqlx::SqlitePool;

fn delta(wire: &str) -> Delta {
    Delta::from_json(wire).unwrap()
}

fn revisions() -> Vec<Delta> {
    [
        r#"{"ops":[{"insert":"Hi 😀日\n"}]}"#,
        r#"{"ops":[{"retain":3},{"retain":2,"attributes":{"bold":true}}]}"#,
        r#"{"ops":[{"delete":2},{"retain":4},{"insert":{"image":"cover.png"}}]}"#,
        r#"{"ops":[{"retain":1},{"retain":2,"attributes":{"bold":null,"italic":true}}]}"#,
    ]
    .map(delta)
    .to_vec()
}

async fn chapter() -> SqlitePool {
    let pool = common::test_pool().await;
    sqlx::raw_sql(
        "INSERT INTO series (id, title, position) VALUES (1, 'Series', 0);
         INSERT INTO books (id, series_id, title, position) VALUES (1, 1, 'Book', 0);
         INSERT INTO chapters (id, book_id, title, position, kind) VALUES (1, 1, 'Chapter', 0, 'chapter');",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool
}

async fn stored(pool: &SqlitePool) -> (i64, i64, String, i64) {
    sqlx::query_as(
        "SELECT head_rev, snapshot_rev, content, (SELECT COUNT(*) FROM chapter_revisions) FROM chapters WHERE id = 1",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[test]
fn rebuild_matches_compose_all_at_every_snapshot_boundary() {
    let revisions = revisions();
    let mut expected = Delta::new();
    for revision in &revisions {
        expected = expected.compose(revision).unwrap();
    }
    for split in 0..=revisions.len() {
        let snapshot = rebuild(&Delta::new(), &revisions[..split]).unwrap();
        assert_eq!(rebuild(&snapshot, &revisions[split..]).unwrap(), expected);
    }
    assert!(rebuild(
        &delta(r#"{"ops":[{"insert":"😀"}]}"#),
        &[delta(r#"{"ops":[{"delete":1}]}"#)]
    )
    .is_err());
}

#[tokio::test]
async fn persistence_replays_only_after_snapshot_and_keeps_all_revisions() {
    for interval in [1, 3] {
        let pool = chapter().await;
        let store = RevisionStore::new(pool.clone(), NonZeroU64::new(interval).unwrap());
        assert_eq!(store.load(1).await.unwrap().delta, Delta::new());
        let mut expected = Delta::new();
        for (index, revision) in revisions().iter().enumerate() {
            let previous = i64::try_from(index).unwrap();
            expected = expected.compose(revision).unwrap();
            let head = store.append(1, previous, revision).await.unwrap();
            assert_eq!(head.head_rev, previous + 1);
            assert_eq!(head.delta, expected);
            assert_eq!(store.load(1).await.unwrap(), head);
            let (head_rev, snapshot_rev, content, count) = stored(&pool).await;
            assert_eq!(head_rev, previous + 1);
            assert_eq!(count, head_rev);
            assert_eq!(snapshot_rev, head_rev / interval as i64 * interval as i64);
            let all = revisions();
            assert_eq!(
                delta(&content),
                rebuild(&Delta::new(), &all[..snapshot_rev as usize]).unwrap()
            );
        }
        sqlx::query("UPDATE chapter_revisions SET delta = 'null' WHERE rev <= (SELECT snapshot_rev FROM chapters WHERE id = 1)")
            .execute(&pool).await.unwrap();
        assert_eq!(store.load(1).await.unwrap().delta, expected);
    }
}

#[tokio::test]
async fn existing_content_is_revision_zero_and_failed_appends_leave_it_unchanged() {
    let pool = chapter().await;
    let initial = r#"{"ops":[{"insert":"😀"}]}"#;
    sqlx::query("UPDATE chapters SET content = ? WHERE id = 1")
        .bind(initial)
        .execute(&pool)
        .await
        .unwrap();
    let store = RevisionStore::new(pool.clone(), NonZeroU64::new(1).unwrap());
    assert_eq!(store.load(1).await.unwrap().delta, delta(initial));
    let before = stored(&pool).await;
    assert!(matches!(
        store
            .append(1, 0, &delta(r#"{"ops":[{"delete":1}]}"#))
            .await,
        Err(RevisionError::Delta(_))
    ));
    assert_eq!(stored(&pool).await, before);
    assert!(matches!(
        store.append(1, 7, &Delta::new()).await,
        Err(RevisionError::StaleHead)
    ));
    assert!(matches!(
        store.load(99).await,
        Err(RevisionError::MissingChapter)
    ));
    assert!(matches!(
        store.append(99, 0, &Delta::new()).await,
        Err(RevisionError::MissingChapter)
    ));
    assert_eq!(stored(&pool).await, before);
    sqlx::query("CREATE TRIGGER reject_snapshot BEFORE UPDATE OF content ON chapters BEGIN SELECT RAISE(ABORT, 'snapshot rejected'); END")
        .execute(&pool).await.unwrap();
    assert!(matches!(
        store.append(1, 0, &Delta::new()).await,
        Err(RevisionError::Database(_))
    ));
    assert_eq!(stored(&pool).await, before);
}

#[tokio::test]
async fn concurrent_expected_heads_allow_only_one_append() {
    let path = std::env::temp_dir().join(format!(
        "guz704-{}-{}.db",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(&path)
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(2)
        .connect_with(options)
        .await
        .unwrap();
    quillai_api::database::migrate(&pool).await.unwrap();
    sqlx::raw_sql(
        "INSERT INTO series (id, title, position) VALUES (1, 'Series', 0);
         INSERT INTO books (id, series_id, title, position) VALUES (1, 1, 'Book', 0);
         INSERT INTO chapters (id, book_id, title, position, kind) VALUES (1, 1, 'Chapter', 0, 'chapter');",
    )
    .execute(&pool)
    .await
    .unwrap();
    let store = RevisionStore::new(pool.clone(), NonZeroU64::new(3).unwrap());
    let revision = &revisions()[0];
    let (first, second) = tokio::join!(store.append(1, 0, revision), store.append(1, 0, revision));
    assert!(matches!(
        (first, second),
        (Ok(_), Err(RevisionError::StaleHead)) | (Err(RevisionError::StaleHead), Ok(_))
    ));
    let head = store.load(1).await.unwrap();
    assert_eq!(head.head_rev, 1);
    assert_eq!(head.delta, *revision);
    assert_eq!(stored(&pool).await.3, 1);
    pool.close().await;
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn revision_constraints_and_cascade_are_enforced() {
    let pool = chapter().await;
    let store = RevisionStore::new(pool.clone(), NonZeroU64::new(3).unwrap());
    store.append(1, 0, &revisions()[0]).await.unwrap();
    let created: String = sqlx::query_scalar("SELECT created_at FROM chapter_revisions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!created.is_empty());
    for statement in [
        "INSERT INTO chapter_revisions (chapter_id, rev, delta) VALUES (1, 1, '{\"ops\":[]}')",
        "INSERT INTO chapter_revisions (chapter_id, rev, delta) VALUES (99, 1, '{\"ops\":[]}')",
        "INSERT INTO chapter_revisions (chapter_id, rev, delta) VALUES (1, 2, 'broken')",
        "INSERT INTO chapter_revisions (chapter_id, rev, delta) VALUES (1, 0, '{\"ops\":[]}')",
        "UPDATE chapters SET snapshot_rev = head_rev + 1",
    ] {
        assert!(sqlx::query(statement).execute(&pool).await.is_err());
    }
    sqlx::query("DELETE FROM series WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chapter_revisions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn incomplete_or_invalid_replay_rolls_back_appends() {
    let pool = chapter().await;
    let store = RevisionStore::new(pool.clone(), NonZeroU64::new(3).unwrap());
    store.append(1, 0, &revisions()[0]).await.unwrap();
    sqlx::query("UPDATE chapter_revisions SET delta = ?")
        .bind(r#"{"ops":[{"insert":"\ud800"}]}"#)
        .execute(&pool)
        .await
        .unwrap();
    let before = stored(&pool).await;
    assert!(matches!(
        store.append(1, 1, &Delta::new()).await,
        Err(RevisionError::Delta(_))
    ));
    assert_eq!(stored(&pool).await, before);
    sqlx::query("DELETE FROM chapter_revisions")
        .execute(&pool)
        .await
        .unwrap();
    let before = stored(&pool).await;
    assert!(matches!(
        store.load(1).await,
        Err(RevisionError::IncompleteHistory)
    ));
    assert!(matches!(
        store.append(1, 1, &Delta::new()).await,
        Err(RevisionError::IncompleteHistory)
    ));
    assert_eq!(stored(&pool).await, before);
}
