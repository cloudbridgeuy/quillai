use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

mod common;

async fn request(app: &Router, method: &str, path: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

#[tokio::test]
async fn lists_explicit_markers_counts_and_usable_paths() {
    let pool = common::test_pool().await;
    for sql in [
        "INSERT INTO universes(id,title,position,deleted_at) VALUES (1,'World',0,'2026-01-01'),(2,'Other',1,NULL)",
        "INSERT INTO containers(id,universe_id,title,position,kind,deleted_at) VALUES (1,1,'Series',0,'book_series','2026-01-02'),(2,2,'Collection',0,'collection','2026-01-03'),(3,2,'TV',1,'tv_series','2026-01-03')",
        "INSERT INTO pieces(id,universe_id,container_id,title,position,label,format,shape,deleted_at) VALUES (1,1,1,'Book',0,'novel','prose','group','2026-01-04')",
        "INSERT INTO pieces(id,universe_id,parent_id,title,position,label,format,shape,content,head_rev,snapshot_rev,deleted_at) VALUES (2,1,1,'Chapter',0,'chapter','prose','leaf','{\"ops\":[]}',0,0,'2026-01-05'),(3,1,1,'Hidden',1,'chapter','prose','leaf','{\"ops\":[]}',0,0,NULL)",
        "INSERT INTO pieces(id,universe_id,title,position,label,format,shape,content,head_rev,snapshot_rev,deleted_at) VALUES (4,2,'Piece',0,'short_story','prose','leaf','{\"ops\":[]}',0,0,'2026-01-06')",
        "INSERT INTO lore_entries(id,universe_id,kind,name,deleted_at) VALUES (1,1,'note','Lore','2026-01-07')",
    ] { sqlx::query(sql).execute(&pool).await.unwrap(); }
    let app = quillai_api::app(pool.clone());
    let (status, result) = request(&app, "GET", "/trash").await;
    assert_eq!(status, StatusCode::OK);
    let items = result.as_array().unwrap();
    assert_eq!(
        items
            .iter()
            .map(|i| i["type"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "lore",
            "piece",
            "chapter",
            "book",
            "collection",
            "series",
            "series",
            "universe"
        ]
    );
    assert_eq!(items[0]["deletedParents"], serde_json::json!(["World"]));
    assert_eq!(
        items[0]["restoreAncestors"],
        serde_json::json!(["/universes/1"])
    );
    assert_eq!(
        items[2]["location"],
        serde_json::json!({"universe":"World","series":"Series","book":"Book"})
    );
    assert_eq!(
        items[2]["deletedParents"],
        serde_json::json!(["World", "Series", "Book"])
    );
    assert_eq!(items[5]["resourcePath"], "/containers/3");
    assert_eq!(
        items[3]["childCounts"],
        serde_json::json!({"chapters":2,"pieces":0,"lore":0})
    );
    assert_eq!(
        items[6]["childCounts"],
        serde_json::json!({"chapters":2,"pieces":1,"lore":1})
    );
    assert_eq!(
        items[7]["childCounts"],
        serde_json::json!({"chapters":2,"pieces":1,"lore":1})
    );
    assert_eq!(
        request(&app, "DELETE", "/universes/1/lore/1?permanent=true")
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request(&app, "POST", "/universes/1/restore").await.0,
        StatusCode::OK
    );
    assert_eq!(
        request(
            &app,
            "POST",
            &format!("{}/restore", items[0]["resourcePath"].as_str().unwrap())
        )
        .await
        .0,
        StatusCode::OK
    );
    sqlx::query("UPDATE lore_entries SET deleted_at = '2026-01-07' WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        request(&app, "DELETE", "/universes/1/lore/1?permanent=true")
            .await
            .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(&app, "POST", "/pieces/2/restore").await.0,
        StatusCode::OK
    );
    assert_eq!(
        request(&app, "DELETE", "/containers/2?permanent=true")
            .await
            .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(&app, "DELETE", "/pieces/4?permanent=true").await.0,
        StatusCode::NO_CONTENT
    );
    let (_, remaining) = request(&app, "GET", "/trash").await;
    let paths: Vec<_> = remaining
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["resourcePath"].as_str().unwrap())
        .collect();
    assert_eq!(paths, ["/containers/3"]);
}

#[tokio::test]
async fn ancestor_markers_and_permanent_counts_match_removed_rows() {
    for (table, path, kind, piece_count, lore_count) in [
        ("pieces", "/pieces/1", "book", 0, 1),
        ("containers", "/series/1", "series", 1, 0),
        ("universes", "/universes/1", "universe", 1, 0),
    ] {
        let pool = common::test_pool().await;
        for sql in [
            "INSERT INTO universes(id,title,position) VALUES (1,'World',0)",
            "INSERT INTO containers(id,universe_id,title,position,kind) VALUES (1,1,'Series',0,'book_series')",
            "INSERT INTO pieces(id,universe_id,container_id,title,position,label,format,shape) VALUES (1,1,1,'Book',0,'novel','prose','group')",
            "INSERT INTO pieces(id,universe_id,parent_id,title,position,label,format,shape,content,head_rev,snapshot_rev,deleted_at) VALUES (2,1,1,'Hidden',0,'chapter','prose','leaf','{\"ops\":[]}',0,0,NULL),(3,1,1,'Deleted',1,'chapter','prose','leaf','{\"ops\":[]}',0,0,'2026-01-02')",
            "INSERT INTO lore_entries(id,universe_id,kind,name) VALUES (1,1,'note','Lore')",
        ] {
            sqlx::query(sql).execute(&pool).await.unwrap();
        }
        sqlx::query(&format!(
            "UPDATE {table} SET deleted_at = '2026-01-01' WHERE id = 1"
        ))
        .execute(&pool)
        .await
        .unwrap();
        let app = quillai_api::app(pool.clone());
        let (_, trash) = request(&app, "GET", "/trash").await;
        let rows = trash.as_array().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["title"], "Deleted");
        assert_eq!(rows[1]["type"], kind);
        assert_eq!(
            rows[1]["childCounts"],
            serde_json::json!({"chapters":2,"pieces":piece_count,"lore":1-lore_count})
        );
        assert_eq!(
            request(&app, "DELETE", &format!("{path}?permanent=true"))
                .await
                .0,
            StatusCode::NO_CONTENT
        );
        let remaining_pieces: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pieces")
            .fetch_one(&pool)
            .await
            .unwrap();
        let remaining_lore: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM lore_entries")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!((remaining_pieces, remaining_lore), (0, lore_count));
        assert!(request(&app, "GET", "/trash")
            .await
            .1
            .as_array()
            .unwrap()
            .is_empty());
    }
}
