//! Local HTTP API used by the browser extension (127.0.0.1 only).
//!
//! - `GET  /api/status`   → is the app ready, which model, the chapter state,
//!   the extension version the app ships
//! - `POST /api/chapter`  → a chapter is open: `{chapter, title?, totalPages?,
//!   url?, pages?: [image URL]}`; with `pages`, the app downloads them itself
//!   (`url` is sent as Referer). Answers `{received, wanted}`.
//! - `POST /api/pages`    → page images the app could not download:
//!   `{chapter, pages: [{page, data}]}` (base64)
//! - `POST /api/position` → the page being read: `{chapter, page, totalPages?}`;
//!   answers the mood of that page and the pages still `wanted` from the extension

use std::sync::Arc;

use axum::extract::{DefaultBodyLimit, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use super::engine::MoodEngine;
use super::image;

/// Bumped when the protocol changes; the extension checks it.
pub const API_VERSION: u32 = 3;
const MAX_BODY: usize = 64 * 1024 * 1024;
/// Page lists longer than this are not chapters.
const MAX_PAGES: usize = 1000;

#[derive(Clone)]
struct Api {
    engine: Arc<MoodEngine>,
}

type ApiError = (StatusCode, Json<Value>);

fn bad_request(message: impl Into<String>) -> ApiError {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": message.into() })),
    )
}

/// Bind the port now (so errors are reported), then serve in the background.
pub async fn start(
    port: u16,
    engine: Arc<MoodEngine>,
) -> Result<tokio::task::JoinHandle<()>, String> {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .map_err(|e| format!("Port {port} indisponible ({e}). Choisis-en un autre."))?;
    let app = Router::new()
        .route("/api/status", get(status_handler))
        .route("/api/chapter", post(chapter_handler))
        .route("/api/pages", post(pages_handler))
        .route("/api/position", post(position_handler))
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .with_state(Api { engine });
    tracing::info!("Mood AI API listening on 127.0.0.1:{port}");
    Ok(tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!("Mood AI API stopped: {e}");
        }
    }))
}

async fn status_handler(State(api): State<Api>) -> Json<Value> {
    api.engine.touch();
    let settings = api.engine.settings();
    Json(json!({
        "app": "keytomusic",
        "apiVersion": API_VERSION,
        "extensionVersion": super::extension::VERSION,
        "enabled": api.engine.is_running(),
        "configured": settings.as_ref().is_some_and(|s| s.is_configured()),
        "model": settings.map(|s| s.model).unwrap_or_default(),
        "error": api.engine.blocking_error(),
        "chapter": api.engine.snapshot(),
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChapterRequest {
    chapter: String,
    title: Option<String>,
    total_pages: Option<u32>,
    /// The reader page, Referer of the downloads.
    url: Option<String>,
    /// Image URL of every page, in reading order.
    pages: Option<Vec<String>>,
}

async fn chapter_handler(
    State(api): State<Api>,
    Json(request): Json<ChapterRequest>,
) -> Result<Json<Value>, ApiError> {
    api.engine.touch();
    if request.chapter.trim().is_empty() {
        return Err(bad_request("chapter is required"));
    }
    let received = api
        .engine
        .open_chapter(&request.chapter, request.title, request.total_pages);
    if let Some(pages) = request.pages.filter(|p| !p.is_empty()) {
        if pages.len() > MAX_PAGES {
            return Err(bad_request("too many pages"));
        }
        api.engine.set_sources(
            &request.chapter,
            pages,
            request.url.or(Some(request.chapter.clone())),
        );
    }
    let wanted = api.engine.snapshot().map(|s| s.wanted).unwrap_or_default();
    Ok(Json(json!({ "received": received, "wanted": wanted })))
}

#[derive(Deserialize)]
struct PageUpload {
    page: u32,
    /// Base64 image (any common format), with or without a `data:` prefix.
    data: String,
}

#[derive(Deserialize)]
struct PagesRequest {
    chapter: String,
    pages: Vec<PageUpload>,
}

async fn pages_handler(
    State(api): State<Api>,
    Json(request): Json<PagesRequest>,
) -> Result<Json<Value>, ApiError> {
    api.engine.touch();
    let decoded = tokio::task::spawn_blocking(move || {
        request
            .pages
            .into_iter()
            .map(|upload| (upload.page, image::prepare_base64(&upload.data)))
            .collect::<Vec<_>>()
    })
    .await
    .map_err(|e| bad_request(e.to_string()))?;

    let mut pages = Vec::new();
    let mut rejected = Vec::new();
    for (page, result) in decoded {
        match result {
            Ok(tiles) => pages.push((page, image::page(tiles))),
            Err(error) => {
                tracing::warn!("Mood AI: page {} rejected: {error}", page + 1);
                rejected.push(page);
            }
        }
    }
    let received = api.engine.add_pages(&request.chapter, pages);
    Ok(Json(json!({ "received": received, "rejected": rejected })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PositionRequest {
    chapter: String,
    page: u32,
    total_pages: Option<u32>,
}

async fn position_handler(
    State(api): State<Api>,
    Json(request): Json<PositionRequest>,
) -> Result<Json<Value>, ApiError> {
    api.engine.touch();
    let snapshot = api
        .engine
        .set_position(&request.chapter, request.page, request.total_pages);
    Ok(Json(json!({
        "received": snapshot.received,
        "analyzed": snapshot.analyzed,
        "total": snapshot.total_pages,
        "current": snapshot.current,
        "wanted": snapshot.wanted,
        "error": snapshot.error,
    })))
}
