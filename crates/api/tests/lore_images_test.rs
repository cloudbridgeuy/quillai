mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::Response,
    Router,
};
use http_body_util::BodyExt;
use quillai_api::ai::catalog::ModelCatalog;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
};
use tower::ServiceExt;

async fn send(app: &Router, method: &str, uri: &str, body: Value) -> Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn json_of(response: Response, expected: StatusCode) -> Value {
    assert_eq!(response.status(), expected);
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}

async fn id_of(app: &Router, uri: &str, body: Value) -> i64 {
    json_of(send(app, "POST", uri, body).await, StatusCode::CREATED).await["id"]
        .as_i64()
        .unwrap()
}

async fn mock_images(status: u16, body: &'static str) -> (String, oneshot::Receiver<Value>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (sender, receiver) = oneshot::channel();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut raw = Vec::new();
        let mut buffer = [0_u8; 8192];
        loop {
            let read = socket.read(&mut buffer).await.unwrap();
            raw.extend_from_slice(&buffer[..read]);
            let text = String::from_utf8_lossy(&raw).to_string();
            if let Some((head, rest)) = text.split_once("\r\n\r\n") {
                let length: usize = head
                    .lines()
                    .find_map(|line| {
                        line.to_lowercase()
                            .strip_prefix("content-length:")
                            .map(|v| v.trim().parse().unwrap())
                    })
                    .unwrap_or(0);
                if rest.len() >= length {
                    assert!(head.starts_with("POST /v1/images/generations "), "{head}");
                    assert!(
                        head.to_lowercase()
                            .contains("authorization: bearer image-key"),
                        "{head}"
                    );
                    let _ = sender.send(serde_json::from_str(rest).unwrap());
                    break;
                }
            }
            if read == 0 {
                break;
            }
        }
        let response = format!(
            "HTTP/1.1 {status} OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        socket.write_all(response.as_bytes()).await.unwrap();
    });
    (format!("http://{addr}/v1"), receiver)
}

fn catalog() -> ModelCatalog {
    ModelCatalog::parse(&json!({"models":[
        {"id":"text-model","label":"Text","input_price_per_million":0,"output_price_per_million":0},
        {"id":"gpt-image-1-mini","label":"Image","kind":"image","price_per_image":0.011,"size":"1024x1024","quality":"medium"}
    ],"defaults":{"generate":"text-model","edit":"text-model","image":"gpt-image-1-mini"}}).to_string()).unwrap()
}

struct World {
    app: Router,
    images: String,
    point: i64,
    lore: String,
}

async fn world(base_url: &str, image_model: &str) -> World {
    let app =
        quillai_api::app(common::test_pool().await).layer(axum::Extension(Arc::new(catalog())));
    let settings =
        json!({"base_url": base_url, "api_key": "image-key", "image_model": image_model});
    json_of(
        send(&app, "PUT", "/settings/ai", settings).await,
        StatusCode::OK,
    )
    .await;
    let universe = id_of(&app, "/universes", json!({"title":"World","position":0})).await;
    let entry = id_of(
        &app,
        &format!("/universes/{universe}/lore"),
        json!({"kind":"character","name":"Mara","visible_text":"A tall smuggler.","hidden_notes":"SECRET twin"}),
    )
    .await;
    let point = id_of(
        &app,
        &format!("/universes/{universe}/timeline"),
        json!({"name":"War","position":10}),
    )
    .await;
    id_of(
        &app,
        &format!("/universes/{universe}/lore/{entry}/states"),
        json!({"name":"Scarred","pointId":point,"visibleText":"Scarred and grey.","hiddenNotes":"SECRET state"}),
    )
    .await;
    World {
        app,
        images: format!("/universes/{universe}/lore/{entry}/images"),
        point,
        lore: format!("/universes/{universe}/lore"),
    }
}

async fn place_images(world: &World, level: &str) -> String {
    let place = id_of(
        &world.app,
        &world.lore,
        json!({"kind":"place","name":"Harbor","visible_text":"Salt docks.","hidden_notes":"SECRET tunnel","level":level}),
    )
    .await;
    format!("{}/{place}/images", world.lore)
}

