use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

mod common;

async fn request(app: &Router, method: &str, uri: &str, body: Value) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        },
    )
}

#[tokio::test]
async fn chapter_trash_preserves_history_and_scope() {
    let app = quillai_api::app(common::test_pool().await);
    let (_, series) = request(
        &app,
        "POST",
        "/series",
        json!({"title":"Series", "position":0}),
    )
    .await;
    let books = format!("/series/{}/books", series["id"]);
    let (_, book) = request(&app, "POST", &books, json!({"title":"Book", "position":0})).await;
    let (_, other) = request(&app, "POST", &books, json!({"title":"Other", "position":1})).await;
    let base = format!("{books}/{}/chapters", book["id"]);
    let chapter = |title, position| json!({"title":title, "position":position, "kind":"chapter", "content":{"ops":[{"insert":"Text\\n"}]}});
    let (_, first) = request(&app, "POST", &base, chapter("First", 0)).await;
    for title in ["Second", "Third"] {
        assert_eq!(
            request(&app, "POST", &base, chapter(title, 5)).await.0,
            StatusCode::CREATED
        );
    }
    let path = format!("{base}/{}", first["id"]);
    let wrong = format!("{books}/{}/chapters/{}", other["id"], first["id"]);
    let delta = json!({"baseRev":0, "delta":{"ops":[{"insert":"Saved "}]}});
    assert_eq!(
        request(&app, "POST", &format!("{path}/deltas"), delta.clone())
            .await
            .0,
        StatusCode::OK
    );
    let before = request(&app, "GET", &path, Value::Null).await.1;
    let history = request(&app, "GET", &format!("{path}/deltas?since=0"), Value::Null)
        .await
        .1;
    assert_eq!(
        request(&app, "DELETE", &path, Value::Null).await.0,
        StatusCode::NO_CONTENT
    );
    for uri in [path.clone(), format!("{path}/deltas?since=0")] {
        assert_eq!(
            request(&app, "GET", &uri, Value::Null).await.0,
            StatusCode::NOT_FOUND
        );
    }
    assert_eq!(
        request(&app, "POST", &format!("{path}/deltas"), delta)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request(&app, "POST", &format!("{wrong}/restore"), Value::Null)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request(
            &app,
            "DELETE",
            &format!("{wrong}?permanent=true"),
            Value::Null
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request(&app, "GET", &base, Value::Null)
            .await
            .1
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let (status, restored) = request(&app, "POST", &format!("{path}/restore"), Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(restored["position"], 6);
    assert_eq!(restored["content"], before["content"]);
    assert_eq!(restored["head_rev"], before["head_rev"]);
    assert_eq!(
        request(&app, "GET", &format!("{path}/deltas?since=0"), Value::Null)
            .await
            .1,
        history
    );
    assert_eq!(
        request(&app, "DELETE", &path, Value::Null).await.0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(
            &app,
            "DELETE",
            &format!("{path}?permanent=true"),
            Value::Null
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(&app, "POST", &format!("{path}/restore"), Value::Null)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn scene_and_group_trash_are_independent() {
    let pool = common::test_pool().await;
    let app = quillai_api::app(pool.clone());
    let (_, universe) = request(
        &app,
        "POST",
        "/universes",
        json!({"title":"Universe", "position":0}),
    )
    .await;
    let roots = format!("/universes/{}/pieces", universe["id"]);
    let (_, group) = request(
        &app,
        "POST",
        &roots,
        json!({"title":"Script", "position":0, "label":"screenplay", "shape":"group"}),
    )
    .await;
    let (_, target) = request(
        &app,
        "POST",
        &roots,
        json!({"title":"Target", "position":1, "label":"screenplay", "shape":"group"}),
    )
    .await;
    let group_path = format!("/pieces/{}", group["id"]);
    let children = format!("{group_path}/children");
    let (_, one) = request(
        &app,
        "POST",
        &children,
        json!({"title":"One", "position":0, "label":"scene"}),
    )
    .await;
    let (_, two) = request(
        &app,
        "POST",
        &children,
        json!({"title":"Two", "position":1, "label":"scene"}),
    )
    .await;
    let one_path = format!("/pieces/{}", one["id"]);
    let two_path = format!("/pieces/{}", two["id"]);
    let delta = json!({"baseRev":0, "delta":{"ops":[{"insert":"Scene"}]}});
    assert_eq!(
        request(&app, "POST", &format!("{one_path}/deltas"), delta.clone())
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        request(&app, "DELETE", &one_path, Value::Null).await.0,
        StatusCode::NO_CONTENT
    );
    let scenes = request(&app, "GET", &format!("{group_path}/scenes"), Value::Null)
        .await
        .1;
    assert_eq!(scenes["scenes"].as_array().unwrap().len(), 1);
    assert_eq!(scenes["scenes"][0]["id"], two["id"]);
    assert_eq!(
        request(&app, "DELETE", &group_path, Value::Null).await.0,
        StatusCode::NO_CONTENT
    );
    sqlx::query("CREATE TEMP TRIGGER reject_hidden_claim BEFORE UPDATE OF head_rev ON pieces WHEN OLD.id NOT IN (SELECT id FROM active_pieces) BEGIN SELECT RAISE(ABORT, 'hidden claim'); END")
        .execute(&pool).await.unwrap();
    assert_eq!(
        request(
            &app,
            "POST",
            &format!("/pieces/{}/attach", target["id"]),
            json!({"pieceId":two["id"],"position":0})
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let store = quillai_api::chapter_revisions::RevisionStore::new(
        pool.clone(),
        std::num::NonZeroU64::new(100).unwrap(),
    );
    for id in [one["id"].as_i64().unwrap(), two["id"].as_i64().unwrap()] {
        assert!(matches!(
            store.load(id).await,
            Err(quillai_api::chapter_revisions::RevisionError::MissingChapter)
        ));
        assert!(matches!(
            store.append(id, 0, &quillai_delta::Delta::new()).await,
            Err(quillai_api::chapter_revisions::RevisionError::MissingChapter)
        ));
    }
    for (method, uri, body) in [
        ("PUT", two_path.clone(), json!({"title":"Changed"})),
        (
            "POST",
            format!("{two_path}/placement"),
            json!({"containerId":null,"position":0}),
        ),
        (
            "POST",
            format!("{two_path}/split"),
            json!({"title":"Split"}),
        ),
        (
            "POST",
            format!("{group_path}/attach"),
            json!({"pieceId":two["id"],"position":0}),
        ),
    ] {
        assert_eq!(
            request(&app, method, &uri, body).await.0,
            StatusCode::NOT_FOUND
        );
    }
    let timestamps: (Option<String>, Option<String>, Option<String>) = sqlx::query_as("SELECT (SELECT deleted_at FROM pieces WHERE id = ?), (SELECT deleted_at FROM pieces WHERE id = ?), (SELECT deleted_at FROM pieces WHERE id = ?)")
        .bind(group["id"].as_i64().unwrap()).bind(one["id"].as_i64().unwrap()).bind(two["id"].as_i64().unwrap()).fetch_one(&pool).await.unwrap();
    assert!(timestamps.0.is_some() && timestamps.1.is_some() && timestamps.2.is_none());
    for uri in [&two_path, &format!("{two_path}/deltas?since=0")] {
        assert_eq!(
            request(&app, "GET", uri, Value::Null).await.0,
            StatusCode::NOT_FOUND
        );
    }
    assert_eq!(
        request(&app, "POST", &format!("{two_path}/deltas"), delta)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request(&app, "POST", &format!("{one_path}/restore"), Value::Null)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request(&app, "POST", &format!("{group_path}/restore"), Value::Null)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        request(&app, "GET", &two_path, Value::Null).await.0,
        StatusCode::OK
    );
    assert_eq!(
        request(&app, "GET", &one_path, Value::Null).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request(
            &app,
            "POST",
            &format!("{group_path}/attach"),
            json!({"pieceId":one["id"],"position":0})
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );

    assert_eq!(
        request(&app, "POST", &format!("{one_path}/restore"), Value::Null)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        request(&app, "DELETE", &one_path, Value::Null).await.0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(
            &app,
            "DELETE",
            &format!("{one_path}?permanent=true"),
            Value::Null
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(&app, "POST", &format!("{one_path}/restore"), Value::Null)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
}
