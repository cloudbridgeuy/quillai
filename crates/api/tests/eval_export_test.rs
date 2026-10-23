use quillai_api::eval::{
    cases::{parse_cases, Action},
    export::export,
    fixture::{Fixture, FIXTURE_FILE},
};
use serde_json::{json, Value};

mod common;

use common::test_pool;

async fn serve() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = quillai_api::app(test_pool().await);
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{address}")
}

async fn post(client: &reqwest::Client, url: &str, body: Value) -> Value {
    let response = client.post(url).json(&body).send().await.unwrap();
    assert!(
        response.status().is_success(),
        "{url}: {}",
        response.status()
    );
    response.json().await.unwrap()
}

async fn seed(api: &str) -> (i64, i64) {
    let client = reqwest::Client::new();
    let series = post(
        &client,
        &format!("{api}/series"),
        json!({"title": "S", "position": 0}),
    )
    .await;
    let books = format!("{api}/series/{}/books", series["id"]);
    post(&client, &books, json!({"title": "Other", "position": 1})).await;
    let book = post(&client, &books, json!({"title": "B", "position": 0})).await;
    let chapters = format!("{books}/{}/chapters", book["id"]);
    let second = post(
        &client,
        &chapters,
        json!({"title": "Two", "position": 1, "kind": "chapter",
            "content": {"ops": [{"insert": "Hi 😀"}, {"insert": {"image": "x"}}, {"insert": "end\n", "attributes": {"bold": true}}]}}),
    )
    .await;
    post(
        &client,
        &chapters,
        json!({"title": "Prologue", "position": 0, "kind": "prologue",
            "content": {"ops": [{"insert": "Start\n"}]}}),
    )
    .await;
    post(
        &client,
        &format!("{chapters}/{}/deltas", second["id"]),
        json!({"baseRev": 0, "delta": {"ops": [{"insert": "Oh. "}]}}),
    )
    .await;
    (series["id"].as_i64().unwrap(), book["id"].as_i64().unwrap())
}

fn out_dir(name: &str) -> std::path::PathBuf {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[tokio::test]
async fn export_writes_rebuilt_chapters_in_api_order() {
    let api = serve().await;
    let (series, book) = seed(&api).await;
    let out = out_dir("eval_export_round_trip");

    let path = export(&api, series, book, &out).await.unwrap();

    assert_eq!(path, out.join(FIXTURE_FILE));
    let fixture: Fixture = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let api_chapters: Value = reqwest::get(format!(
        "{api}/series/{series}/books/{book}/chapters?content=true"
    ))
    .await
    .unwrap()
    .json()
    .await
    .unwrap();
    assert_eq!((fixture.series_id, fixture.book_id), (series, book));
    let written: Vec<_> = fixture
        .chapters
        .iter()
        .map(|chapter| {
            json!({"id": chapter.id, "title": chapter.title, "position": chapter.position,
                "kind": chapter.kind, "head_rev": chapter.head_rev,
                "content": serde_json::to_value(&chapter.content).unwrap()})
        })
        .collect();
    let expected: Vec<_> = api_chapters
        .as_array()
        .unwrap()
        .iter()
        .map(|chapter| {
            json!({"id": chapter["id"], "title": chapter["title"], "position": chapter["position"],
                "kind": chapter["kind"], "head_rev": chapter["head_rev"], "content": chapter["content"]})
        })
        .collect();
    assert_eq!(written, expected);
    let titles: Vec<_> = fixture.chapters.iter().map(|c| c.title.as_str()).collect();
    assert_eq!(titles, ["Prologue", "Two"]);
    assert_eq!(fixture.chapters[1].head_rev, 1);
    assert_eq!(fixture.chapters[0].text, "Start\n");
    assert_eq!(fixture.chapters[1].text, "Oh. Hi 😀end\n");

    let cases = parse_cases(
        &json!([{"chapter": fixture.chapters[1].id, "start": 10, "end": 13, "action": "edit", "instruction": "Fix"}])
            .to_string(),
        &fixture,
    )
    .unwrap();
    assert_eq!(
        cases[0].action,
        Action::Edit {
            start: 10,
            end: 13,
            instruction: "Fix".into()
        }
    );
}

#[tokio::test]
async fn export_refuses_a_path_git_does_not_ignore() {
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("eval-not-ignored");

    let error = export("http://127.0.0.1:9", 1, 1, &out).await.unwrap_err();

    assert!(
        error.to_string().contains("git does not ignore it"),
        "{error}"
    );
    assert!(!out.exists());
}

#[tokio::test]
async fn export_reports_missing_book() {
    let api = serve().await;
    let out = out_dir("eval_export_missing");

    let error = export(&api, 99, 99, &out).await.unwrap_err();

    assert!(error.to_string().contains("404"), "{error}");
    assert!(!out.exists());
}