#[tokio::test]
async fn place_maps_and_renders_keep_hidden_notes_out_and_reject_leave_out() {
    let (base_url, mut unused) =
        mock_images(200, r#"{"data":[{"b64_json":"UklGRmltYWdl"}]}"#).await;
    let world = world(&base_url, "gpt-image-1-mini").await;
    let place = place_images(&world, "include").await;
    let hidden = place_images(&world, "leave_out").await;
    for (uri, body) in [
        (&hidden, json!({"purpose":"map"})),
        (&hidden, json!({"purpose":"render","timeOfDay":"dusk"})),
        (&world.images, json!({"purpose":"map"})),
        (&world.images, json!({"purpose":"render"})),
        (&place, json!({"purpose":"map","timeOfDay":"dusk"})),
        (&place, json!({"timeOfDay":"night"})),
    ] {
        let response = send(&world.app, "POST", uri, body.clone()).await;
        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{uri} {body}"
        );
    }
    assert!(matches!(
        unused.try_recv(),
        Err(oneshot::error::TryRecvError::Empty)
    ));
    let mut created = Vec::new();
    for (body, preamble) in [
        (json!({"purpose":"map"}), "Top-down illustrated map."),
        (
            json!({"purpose":"render","timeOfDay":"dusk"}),
            "Establishing view.",
        ),
    ] {
        let (base_url, request) =
            mock_images(200, r#"{"data":[{"b64_json":"UklGRmltYWdl"}]}"#).await;
        json_of(
            send(
                &world.app,
                "PUT",
                "/settings/ai",
                json!({"base_url": base_url}),
            )
            .await,
            StatusCode::OK,
        )
        .await;
        let image = json_of(
            send(&world.app, "POST", &place, body).await,
            StatusCode::CREATED,
        )
        .await;
        let prompt = request.await.unwrap()["prompt"]
            .as_str()
            .unwrap()
            .to_owned();
        assert!(
            prompt.starts_with(preamble)
                && prompt.contains("Salt docks.")
                && !prompt.contains("SECRET"),
            "{prompt}"
        );
        created.push(image);
    }
    assert_eq!(created[0]["purpose"], "map");
    assert_eq!(created[0]["timeOfDay"], Value::Null);
    assert_eq!(created[1]["purpose"], "render");
    assert_eq!(created[1]["timeOfDay"], "dusk");
    assert!(created[1]["prompt"]
        .as_str()
        .unwrap()
        .contains("Time of day: dusk."));
    let listed = json_of(
        send(&world.app, "GET", &place, json!({})).await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(listed, json!(created));
}

#[tokio::test]
async fn generates_stores_lists_and_serves_a_lore_image() {
    let (base_url, request) = mock_images(200, r#"{"data":[{"b64_json":"UklGRmltYWdl"}]}"#).await;
    let world = world(&base_url, "gpt-image-1-mini").await;
    let created = json_of(
        send(
            &world.app,
            "POST",
            &world.images,
            json!({"timelinePointId": world.point}),
        )
        .await,
        StatusCode::CREATED,
    )
    .await;
    let sent = request.await.unwrap();
    assert_eq!(sent["model"], "gpt-image-1-mini");
    assert_eq!(sent["size"], "1024x1024");
    assert_eq!(sent["quality"], "medium");
    assert_eq!(sent["output_format"], "webp");
    assert!(sent.get("response_format").is_none());
    let prompt = sent["prompt"].as_str().unwrap();
    assert!(
        prompt.contains("Scarred and grey.") && !prompt.contains("SECRET"),
        "{prompt}"
    );
    assert_eq!(created["prompt"], prompt);
    assert_eq!(created["timelinePointId"], world.point);
    assert_eq!(created["purpose"], "portrait");
    assert_eq!(created["timeOfDay"], Value::Null);
    assert_eq!(created["mime"], "image/webp");
    assert_eq!(created["cost"], 0.011);
    assert!(created.get("bytes").is_none());
    let listed = json_of(
        send(&world.app, "GET", &world.images, json!({})).await,
        StatusCode::OK,
    )
    .await;
    assert_eq!(listed, json!([created]));
    let image = send(
        &world.app,
        "GET",
        &format!("{}/{}", world.images, created["id"]),
        json!({}),
    )
    .await;
    assert_eq!(image.status(), StatusCode::OK);
    assert_eq!(image.headers()["content-type"], "image/webp");
    assert_eq!(
        &image.into_body().collect().await.unwrap().to_bytes()[..],
        b"RIFFimage"
    );
}

#[tokio::test]
async fn missing_model_wrong_ids_and_provider_errors() {
    let (base_url, _request) = mock_images(400, r#"{"error":{"message":"bad size"}}"#).await;
    let world = world(&base_url, "").await;
    let unset = json_of(
        send(&world.app, "POST", &world.images, json!({})).await,
        StatusCode::SERVICE_UNAVAILABLE,
    )
    .await;
    assert_eq!(unset["code"], "AI_NOT_CONFIGURED");
    assert!(
        unset["error"]
            .as_str()
            .unwrap()
            .contains("QUILLAI_AI_IMAGE_MODEL"),
        "{unset}"
    );
    for uri in [
        "/universes/999/lore/1/images",
        &format!("{}/999", world.images),
    ] {
        assert_eq!(
            send(&world.app, "GET", uri, json!({})).await.status(),
            StatusCode::NOT_FOUND,
            "{uri}"
        );
    }
    json_of(
        send(
            &world.app,
            "PUT",
            "/settings/ai",
            json!({"image_model":"gpt-image-1-mini"}),
        )
        .await,
        StatusCode::OK,
    )
    .await;
    let failed = json_of(
        send(&world.app, "POST", &world.images, json!({})).await,
        StatusCode::INTERNAL_SERVER_ERROR,
    )
    .await;
    assert!(
        failed["error"].as_str().unwrap().contains("bad size"),
        "{failed}"
    );
    assert_eq!(
        json_of(
            send(&world.app, "GET", &world.images, json!({})).await,
            StatusCode::OK
        )
        .await,
        json!([])
    );
}
