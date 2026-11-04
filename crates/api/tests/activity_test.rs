use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

mod common;

async fn get(app: &Router, uri: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn activity_orders_latest_revisions_and_excludes_hidden_items() {
    let pool = common::test_pool().await;
    sqlx::raw_sql(r#"
        INSERT INTO universes(id,title,position) VALUES (1,'World',0);
        INSERT INTO containers(id,universe_id,title,position,kind) VALUES (1,1,'Series',0,'book_series'),(2,1,'Hidden',1,'collection');
        INSERT INTO pieces(id,universe_id,container_id,title,position,label,format,shape) VALUES (1,1,1,'Book',0,'novel','prose','group');
        INSERT INTO pieces(id,universe_id,parent_id,title,position,label,format,shape,content,head_rev,snapshot_rev) VALUES (2,1,1,'Chapter',0,'chapter','prose','leaf','{}',2,0);
        INSERT INTO pieces(id,universe_id,title,position,label,format,shape,content,head_rev,snapshot_rev) VALUES (3,1,'Loose',0,'piece','prose','leaf','{}',1,0),(4,1,'Deleted',1,'piece','prose','leaf','{}',1,0),(5,1,'Unedited',2,'piece','prose','leaf','{}',0,0);
        INSERT INTO pieces(id,universe_id,container_id,title,position,label,format,shape,content,head_rev,snapshot_rev) VALUES (6,1,2,'Hidden',0,'piece','prose','leaf','{}',1,0);
        INSERT INTO piece_revisions(piece_id,rev,delta,created_at) VALUES (2,1,'{}','2026-01-01 00:00:00'),(2,2,'{}','2026-11-04 17:00:00'),(3,1,'{}','2026-11-03 17:00:00'),(4,1,'{}','2026-11-05 00:00:00'),(6,1,'{}','2026-11-05 00:00:00');
        UPDATE pieces SET deleted_at='2026-11-06' WHERE id=4;
        UPDATE containers SET deleted_at='2026-11-06' WHERE id=2;
    "#).execute(&pool).await.unwrap();
    let (status, body) = get(&quillai_api::app(pool), "/activity").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!([
            {"pieceId":2,"universeId":1,"title":"Chapter","scriptTitle":null,"label":"chapter","format":"prose","parentId":1,"changedAt":"2026-11-04T17:00:00Z","chapter":{"seriesId":1,"bookId":1,"chapterId":1}},
            {"pieceId":3,"universeId":1,"title":"Loose","scriptTitle":null,"label":"piece","format":"prose","parentId":null,"changedAt":"2026-11-03T17:00:00Z","chapter":null}
        ])
    );
}

#[tokio::test]
async fn activity_breaks_timestamp_ties_by_descending_piece_id() {
    let pool = common::test_pool().await;
    sqlx::raw_sql(r#"
        INSERT INTO universes(id,title,position) VALUES (1,'World',0);
        INSERT INTO pieces(id,universe_id,title,position,label,format,shape,content,head_rev,snapshot_rev) VALUES (1,1,'First',0,'piece','prose','leaf','{}',1,0),(2,1,'Second',1,'piece','prose','leaf','{}',1,0);
        INSERT INTO piece_revisions(piece_id,rev,delta,created_at) VALUES (1,1,'{}','2026-11-04 17:00:00'),(2,1,'{}','2026-11-04 17:00:00');
    "#).execute(&pool).await.unwrap();
    let (status, body) = get(&quillai_api::app(pool), "/activity").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!([
            {"pieceId":2,"universeId":1,"title":"Second","scriptTitle":null,"label":"piece","format":"prose","parentId":null,"changedAt":"2026-11-04T17:00:00Z","chapter":null},
            {"pieceId":1,"universeId":1,"title":"First","scriptTitle":null,"label":"piece","format":"prose","parentId":null,"changedAt":"2026-11-04T17:00:00Z","chapter":null}
        ])
    );
}

#[tokio::test]
async fn activity_returns_script_title_only_for_script_scenes() {
    let pool = common::test_pool().await;
    sqlx::raw_sql(r#"
        INSERT INTO universes(id,title,position) VALUES (1,'World',0);
        INSERT INTO pieces(id,universe_id,title,position,label,format,shape) VALUES (1,1,'Screenplay',0,'screenplay','script','group'),(2,1,'Novel',1,'novel','prose','group');
        INSERT INTO pieces(id,universe_id,parent_id,title,position,label,format,shape,content,head_rev,snapshot_rev) VALUES (3,1,1,'Opening scene',0,'scene','script','leaf','{}',1,0),(4,1,2,'Chapter',0,'chapter','prose','leaf','{}',1,0),(5,1,NULL,'Loose script',2,'scene','script','leaf','{}',1,0),(6,1,NULL,'Loose prose',3,'piece','prose','leaf','{}',1,0);
        INSERT INTO piece_revisions(piece_id,rev,delta) VALUES (3,1,'{}'),(4,1,'{}'),(5,1,'{}'),(6,1,'{}');
    "#).execute(&pool).await.unwrap();
    let (status, body) = get(&quillai_api::app(pool), "/activity").await;
    assert_eq!(status, StatusCode::OK);
    let rows = body.as_array().unwrap();
    assert_eq!(rows.len(), 4);
    let scene = rows.iter().find(|row| row["pieceId"] == 3).unwrap();
    assert_eq!(scene.get("scriptTitle"), Some(&json!("Screenplay")));
    assert_eq!(scene["title"], "Opening scene");
    for id in [4, 5, 6] {
        let row = rows.iter().find(|row| row["pieceId"] == id).unwrap();
        assert_eq!(row.get("scriptTitle"), Some(&Value::Null));
    }
}

#[tokio::test]
async fn activity_limit_defaults_caps_and_rejects_invalid_values() {
    let pool = common::test_pool().await;
    sqlx::query("INSERT INTO universes(id,title,position) VALUES (1,'World',0)")
        .execute(&pool)
        .await
        .unwrap();
    for id in 1..=51 {
        sqlx::query("INSERT INTO pieces(id,universe_id,title,position,label,format,shape,content,head_rev,snapshot_rev) VALUES (?,1,'Leaf',?,'piece','prose','leaf','{}',1,0)").bind(id).bind(id).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO piece_revisions(piece_id,rev,delta) VALUES (?,1,'{}')")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
    }
    let app = quillai_api::app(pool);
    for (uri, count) in [
        ("/activity", 20),
        ("/activity?limit=1", 1),
        ("/activity?limit=50", 50),
    ] {
        let (status, body) = get(&app, uri).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.as_array().unwrap().len(), count);
    }
    for uri in [
        "/activity?limit=0",
        "/activity?limit=51",
        "/activity?limit=abc",
        "/activity?limit=-1",
        "/activity?limit=",
    ] {
        let (status, body) = get(&app, uri).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{uri}");
        assert_eq!(body["code"], "VALIDATION_ERROR");
    }
}
