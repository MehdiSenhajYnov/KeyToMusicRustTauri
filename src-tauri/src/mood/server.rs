use std::sync::Arc;
use tokio::sync::Mutex;

use super::cache::MoodCache;
use super::inference::{self, LlamaServer};

/// State shared with axum handlers.
pub struct MoodApiState {
    pub llama_server: Arc<Mutex<Option<LlamaServer>>>,
    pub app_handle: tauri::AppHandle,
    pub mood_cache: Arc<std::sync::Mutex<MoodCache>>,
}

/// Start the HTTP API server for external tools.
pub async fn start_api_server(
    port: u16,
    llama_server: Arc<Mutex<Option<LlamaServer>>>,
    app_handle: tauri::AppHandle,
    mood_cache: Arc<std::sync::Mutex<MoodCache>>,
) -> Result<(), String> {
    use axum::routing::{get, post};
    use axum::Router;

    let state = Arc::new(MoodApiState {
        llama_server,
        app_handle,
        mood_cache,
    });

    let app = Router::new()
        .route("/api/analyze", post(analyze_handler))
        .route("/api/trigger", post(trigger_handler))
        .route("/api/lookup", post(lookup_handler))
        .route("/api/cache/status", get(cache_status_handler))
        .route("/api/status", get(status_handler))
        .route("/api/moods", get(moods_handler))
        .with_state(state);

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("Mood API server starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| {
            tracing::error!("Mood API: failed to bind port {}: {}", port, e);
            format!("Failed to bind port {}: {}", port, e)
        })?;

    tracing::info!("Mood API server listening on http://{}", addr);

    axum::serve(listener, app)
        .await
        .map_err(|e| {
            tracing::error!("Mood API server crashed: {}", e);
            format!("Mood API server error: {}", e)
        })?;

    Ok(())
}

#[derive(serde::Deserialize)]
struct AnalyzeRequest {
    image: String, // base64-encoded image
    #[serde(default)]
    precalculate: bool, // true = cache only, no sound trigger
    #[serde(default)]
    chapter: Option<String>, // URL pathname of the chapter
    #[serde(default)]
    page: Option<u32>, // page index within the chapter
}

#[derive(serde::Serialize)]
struct AnalyzeResponse {
    mood: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cached: Option<bool>,
}

#[derive(serde::Serialize)]
struct StatusResponse {
    server: String,
    model: String,
    port: u16,
}

#[derive(serde::Serialize)]
struct ErrorResponse {
    error: String,
    status: String,
}

async fn analyze_handler(
    axum::extract::State(state): axum::extract::State<Arc<MoodApiState>>,
    axum::Json(payload): axum::Json<AnalyzeRequest>,
) -> Result<axum::Json<AnalyzeResponse>, (axum::http::StatusCode, axum::Json<ErrorResponse>)> {
    use base64::Engine;
    use tauri::Emitter;

    tracing::info!(
        "POST /api/analyze — received image ({} bytes base64), precalculate={}, chapter={:?}, page={:?}",
        payload.image.len(),
        payload.precalculate,
        payload.chapter,
        payload.page,
    );

    // Check cache before inference
    if let (Some(chapter), Some(page)) = (&payload.chapter, payload.page) {
        let cache = state.mood_cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(cached_mood) = cache.get(chapter, page) {
            let mood_str = serde_json::to_value(cached_mood)
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_default();

            tracing::info!(
                "POST /api/analyze — cache hit for ({}, {}): {}",
                chapter,
                page,
                mood_str
            );

            if !payload.precalculate {
                let _ = state.app_handle.emit(
                    "mood_detected",
                    serde_json::json!({ "mood": mood_str, "source": "api" }),
                );
            }

            return Ok(axum::Json(AnalyzeResponse {
                mood: mood_str,
                status: "ok".to_string(),
                cached: Some(true),
            }));
        }
    }

    let mut guard = state.llama_server.lock().await;
    let server = guard.as_mut().ok_or_else(|| {
        tracing::error!("POST /api/analyze — llama-server not running");
        (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(ErrorResponse {
                error: "llama-server not running".to_string(),
                status: "error".to_string(),
            }),
        )
    })?;

    // Decode base64 to verify it's valid, then re-encode after resize
    let image_bytes = base64::engine::general_purpose::STANDARD
        .decode(&payload.image)
        .map_err(|e| {
            tracing::error!("POST /api/analyze — invalid base64: {}", e);
            (
                axum::http::StatusCode::BAD_REQUEST,
                axum::Json(ErrorResponse {
                    error: format!("Invalid base64 image: {}", e),
                    status: "error".to_string(),
                }),
            )
        })?;

    tracing::info!(
        "POST /api/analyze — decoded {} bytes, resizing to 672px",
        image_bytes.len()
    );

    let resized_b64 = inference::prepare_image(&image_bytes).map_err(|e| {
        tracing::error!("POST /api/analyze — image processing failed: {}", e);
        (
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(ErrorResponse {
                error: format!("Image processing failed: {}", e),
                status: "error".to_string(),
            }),
        )
    })?;

    tracing::info!("POST /api/analyze — sending to llama-server for inference...");

    let mood = server.analyze_mood(&resized_b64).await.map_err(|e| {
        tracing::error!("POST /api/analyze — inference failed: {}", e);
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(ErrorResponse {
                error: format!("Analysis failed: {}", e),
                status: "error".to_string(),
            }),
        )
    })?;

    let mood_str = serde_json::to_value(&mood)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default();

    tracing::info!("POST /api/analyze — mood detected: {}", mood_str);

    // Store in cache if chapter + page provided
    if let (Some(chapter), Some(page)) = (&payload.chapter, payload.page) {
        let mut cache = state.mood_cache.lock().unwrap_or_else(|e| e.into_inner());
        cache.insert(chapter, page, mood.clone());
        tracing::debug!(
            "POST /api/analyze — cached mood for ({}, {}), cache size: {}",
            chapter,
            page,
            cache.len()
        );
    }

    // Only emit event if not a precalculation request
    if !payload.precalculate {
        let _ = state.app_handle.emit(
            "mood_detected",
            serde_json::json!({ "mood": mood_str, "source": "api" }),
        );
    }

    Ok(axum::Json(AnalyzeResponse {
        mood: mood_str,
        status: "ok".to_string(),
        cached: Some(false),
    }))
}

