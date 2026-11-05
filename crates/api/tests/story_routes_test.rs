use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

mod common;

use common::test_pool;

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
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, body)
}

async fn created(app: &Router, uri: &str, body: Value) -> Value {
    let (status, body) = request(app, "POST", uri, body).await;
    assert_eq!(status, StatusCode::CREATED, "{uri}: {body}");
    body
}

async fn status(app: &Router, method: &str, uri: &str, body: Value) -> StatusCode {
    request(app, method, uri, body).await.0
}

async fn universe(app: &Router) -> i64 {
    created(app, "/universes", json!({"title": "World", "position": 0})).await["id"]
        .as_i64()
        .unwrap()
}

async fn container(app: &Router, universe_id: i64, kind: &str) -> i64 {
    created(
        app,
        &format!("/universes/{universe_id}/containers"),
        json!({"title": kind, "position": 0, "kind": kind}),
    )
    .await["id"]
        .as_i64()
        .unwrap()
}

#[tokio::test]
async fn universes_crud_validates_and_cascades() {
    let pool = test_pool().await;
    let app = quillai_api::app(pool.clone());
    let second = created(&app, "/universes", json!({"title": "B", "position": 1})).await;
    let first = created(&app, "/universes", json!({"title": "A", "position": 0})).await;
    assert_eq!(
        request(&app, "GET", "/universes", Value::Null).await,
        (StatusCode::OK, json!([first, second]))
    );
    let uri = format!("/universes/{}", first["id"]);
    assert_eq!(
        request(&app, "PUT", &uri, json!({"title": "Renamed"})).await,
        (
            StatusCode::OK,
            json!({"id": first["id"], "title": "Renamed", "position": 0})
        )
    );
    for body in [
        json!({"title": " ", "position": 0}),
        json!({"title": "A", "position": -1}),
        json!({"title": "A"}),
    ] {
        assert_eq!(
            status(&app, "POST", "/universes", body).await,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    assert_eq!(
        status(&app, "PUT", &uri, json!({"position": -1})).await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let id = first["id"].as_i64().unwrap();
    container(&app, id, "collection").await;
    created(
        &app,
        &format!("{uri}/pieces"),
        json!({"title": "Scrap", "position": 0, "label": "piece", "shape": "leaf"}),
    )
    .await;
    created(
        &app,
        &format!("{uri}/lore"),
        json!({"kind": "note", "name": "N"}),
    )
    .await;
    assert_eq!(
        status(
            &app,
            "DELETE",
            &format!("{uri}?permanent=true"),
            Value::Null
        )
        .await,
        StatusCode::NO_CONTENT
    );
    for method in ["GET", "PUT", "DELETE"] {
        assert_eq!(
            status(&app, method, &uri, json!({})).await,
            StatusCode::NOT_FOUND
        );
    }
    let remaining: (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM containers), (SELECT COUNT(*) FROM pieces), (SELECT COUNT(*) FROM lore_entries)",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(remaining, (0, 0, 0));
}

#[tokio::test]
async fn universe_lore_is_scoped_to_its_universe() {
    let app = quillai_api::app(test_pool().await);
    let id = universe(&app).await;
    let other = universe(&app).await;
    let collection = format!("/universes/{id}/lore");
    let entry = created(
        &app,
        &collection,
        json!({"kind": "character", "name": "Ada"}),
    )
    .await;
    assert_eq!(
        entry,
        json!({"id": entry["id"], "universe_id": id, "kind": "character", "name": "Ada",
            "keywords": [], "aliases": [], "visible_text": "", "hidden_notes": "",
            "pinned": false, "level": "include"})
    );
    let uri = format!("{collection}/{}", entry["id"]);
    let (code, updated) = request(&app, "PUT", &uri, json!({"pinned": true})).await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(updated["pinned"], true);
    assert_eq!(
        request(&app, "GET", &collection, Value::Null).await,
        (StatusCode::OK, json!([updated]))
    );
    assert_eq!(
        status(
            &app,
            "POST",
            &collection,
            json!({"kind": "bad", "name": "X"})
        )
        .await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    for path in [
        format!("/universes/{other}/lore/{}", entry["id"]),
        format!("/universes/999/lore/{}", entry["id"]),
    ] {
        for method in ["GET", "PUT", "DELETE"] {
            assert_eq!(
                status(&app, method, &path, json!({})).await,
                StatusCode::NOT_FOUND
            );
        }
    }
    for method in ["GET", "POST"] {
        assert_eq!(
            status(
                &app,
                method,
                "/universes/999/lore",
                json!({"kind": "note", "name": "N"})
            )
            .await,
            StatusCode::NOT_FOUND
        );
    }
    assert_eq!(
        status(&app, "DELETE", &uri, Value::Null).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(&app, "GET", &collection, Value::Null).await,
        (StatusCode::OK, json!([]))
    );
}

#[tokio::test]
async fn containers_crud_with_fixed_kind() {
    let app = quillai_api::app(test_pool().await);
    let id = universe(&app).await;
    let list = format!("/universes/{id}/containers");
    let tv = created(
        &app,
        &list,
        json!({"title": "Show", "position": 1, "kind": "tv_series"}),
    )
    .await;
    let books = created(
        &app,
        &list,
        json!({"title": "Saga", "position": 0, "kind": "book_series"}),
    )
    .await;
    assert_eq!(
        books,
        json!({"id": books["id"], "universeId": id, "title": "Saga", "position": 0, "kind": "book_series"})
    );
    assert_eq!(
        request(&app, "GET", &list, Value::Null).await,
        (StatusCode::OK, json!([books, tv]))
    );
    for body in [
        json!({"title": "X", "position": 0, "kind": "anthology"}),
        json!({"title": "", "position": 0, "kind": "collection"}),
        json!({"title": "X", "position": -1, "kind": "collection"}),
        json!({"title": "X", "position": 0}),
    ] {
        assert_eq!(
            status(&app, "POST", &list, body).await,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    let uri = format!("/containers/{}", tv["id"]);
    for body in [
        json!({"kind": "collection"}),
        json!({"universeId": id}),
        json!({"title": " "}),
    ] {
        assert_eq!(
            status(&app, "PUT", &uri, body).await,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    let (code, renamed) = request(
        &app,
        "PUT",
        &uri,
        json!({"title": "Renamed", "position": 3}),
    )
    .await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(renamed["kind"], "tv_series");
    assert_eq!(renamed["title"], "Renamed");
    assert_eq!(
        request(&app, "GET", &uri, Value::Null).await,
        (StatusCode::OK, renamed)
    );
    assert_eq!(
        status(&app, "DELETE", &uri, Value::Null).await,
        StatusCode::NO_CONTENT
    );
    for method in ["GET", "PUT", "DELETE"] {
        assert_eq!(
            status(&app, method, &uri, json!({})).await,
            StatusCode::NOT_FOUND
        );
    }
    for method in ["GET", "POST"] {
        assert_eq!(
            status(
                &app,
                method,
                "/universes/999/containers",
                json!({"title": "X", "position": 0, "kind": "collection"})
            )
            .await,
            StatusCode::NOT_FOUND
        );
    }
}

#[tokio::test]
async fn loose_pieces_leaves_groups_and_children() {
    let app = quillai_api::app(test_pool().await);
    let id = universe(&app).await;
    let loose = format!("/universes/{id}/pieces");
    let leaf = created(
        &app,
        &loose,
        json!({"title": "Scrap", "position": 1, "label": "short_story", "shape": "leaf"}),
    )
    .await;
    assert_eq!(
        leaf,
        json!({"id": leaf["id"], "universeId": id, "containerId": null, "parentId": null,
            "title": "Scrap", "position": 1, "label": "short_story", "format": "prose", "shape": "leaf", "timelinePointId": null})
    );
    let mut detail = leaf.clone();
    detail["content"] = json!({"ops": []});
    detail["headRev"] = json!(0);
    assert_eq!(
        request(&app, "GET", &format!("/pieces/{}", leaf["id"]), Value::Null).await,
        (StatusCode::OK, detail)
    );
    let film = created(
        &app,
        &loose,
        json!({"title": "Film", "position": 0, "label": "screenplay", "shape": "group"}),
    )
    .await;
    assert_eq!(film["format"], "script");
    let children = format!("/pieces/{}/children", film["id"]);
    let scene = created(&app, &children, json!({"title": "Open", "position": 0})).await;
    assert_eq!(
        (
            &scene["label"],
            &scene["format"],
            &scene["parentId"],
            &scene["shape"]
        ),
        (
            &json!("scene"),
            &json!("script"),
            &film["id"],
            &json!("leaf")
        )
    );
    let mut group = film.clone();
    group["children"] = json!([scene]);
    assert_eq!(
        request(&app, "GET", &format!("/pieces/{}", film["id"]), Value::Null).await,
        (StatusCode::OK, group)
    );
    assert_eq!(
        request(&app, "GET", &format!("{loose}?container=none"), Value::Null).await,
        (StatusCode::OK, json!([film, leaf]))
    );
    for query in ["", "?container=1"] {
        assert_eq!(
            status(&app, "GET", &format!("{loose}{query}"), Value::Null).await,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    assert_eq!(
        status(
            &app,
            "POST",
            &format!("/pieces/{}/children", leaf["id"]),
            json!({"title": "X", "position": 0})
        )
        .await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    for body in [
        json!({"title": "X", "position": 0, "label": "saga", "shape": "leaf"}),
        json!({"title": "X", "position": 0, "label": "piece", "shape": "tree"}),
        json!({"title": "X", "position": 0, "label": "piece", "shape": "leaf", "format": "verse"}),
        json!({"title": " ", "position": 0, "label": "piece", "shape": "leaf"}),
        json!({"title": "X", "position": -1, "label": "piece", "shape": "leaf"}),
    ] {
        assert_eq!(
            status(&app, "POST", &loose, body).await,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    for uri in ["/universes/999/pieces", "/pieces/999/children"] {
        assert_eq!(
            status(
                &app,
                "POST",
                uri,
                json!({"title": "X", "position": 0, "label": "piece", "shape": "leaf"})
            )
            .await,
            StatusCode::NOT_FOUND
        );
    }
    assert_eq!(
        status(
            &app,
            "DELETE",
            &format!("/pieces/{}", film["id"]),
            Value::Null
        )
        .await,
        StatusCode::NO_CONTENT
    );
    for piece in [&film, &scene] {
        assert_eq!(
            status(
                &app,
                "GET",
                &format!("/pieces/{}", piece["id"]),
                Value::Null
            )
            .await,
            StatusCode::NOT_FOUND
        );
    }
}

#[tokio::test]
async fn container_membership_and_updates() {
    let app = quillai_api::app(test_pool().await);
    let id = universe(&app).await;
    let books = container(&app, id, "book_series").await;
    let tv = container(&app, id, "tv_series").await;
    let shelf = container(&app, id, "collection").await;
    let piece = |title: &str, label: &str, shape: &str| json!({"title": title, "position": 0, "label": label, "shape": shape});
    let novel = created(
        &app,
        &format!("/containers/{books}/pieces"),
        piece("Novel", "novel", "group"),
    )
    .await;
    assert_eq!(novel["containerId"], books);
    for (container_id, label) in [(books, "short_story"), (books, "episode"), (tv, "novel")] {
        let (code, body) = request(
            &app,
            "POST",
            &format!("/containers/{container_id}/pieces"),
            piece("X", label, "leaf"),
        )
        .await;
        assert_eq!(code, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        assert_eq!(body["code"], "VALIDATION_ERROR");
    }
    let episode = created(
        &app,
        &format!("/containers/{tv}/pieces"),
        piece("Pilot", "episode", "leaf"),
    )
    .await;
    assert_eq!(episode["format"], "script");
    created(
        &app,
        &format!("/containers/{shelf}/pieces"),
        piece("Short", "short_story", "group"),
    )
    .await;
    assert_eq!(
        request(
            &app,
            "GET",
            &format!("/containers/{books}/pieces"),
            Value::Null
        )
        .await,
        (StatusCode::OK, json!([novel]))
    );
    assert_eq!(
        status(&app, "GET", "/containers/999/pieces", Value::Null).await,
        StatusCode::NOT_FOUND
    );
    let chapter = created(
        &app,
        &format!("/pieces/{}/children", novel["id"]),
        json!({"title": "One", "position": 0}),
    )
    .await;
    assert_eq!(chapter["label"], "chapter");
    let novel_uri = format!("/pieces/{}", novel["id"]);
    let chapter_uri = format!("/pieces/{}", chapter["id"]);
    for (uri, body) in [
        (&novel_uri, json!({"label": "short_story"})),
        (&novel_uri, json!({"universeId": id})),
        (&novel_uri, json!({"containerId": shelf})),
        (&novel_uri, json!({"label": "saga"})),
        (&chapter_uri, json!({"content": {"ops": []}})),
        (&chapter_uri, json!({"format": "script"})),
        (&chapter_uri, json!({"parentId": null})),
    ] {
        let (code, response) = request(&app, "PUT", uri, body.clone()).await;
        assert_eq!(code, StatusCode::UNPROCESSABLE_ENTITY, "{body}: {response}");
    }
    let (code, updated) = request(
        &app,
        "PUT",
        &novel_uri,
        json!({"title": "Novella", "label": "novella", "format": "script"}),
    )
    .await;
    assert_eq!(code, StatusCode::OK, "{updated}");
    assert_eq!(
        (
            &updated["label"],
            &updated["format"],
            &updated["children"][0]["format"]
        ),
        (&json!("novella"), &json!("script"), &json!("script"))
    );
    let (code, renamed) = request(
        &app,
        "PUT",
        &chapter_uri,
        json!({"title": "Uno", "label": "scene"}),
    )
    .await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(
        (
            &renamed["title"],
            &renamed["label"],
            &renamed["format"],
            &renamed["headRev"]
        ),
        (&json!("Uno"), &json!("scene"), &json!("script"), &json!(0))
    );
    assert_eq!(
        status(&app, "PUT", "/pieces/999", json!({"title": "X"})).await,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn leaf_deltas_sync_and_groups_have_none() {
    let app = quillai_api::app(test_pool().await);
    let id = universe(&app).await;
    let leaf = created(
        &app,
        &format!("/universes/{id}/pieces"),
        json!({"title": "Scrap", "position": 0, "label": "piece", "shape": "leaf"}),
    )
    .await;
    let group = created(
        &app,
        &format!("/universes/{id}/pieces"),
        json!({"title": "Book", "position": 0, "label": "novel", "shape": "group"}),
    )
    .await;
    let deltas = format!("/pieces/{}/deltas", leaf["id"]);
    let first = json!({"ops": [{"insert": "Hi"}]});
    assert_eq!(
        request(&app, "POST", &deltas, json!({"baseRev": 0, "delta": first})).await,
        (StatusCode::OK, json!({"rev": 1, "delta": first}))
    );
    let (code, second) = request(
        &app,
        "POST",
        &deltas,
        json!({"baseRev": 0, "delta": {"ops": [{"insert": "A"}]}}),
    )
    .await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!(second["rev"], 2);
    assert_eq!(
        request(&app, "GET", &format!("{deltas}?since=0"), Value::Null).await,
        (StatusCode::OK, json!([{"rev": 1, "delta": first}, second]))
    );
    let (_, detail) = request(&app, "GET", &format!("/pieces/{}", leaf["id"]), Value::Null).await;
    assert_eq!(detail["headRev"], 2);
    assert_eq!(detail["content"], json!({"ops": [{"insert": "HiA"}]}));
    assert_eq!(
        status(&app, "GET", &format!("{deltas}?since=3"), Value::Null).await,
        StatusCode::CONFLICT
    );
    assert_eq!(
        status(&app, "POST", &deltas, json!({"baseRev": 0, "delta": "bad"})).await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    for uri in [
        format!("/pieces/{}/deltas", group["id"]),
        "/pieces/999/deltas".into(),
    ] {
        assert_eq!(
            status(&app, "GET", &format!("{uri}?since=0"), Value::Null).await,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            status(
                &app,
                "POST",
                &uri,
                json!({"baseRev": 0, "delta": {"ops": []}})
            )
            .await,
            StatusCode::NOT_FOUND
        );
    }
}

#[tokio::test]
async fn facade_shares_universe_lore_and_hides_other_shapes() {
    let pool = test_pool().await;
    let app = quillai_api::app(pool.clone());
    let id = universe(&app).await;
    let first = container(&app, id, "book_series").await;
    let second = container(&app, id, "book_series").await;
    let shelf = container(&app, id, "collection").await;
    created(
        &app,
        &format!("/universes/{id}/pieces"),
        json!({"title": "Loose", "position": 0, "label": "novel", "shape": "group"}),
    )
    .await;
    created(
        &app,
        &format!("/containers/{shelf}/pieces"),
        json!({"title": "Shelved", "position": 0, "label": "novel", "shape": "group"}),
    )
    .await;
    let novel = created(
        &app,
        &format!("/containers/{first}/pieces"),
        json!({"title": "Novel", "position": 0, "label": "novel", "shape": "group"}),
    )
    .await;
    created(
        &app,
        &format!("/containers/{first}/pieces"),
        json!({"title": "Novella", "position": 1, "label": "novella", "shape": "group"}),
    )
    .await;
    created(
        &app,
        &format!("/pieces/{}/children", novel["id"]),
        json!({"title": "Scene", "position": 0, "label": "scene"}),
    )
    .await;
    created(
        &app,
        &format!("/pieces/{}/children", novel["id"]),
        json!({"title": "One", "position": 1}),
    )
    .await;
    let (_, series) = request(&app, "GET", "/series", Value::Null).await;
    let ids: Vec<&Value> = series
        .as_array()
        .unwrap()
        .iter()
        .map(|s| &s["id"])
        .collect();
    assert_eq!(ids, [&json!(first), &json!(second)]);
    assert_eq!(
        status(&app, "GET", &format!("/series/{shelf}"), Value::Null).await,
        StatusCode::NOT_FOUND
    );
    let (_, books) = request(&app, "GET", &format!("/series/{first}/books"), Value::Null).await;
    assert_eq!(books.as_array().unwrap().len(), 1);
    assert_eq!(books[0]["title"], "Novel");
    let book_id = books[0]["id"].as_i64().unwrap();
    let (_, chapters) = request(
        &app,
        "GET",
        &format!("/series/{first}/books/{book_id}/chapters"),
        Value::Null,
    )
    .await;
    assert_eq!(chapters.as_array().unwrap().len(), 1);
    assert_eq!(chapters[0]["title"], "One");

    let entry = created(
        &app,
        &format!("/series/{first}/lore"),
        json!({"kind": "place", "name": "Shared"}),
    )
    .await;
    let (_, from_second) =
        request(&app, "GET", &format!("/series/{second}/lore"), Value::Null).await;
    let mut expected = entry.clone();
    expected["series_id"] = json!(second);
    assert_eq!(from_second, json!([expected]));
    let (_, from_universe) =
        request(&app, "GET", &format!("/universes/{id}/lore"), Value::Null).await;
    assert_eq!(from_universe[0]["universe_id"], id);
    assert_eq!(
        status(&app, "DELETE", &format!("/series/{first}"), Value::Null).await,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(&app, "GET", &format!("/series/{second}/lore"), Value::Null).await,
        (StatusCode::OK, json!([expected]))
    );
    let lone = created(&app, "/series", json!({"title": "Lone", "position": 0})).await;
    assert_eq!(
        request(
            &app,
            "GET",
            &format!("/series/{}/lore", lone["id"]),
            Value::Null
        )
        .await,
        (StatusCode::OK, json!([]))
    );
}
