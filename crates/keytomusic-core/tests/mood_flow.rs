//! End to end: the extension's HTTP calls → engine → an OpenAI-compatible
//! model (mocked here) → the "play this mood" events the session acts on.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::routing::post;
use axum::{Json, Router};
use base64::Engine as _;
use keytomusic_core::mood::provider::{AuthMode, ProviderSettings};
use keytomusic_core::mood::{engine::MoodEngine, server};
use keytomusic_core::types::MoodImageDetail;
use keytomusic_core::CoreEvent;
use serde_json::{json, Value};

/// Pages 1-5 are a fight, 6-10 a gag. Rejects `reasoning_effort` like a
/// non-reasoning model would, to exercise the fallback.
async fn fake_model(Json(body): Json<Value>) -> (axum::http::StatusCode, Json<Value>) {
    if body.get("reasoning_effort").is_some() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(json!({ "error": {
                "message": "Unsupported parameter: 'reasoning_effort' is not supported with this model.",
                "param": "reasoning_effort",
                "code": "unsupported_parameter"
            }})),
        );
    }
    let texts: Vec<String> = body["messages"][1]["content"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|part| part["text"].as_str().map(str::to_string))
        .collect();
    let range = texts
        .iter()
        .find_map(|t| t.strip_prefix("Pages to split into scenes: "))
        .expect("pages to classify")
        .trim_end_matches('.')
        .split(" to ")
        .map(|n| n.parse::<i64>().unwrap())
        .collect::<Vec<_>>();
    let (first, last) = (range[0], range[1]);
    let mut scenes = Vec::new();
    if first <= 5 {
        scenes.push(json!({ "first_page": first, "last_page": last.min(5), "mood": "tension", "intensity": 2, "summary": "fight" }));
    }
    if last >= 6 {
        scenes.push(json!({ "first_page": first.max(6), "last_page": last, "mood": "comedy", "intensity": 1, "summary": "gag" }));
    }
    let content = json!({ "scenes": scenes }).to_string();
    (
        axum::http::StatusCode::OK,
        Json(json!({ "choices": [{ "message": { "role": "assistant", "content": content } }] })),
    )
}

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn page_png() -> String {
    let page = image::RgbImage::from_pixel(600, 900, image::Rgb([240, 240, 240]));
    let mut png = Vec::new();
    image::DynamicImage::ImageRgb8(page)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    base64::engine::general_purpose::STANDARD.encode(png)
}

async fn wait_for(events: &Arc<Mutex<Vec<CoreEvent>>>, mood: &str) -> CoreEvent {
    for _ in 0..200 {
        let found = events.lock().unwrap().iter().find_map(|event| match event {
            CoreEvent::MoodCommitted { mood: m, .. } if m == mood => Some(event.clone()),
            _ => None,
        });
        if let Some(event) = found {
            return event;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("no `{mood}` commit; events: {:?}", events.lock().unwrap());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_chapter_read_in_the_browser_drives_the_music() {
    // The model.
    let model_port = free_port();
    let model = Router::new().route("/v1/chat/completions", post(fake_model));
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", model_port))
        .await
        .unwrap();
    tokio::spawn(async move { axum::serve(listener, model).await.unwrap() });

    // The engine and its API, as the app runs them.
    let events: Arc<Mutex<Vec<CoreEvent>>> = Arc::default();
    let sink = events.clone();
    let engine = Arc::new(MoodEngine::new(None));
    engine.configure(ProviderSettings {
        auth: AuthMode::ApiKey,
        base_url: format!("http://127.0.0.1:{model_port}/v1"),
        model: "gpt-5-mini".into(),
        api_key: None,
        image_detail: MoodImageDetail::High,
    });
    engine.start(Arc::new(move |event: CoreEvent| {
        sink.lock().unwrap().push(event)
    }));
    let api_port = free_port();
    server::start(api_port, engine.clone()).await.unwrap();

    // The extension.
    let http = reqwest::Client::new();
    let url = |path: &str| format!("http://127.0.0.1:{api_port}{path}");
    let chapter = "https://reader.example/series/chapter-12";

    let status: Value = http
        .get(url("/api/status"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(status["app"], "keytomusic");
    assert_eq!(status["configured"], true);

    let opened: Value = http
        .post(url("/api/chapter"))
        .json(&json!({ "chapter": chapter, "title": "Chapitre 12", "totalPages": 10 }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(opened["received"], json!([]));

    let png = page_png();
    let pages: Vec<Value> = (0..10)
        .map(|page| json!({ "page": page, "data": png }))
        .collect();
    let stored: Value = http
        .post(url("/api/pages"))
        .json(&json!({ "chapter": chapter, "pages": pages }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(stored["received"], 10);

    // Reading page 1: the fight plays once analyzed.
    http.post(url("/api/position"))
        .json(&json!({ "chapter": chapter, "page": 0, "totalPages": 10 }))
        .send()
        .await
        .unwrap();
    let event = wait_for(&events, "tension").await;
    assert!(
        matches!(event, CoreEvent::MoodCommitted { intensity: 2, ref summary, .. } if summary == "fight")
    );

    // Same scene: nothing new. Next scene: the gag.
    let before = events.lock().unwrap().len();
    let position: Value = http
        .post(url("/api/position"))
        .json(&json!({ "chapter": chapter, "page": 3 }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(position["current"]["mood"], "tension");
    let commits_after = |from: usize| {
        events.lock().unwrap()[from..]
            .iter()
            .filter(|e| matches!(e, CoreEvent::MoodCommitted { .. }))
            .count()
    };
    assert_eq!(commits_after(before), 0);

    http.post(url("/api/position"))
        .json(&json!({ "chapter": chapter, "page": 7 }))
        .send()
        .await
        .unwrap();
    let event = wait_for(&events, "comedy").await;
    assert!(
        matches!(event, CoreEvent::MoodCommitted { previous_mood: Some(ref p), .. } if p == "tension")
    );

    let status: Value = http
        .get(url("/api/status"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(status["chapter"]["analyzed"], 10);
    assert_eq!(status["chapter"]["title"], "Chapitre 12");
    assert_eq!(status["chapter"]["pages"]["9"]["mood"], "comedy");
}