const VALID_MOODS: &[&str] = &[
    "epic_battle",
    "tension",
    "sadness",
    "comedy",
    "romance",
    "horror",
    "peaceful",
    "emotional_climax",
    "mystery",
    "chase_action",
];

#[derive(serde::Deserialize)]
struct TriggerRequest {
    mood: String,
}

/// Re-emit a previously detected mood without running inference.
async fn trigger_handler(
    axum::extract::State(state): axum::extract::State<Arc<MoodApiState>>,
    axum::Json(payload): axum::Json<TriggerRequest>,
) -> Result<axum::Json<AnalyzeResponse>, (axum::http::StatusCode, axum::Json<ErrorResponse>)> {
    use tauri::Emitter;

    if !VALID_MOODS.contains(&payload.mood.as_str()) {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(ErrorResponse {
                error: format!("Unknown mood: {}", payload.mood),
                status: "error".to_string(),
            }),
        ));
    }

    tracing::info!("POST /api/trigger — re-emitting mood: {}", payload.mood);

    let _ = state.app_handle.emit(
        "mood_detected",
        serde_json::json!({ "mood": payload.mood, "source": "api" }),
    );

    Ok(axum::Json(AnalyzeResponse {
        mood: payload.mood,
        status: "ok".to_string(),
        cached: None,
    }))
}

// ─── Lookup endpoint ─────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct LookupRequest {
    chapter: String,
    page: u32,
}

#[derive(serde::Serialize)]
struct LookupResponse {
    hit: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    mood: Option<String>,
}

/// Look up a pre-calculated mood from the cache.
/// On cache hit, emits `mood_detected` event and returns the mood.
async fn lookup_handler(
    axum::extract::State(state): axum::extract::State<Arc<MoodApiState>>,
    axum::Json(payload): axum::Json<LookupRequest>,
) -> axum::Json<LookupResponse> {
    use tauri::Emitter;

    let cache = state.mood_cache.lock().unwrap_or_else(|e| e.into_inner());
    let cached = cache.get(&payload.chapter, payload.page).cloned();
    drop(cache);

    if let Some(mood) = cached {
        let mood_str = serde_json::to_value(&mood)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();

        tracing::info!(
            "POST /api/lookup — cache hit ({}, {}): {}",
            payload.chapter,
            payload.page,
            mood_str
        );

        let _ = state.app_handle.emit(
            "mood_detected",
            serde_json::json!({ "mood": mood_str, "source": "api" }),
        );

        axum::Json(LookupResponse {
            hit: true,
            mood: Some(mood_str),
        })
    } else {
        tracing::debug!(
            "POST /api/lookup — cache miss ({}, {})",
            payload.chapter,
            payload.page
        );

        axum::Json(LookupResponse {
            hit: false,
            mood: None,
        })
    }
}

// ─── Cache status endpoint ───────────────────────────────────────────────────

#[derive(serde::Serialize)]
struct CacheStatusResponse {
    entries: usize,
    chapter: Option<String>,
}

async fn cache_status_handler(
    axum::extract::State(state): axum::extract::State<Arc<MoodApiState>>,
) -> axum::Json<CacheStatusResponse> {
    let cache = state.mood_cache.lock().unwrap_or_else(|e| e.into_inner());
    let entries = cache.len();
    let chapter = cache.current_chapter().map(|s| s.to_string());
    drop(cache);

    tracing::debug!(
        "GET /api/cache/status — entries={}, chapter={:?}",
        entries,
        chapter
    );

    axum::Json(CacheStatusResponse { entries, chapter })
}

// ─── Status & moods ──────────────────────────────────────────────────────────

async fn status_handler(
    axum::extract::State(state): axum::extract::State<Arc<MoodApiState>>,
) -> axum::Json<StatusResponse> {
    tracing::debug!("GET /api/status");

    let guard = state.llama_server.lock().await;
    let (server_status, model_status, port) = match guard.as_ref() {
        Some(server) => ("running".to_string(), "loaded".to_string(), server.port),
        None => ("stopped".to_string(), "not_loaded".to_string(), 0),
    };

    tracing::debug!(
        "GET /api/status — server={}, model={}, port={}",
        server_status,
        model_status,
        port
    );

    axum::Json(StatusResponse {
        server: server_status,
        model: model_status,
        port,
    })
}

async fn moods_handler() -> axum::Json<Vec<&'static str>> {
    tracing::debug!("GET /api/moods");

    axum::Json(vec![
        "epic_battle",
        "tension",
        "sadness",
        "comedy",
        "romance",
        "horror",
        "peaceful",
        "emotional_climax",
        "mystery",
        "chase_action",
    ])
}
