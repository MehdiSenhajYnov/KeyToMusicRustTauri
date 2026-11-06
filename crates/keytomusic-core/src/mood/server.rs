//! Local HTTP API used by the browser extension (127.0.0.1 only).
//!
//! - `GET  /api/status`   → is the app ready, which model, the chapter state
//! - `POST /api/chapter`  → a chapter is open: `{chapter, title?, totalPages?}`
//! - `POST /api/pages`    → page images: `{chapter, pages: [{page, data}]}` (base64)
//! - `POST /api/position` → the page being read: `{chapter, page, totalPages?}`

use std::sync::Arc;

use axum::extract::{DefaultBodyLimit, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine as _;
use serde::Deserialize;
use serde_json::{json, Value};

use super::engine::MoodEngine;

/// Bumped when the protocol changes; the extension checks it.
pub const API_VERSION: u32 = 2;
const MAX_BODY: usize = 64 * 1024 * 1024;
/// Width the model needs to read the bubbles; taller strips keep their height.
const MAX_WIDTH: u32 = 900;
const MAX_HEIGHT: u32 = 2400;

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
    let settings = api.engine.settings();
    Json(json!({
        "app": "keytomusic",
        "apiVersion": API_VERSION,
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
}

async fn chapter_handler(
    State(api): State<Api>,
    Json(request): Json<ChapterRequest>,
) -> Result<Json<Value>, ApiError> {
    if request.chapter.trim().is_empty() {
        return Err(bad_request("chapter is required"));
    }
    let received = api
        .engine
        .open_chapter(&request.chapter, request.title, request.total_pages);
    Ok(Json(json!({ "received": received })))
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
    let decoded = tokio::task::spawn_blocking(move || {
        request
            .pages
            .into_iter()
            .map(|upload| (upload.page, prepare_image(&upload.data)))
            .collect::<Vec<_>>()
    })
    .await
    .map_err(|e| bad_request(e.to_string()))?;

    let mut pages = Vec::new();
    let mut rejected = Vec::new();
    for (page, result) in decoded {
        match result {
            Ok(image) => pages.push((page, Arc::new(image))),
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
    let snapshot = api
        .engine
        .set_position(&request.chapter, request.page, request.total_pages);
    Ok(Json(json!({
        "received": snapshot.received,
        "analyzed": snapshot.analyzed,
        "total": snapshot.total_pages,
        "current": snapshot.current,
        "error": snapshot.error,
    })))
}

/// Decode, shrink to what the model needs, re-encode as base64 JPEG.
pub fn prepare_image(data: &str) -> Result<String, String> {
    let b64 = match data.split_once(";base64,") {
        Some((_, rest)) => rest,
        None => data,
    };
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .map_err(|e| format!("base64 invalide : {e}"))?;
    let image = image::load_from_memory(&bytes).map_err(|e| format!("image illisible : {e}"))?;
    let (width, height) = (image.width(), image.height());
    if width < 32 || height < 32 {
        return Err("image trop petite".into());
    }
    let scale = (MAX_WIDTH as f32 / width as f32)
        .min(MAX_HEIGHT as f32 / height as f32)
        .min(1.0);
    let image = if scale < 1.0 {
        image.resize(
            ((width as f32 * scale).round() as u32).max(1),
            ((height as f32 * scale).round() as u32).max(1),
            image::imageops::FilterType::Triangle,
        )
    } else {
        image
    };
    let mut jpeg = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 82);
    image
        .to_rgb8()
        .write_with_encoder(encoder)
        .map_err(|e| format!("encodage JPEG impossible : {e}"))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(jpeg))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_pages_are_shrunk_to_jpeg() {
        let page = image::RgbImage::from_pixel(1800, 2600, image::Rgb([200, 200, 200]));
        let mut png = Vec::new();
        image::DynamicImage::ImageRgb8(page)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let data = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&png)
        );
        let out = prepare_image(&data).unwrap();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(out)
            .unwrap();
        let back = image::load_from_memory(&bytes).unwrap();
        assert_eq!(back.width(), 900);
        assert_eq!(back.height(), 1300);
        assert!(prepare_image("not base64!").is_err());
    }
}
