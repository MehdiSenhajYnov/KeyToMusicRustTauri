use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use super::cache::CachedMoodSource;
use super::cache::MoodCache;
use super::chapter_pipeline::{
    analyze_visible_window, execute_action, ActionPage, ChapterMoodPipeline, PipelineActivity,
    PublishedPageUpdate,
};
use super::director::MoodScores;
use super::inference::{self, LlamaServer, PageFeatures};
use crate::types::MoodCategory;

/// State shared with axum handlers.
pub struct MoodApiState {
    pub llama_server: Arc<Mutex<Option<LlamaServer>>>,
    pub app_handle: tauri::AppHandle,
    pub mood_cache: Arc<std::sync::Mutex<MoodCache>>,
    pub chapter_pipeline: Arc<Mutex<ChapterMoodPipeline>>,
    pub live_requests: Arc<AtomicUsize>,
    pub active_live_cancel: Arc<Mutex<Option<LiveCancelHandle>>>,
    pub live_cancel_seq: Arc<AtomicUsize>,
}

#[derive(Clone)]
pub struct LiveCancelHandle {
    pub id: usize,
    pub request_id: usize,
    pub token: CancellationToken,
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
        chapter_pipeline: Arc::new(Mutex::new(ChapterMoodPipeline::new())),
        live_requests: Arc::new(AtomicUsize::new(0)),
        active_live_cancel: Arc::new(Mutex::new(None)),
        live_cancel_seq: Arc::new(AtomicUsize::new(0)),
    });

    let app = Router::new()
        .route("/api/analyze-window", post(analyze_window_handler))
        .route("/api/chapter/page", post(chapter_page_handler))
        .route("/api/chapter/focus", post(chapter_focus_handler))
        .route("/api/live/cancel", post(cancel_live_handler))
        .route("/api/trigger", post(trigger_handler))
        .route("/api/lookup", post(lookup_handler))
        .route("/api/cache/status", get(cache_status_handler))
        .route("/api/status", get(status_handler))
        .route("/api/moods", get(moods_handler))
        .with_state(state);

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("Mood API server starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
        tracing::error!("Mood API: failed to bind port {}: {}", port, e);
        format!("Failed to bind port {}: {}", port, e)
    })?;

    tracing::info!("Mood API server listening on http://{}", addr);

    axum::serve(listener, app).await.map_err(|e| {
        tracing::error!("Mood API server crashed: {}", e);
        format!("Mood API server error: {}", e)
    })?;

    Ok(())
}

#[derive(serde::Serialize)]
struct AnalyzeResponse {
    mood: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cached: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    committed_mood: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mood_changed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scores: Option<std::collections::HashMap<String, f32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    narrative_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dwell_count: Option<u32>,
}

#[derive(serde::Deserialize)]
struct AnalyzeWindowMemberRequest {
    page: u32,
    image: String,
}

#[derive(serde::Deserialize)]
struct AnalyzeWindowRequest {
    request_id: usize,
    chapter: String,
    page: u32,
    total_pages: u32,
    members: Vec<AnalyzeWindowMemberRequest>,
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

/// Convert MoodScores to a HashMap for JSON serialization.
fn scores_to_map(scores: &MoodScores) -> std::collections::HashMap<String, f32> {
    MoodCategory::ALL
        .iter()
        .map(|m| (m.as_str().to_string(), scores.get(*m)))
        .collect()
}

/// Helper to create an error response tuple.
fn err_response(
    status: axum::http::StatusCode,
    msg: String,
) -> (axum::http::StatusCode, axum::Json<ErrorResponse>) {
    (
        status,
        axum::Json(ErrorResponse {
            error: msg,
            status: "error".to_string(),
        }),
    )
}

fn publish_pipeline_updates(
    cache: &std::sync::Mutex<MoodCache>,
    updates: Vec<PublishedPageUpdate>,
) {
    if updates.is_empty() {
        return;
    }
    let mut guard = cache.lock().unwrap_or_else(|e| e.into_inner());
    for update in updates {
        guard.insert(
            &update.chapter,
            update.page,
            update.mood,
            update.intensity,
            update.scores,
            update.narrative_role,
            update.source,
            update.finalized,
        );
    }
}

async fn spawn_pipeline_worker_if_needed(state: Arc<MoodApiState>) {
    let should_spawn = {
        let mut pipeline = state.chapter_pipeline.lock().await;
        pipeline.start_processing_if_idle()
    };
    if !should_spawn {
        return;
    }

    tokio::spawn(async move {
        loop {
            while state.live_requests.load(Ordering::Relaxed) > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }

            let request = {
                let mut pipeline = state.chapter_pipeline.lock().await;
                pipeline.next_action()
            };

            let Some(request) = request else {
                let mut pipeline = state.chapter_pipeline.lock().await;
                pipeline.finish_processing();
                break;
            };

            let result = {
                let mut guard = state.llama_server.lock().await;
                let Some(server) = guard.as_mut() else {
                    let mut pipeline = state.chapter_pipeline.lock().await;
                    pipeline.commit_failure(&request);
                    pipeline.finish_processing();
                    break;
                };
                execute_action(server, &request).await
            };

            match result {
                Ok(result) => {
                    let updates = {
                        let mut pipeline = state.chapter_pipeline.lock().await;
                        pipeline.commit_success(&request, result)
                    };
                    publish_pipeline_updates(&state.mood_cache, updates);
                }
                Err(err) => {
                    tracing::warn!("Chapter pipeline step failed: {}", err);
                    let mut pipeline = state.chapter_pipeline.lock().await;
                    pipeline.commit_failure(&request);
                    pipeline.record_error(err.clone());
                    pipeline.finish_processing();
                    break;
                }
            }

            tokio::task::yield_now().await;
        }
    });
}

async fn register_live_cancel(state: &Arc<MoodApiState>, request_id: usize) -> LiveCancelHandle {
    let handle = LiveCancelHandle {
        id: state.live_cancel_seq.fetch_add(1, Ordering::Relaxed) + 1,
        request_id,
        token: CancellationToken::new(),
    };

    let previous = {
        let mut guard = state.active_live_cancel.lock().await;
        guard.replace(handle.clone())
    };
    if let Some(previous) = previous {
        previous.token.cancel();
    }

    handle
}

async fn clear_live_cancel_if_current(state: &Arc<MoodApiState>, id: usize) {
    let mut guard = state.active_live_cancel.lock().await;
    if guard.as_ref().map(|handle| handle.id) == Some(id) {
        *guard = None;
    }
}

#[derive(serde::Deserialize)]
struct CancelLiveRequest {
    request_id: usize,
}

struct LiveRequestGuard {
    counter: Arc<AtomicUsize>,
}

impl LiveRequestGuard {
    fn new(counter: Arc<AtomicUsize>) -> Self {
        counter.fetch_add(1, Ordering::Relaxed);
        Self { counter }
    }
}

impl Drop for LiveRequestGuard {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::Relaxed);
    }
}

#[derive(serde::Serialize)]
struct CancelLiveResponse {
    status: String,
    cancelled: bool,
}

async fn cancel_live_handler(
    axum::extract::State(state): axum::extract::State<Arc<MoodApiState>>,
    axum::Json(payload): axum::Json<CancelLiveRequest>,
) -> axum::Json<CancelLiveResponse> {
    let cancelled = {
        let guard = state.active_live_cancel.lock().await;
        if let Some(handle) = guard.as_ref() {
            if handle.request_id == payload.request_id {
                handle.token.cancel();
                true
            } else {
                false
            }
        } else {
            false
        }
    };

    axum::Json(CancelLiveResponse {
        status: "ok".to_string(),
        cancelled,
    })
}

#[derive(serde::Deserialize)]
struct ChapterPageRequest {
    chapter: String,
    page: u32,
    image: String,
    #[serde(default)]
    total_pages: Option<u32>,
}

#[derive(serde::Serialize)]
struct ChapterPageResponse {
    status: String,
    queued: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    mood: Option<String>,
}

#[derive(serde::Deserialize)]
struct ChapterFocusRequest {
    chapter: String,
    page: u32,
    #[serde(default)]
    direction: i8,
    #[serde(default)]
    total_pages: Option<u32>,
}

#[derive(serde::Serialize)]
struct ChapterFocusResponse {
    status: String,
}

async fn chapter_page_handler(
    axum::extract::State(state): axum::extract::State<Arc<MoodApiState>>,
    axum::Json(payload): axum::Json<ChapterPageRequest>,
) -> Result<axum::Json<ChapterPageResponse>, (axum::http::StatusCode, axum::Json<ErrorResponse>)> {
    use base64::Engine;

    tracing::debug!(
        "POST /api/chapter/page — chapter={}, page={}, total_pages={:?}",
        payload.chapter,
        payload.page,
        payload.total_pages
    );

    let image_bytes = base64::engine::general_purpose::STANDARD
        .decode(&payload.image)
        .map_err(|e| {
            err_response(
                axum::http::StatusCode::BAD_REQUEST,
                format!("Invalid base64 image: {}", e),
            )
        })?;

    {
        let mut pipeline = state.chapter_pipeline.lock().await;
        pipeline.register_page(
            &payload.chapter,
            payload.page,
            payload.total_pages,
            image_bytes,
        );
    }

    spawn_pipeline_worker_if_needed(state.clone()).await;

    let mood = {
        let cache = state.mood_cache.lock().unwrap_or_else(|e| e.into_inner());
        cache
            .get(&payload.chapter, payload.page)
            .map(|entry| entry.mood.as_str().to_string())
    };

    Ok(axum::Json(ChapterPageResponse {
        status: "queued".to_string(),
        queued: true,
        mood,
    }))
}

async fn chapter_focus_handler(
    axum::extract::State(state): axum::extract::State<Arc<MoodApiState>>,
    axum::Json(payload): axum::Json<ChapterFocusRequest>,
) -> Result<axum::Json<ChapterFocusResponse>, (axum::http::StatusCode, axum::Json<ErrorResponse>)> {
    tracing::debug!(
        "POST /api/chapter/focus — chapter={}, page={}, direction={}, total_pages={:?}",
        payload.chapter,
        payload.page,
        payload.direction,
        payload.total_pages
    );

    {
        let mut pipeline = state.chapter_pipeline.lock().await;
        pipeline.update_focus(
            &payload.chapter,
            payload.page,
            payload.direction,
            payload.total_pages,
        );
    }

    spawn_pipeline_worker_if_needed(state.clone()).await;

    Ok(axum::Json(ChapterFocusResponse {
        status: "queued".to_string(),
    }))
}

async fn analyze_window_handler(
    axum::extract::State(state): axum::extract::State<Arc<MoodApiState>>,
    axum::Json(payload): axum::Json<AnalyzeWindowRequest>,
) -> Result<axum::Json<AnalyzeResponse>, (axum::http::StatusCode, axum::Json<ErrorResponse>)> {
    use base64::Engine;

    tracing::info!(
        "POST /api/analyze-window — chapter={}, page={}, members={}",
        payload.chapter,
        payload.page,
        payload.members.len()
    );

    if payload.members.is_empty() {
        return Err(err_response(
            axum::http::StatusCode::BAD_REQUEST,
            "Visible window members are required".to_string(),
        ));
    }

    if let Some(entry) = {
        let cache = state.mood_cache.lock().unwrap_or_else(|e| e.into_inner());
        cache.get(&payload.chapter, payload.page).cloned()
    } {
        return Ok(axum::Json(AnalyzeResponse {
            mood: entry.mood.as_str().to_string(),
            status: "ok".to_string(),
            cached: Some(true),
            committed_mood: Some(entry.mood.as_str().to_string()),
            mood_changed: Some(true),
            scores: Some(scores_to_map(&entry.scores)),
            narrative_role: Some(entry.narrative_role.as_str().to_string()),
            dwell_count: None,
        }));
    }

    let mut members = payload
        .members
        .iter()
        .map(|member| {
            let raw_bytes = base64::engine::general_purpose::STANDARD
                .decode(&member.image)
                .map_err(|e| format!("Invalid base64 for page {}: {}", member.page, e))?;
            Ok(ActionPage {
                page: member.page,
                raw_bytes,
            })
        })
        .collect::<Result<Vec<_>, String>>()
        .map_err(|e| err_response(axum::http::StatusCode::BAD_REQUEST, e))?;

    members.sort_by_key(|member| member.page);

    let _live_request = LiveRequestGuard::new(state.live_requests.clone());
    let live_cancel = register_live_cancel(&state, payload.request_id).await;
    let prediction = tokio::select! {
        _ = live_cancel.token.cancelled() => Err(err_response(
            axum::http::StatusCode::CONFLICT,
            "live request cancelled".to_string(),
        )),
        prediction = async {
            let mut guard = state.llama_server.lock().await;
            let server = guard.as_mut().ok_or_else(|| {
                err_response(
                    axum::http::StatusCode::SERVICE_UNAVAILABLE,
                    "llama-server not running".to_string(),
                )
            })?;
            analyze_visible_window(
                server,
                payload.page,
                payload.total_pages.max(payload.page + 1),
                members,
            )
            .await
            .map_err(|e| err_response(axum::http::StatusCode::INTERNAL_SERVER_ERROR, e))
        } => prediction,
    };
    clear_live_cancel_if_current(&state, live_cancel.id).await;
    let prediction = prediction?;

    {
        let mut cache = state.mood_cache.lock().unwrap_or_else(|e| e.into_inner());
        cache.insert(
            &payload.chapter,
            payload.page,
            prediction.mood,
            prediction.intensity,
            prediction.scores.clone(),
            prediction.narrative_role,
            CachedMoodSource::VisibleWindowAnalyze,
            true,
        );
    }

    Ok(axum::Json(AnalyzeResponse {
        mood: prediction.mood.as_str().to_string(),
        status: "ok".to_string(),
        cached: Some(false),
        committed_mood: Some(prediction.mood.as_str().to_string()),
        mood_changed: Some(true),
        scores: Some(scores_to_map(&prediction.scores)),
        narrative_role: Some(prediction.narrative_role.as_str().to_string()),
        dwell_count: None,
    }))
}

const VALID_MOODS: &[&str] = &[
    "epic", "tension", "sadness", "comedy", "romance", "horror", "peaceful", "mystery",
];

#[derive(serde::Deserialize)]
struct TriggerRequest {
    mood: String,
}

/// Re-emit a previously detected mood without running inference.
/// Bypasses the director — directly emits mood_committed for immediate playback.
async fn trigger_handler(
    axum::extract::State(state): axum::extract::State<Arc<MoodApiState>>,
    axum::Json(payload): axum::Json<TriggerRequest>,
) -> Result<axum::Json<AnalyzeResponse>, (axum::http::StatusCode, axum::Json<ErrorResponse>)> {
    use tauri::Emitter;

    if !VALID_MOODS.contains(&payload.mood.as_str()) {
        return Err(err_response(
            axum::http::StatusCode::BAD_REQUEST,
            format!("Unknown mood: {}", payload.mood),
        ));
    }

    tracing::info!("POST /api/trigger — re-emitting mood: {}", payload.mood);

    let _ = state.app_handle.emit(
        "mood_detected",
        serde_json::json!({ "mood": payload.mood, "source": "api" }),
    );

    // Trigger also emits mood_committed for immediate playback
    let _ = state.app_handle.emit(
        "mood_committed",
        serde_json::json!({ "mood": payload.mood, "source": "api" }),
    );

    Ok(axum::Json(AnalyzeResponse {
        mood: payload.mood,
        status: "ok".to_string(),
        cached: None,
        committed_mood: None,
        mood_changed: None,
        scores: None,
        narrative_role: None,
        dwell_count: None,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    finalized: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<String>,
}

/// Look up a cached mood from the local chapter/page cache.
/// This endpoint is side-effect free; the caller decides whether to trigger playback.
async fn lookup_handler(
    axum::extract::State(state): axum::extract::State<Arc<MoodApiState>>,
    axum::Json(payload): axum::Json<LookupRequest>,
) -> axum::Json<LookupResponse> {
    let cache = state.mood_cache.lock().unwrap_or_else(|e| e.into_inner());
    let cached = cache.get(&payload.chapter, payload.page).cloned();
    drop(cache);

    if let Some(entry) = cached {
        let mood_str = entry.mood.as_str().to_string();
        let source = entry.source;
        let finalized = entry.finalized;

        tracing::info!(
            "POST /api/lookup — cache hit ({}, {}): {}",
            payload.chapter,
            payload.page,
            mood_str
        );

        axum::Json(LookupResponse {
            hit: true,
            mood: Some(mood_str),
            finalized: Some(finalized),
            source: Some(match source {
                CachedMoodSource::VisibleWindowAnalyze => "visible_window".to_string(),
                CachedMoodSource::ChapterPipeline => "chapter_pipeline".to_string(),
            }),
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
            finalized: None,
            source: None,
        })
    }
}

// ─── Cache status endpoint ───────────────────────────────────────────────────

#[derive(serde::Serialize)]
struct CacheStatusResponse {
    entries: usize,
    chapter: Option<String>,
    pages: Vec<u32>,
    pipeline_pages: Vec<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    focus_page: Option<u32>,
    pipeline_processing: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    active_phase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    active_page: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    active_started_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_error: Option<String>,
}

async fn cache_status_handler(
    axum::extract::State(state): axum::extract::State<Arc<MoodApiState>>,
) -> axum::Json<CacheStatusResponse> {
    let (entries, chapter, pages) = {
        let cache = state.mood_cache.lock().unwrap_or_else(|e| e.into_inner());
        (
            cache.len(),
            cache.current_chapter().map(|s| s.to_string()),
            cache.pages(),
        )
    };

    let (pipeline_pages, focus_page, pipeline_processing, active_activity, last_error) = {
        let pipeline = state.chapter_pipeline.lock().await;
        (
            pipeline.registered_pages(),
            pipeline.focus_page(),
            pipeline.is_processing(),
            pipeline.active_activity(),
            pipeline.last_error(),
        )
    };

    let (active_phase, active_page, active_started_at) = active_activity
        .map(|activity: PipelineActivity| {
            (
                Some(activity.phase.to_string()),
                Some(activity.page),
                Some(activity.started_at_ms),
            )
        })
        .unwrap_or((None, None, None));

    tracing::debug!(
        "GET /api/cache/status — entries={}, chapter={:?}",
        entries,
        chapter
    );

    axum::Json(CacheStatusResponse {
        entries,
        chapter,
        pages,
        pipeline_pages,
        focus_page,
        pipeline_processing,
        active_phase,
        active_page,
        active_started_at,
        last_error,
    })
}

// ─── Status & moods ──────────────────────────────────────────────────────────

async fn status_handler(
    axum::extract::State(state): axum::extract::State<Arc<MoodApiState>>,
) -> axum::Json<StatusResponse> {
    tracing::debug!("GET /api/status");

    let (server_status, model_status, port) = match state.llama_server.try_lock() {
        Ok(mut guard) => {
            if let Some(server) = guard.as_mut() {
                let port = server.port;
                if server.is_running() {
                    ("running".to_string(), "loaded".to_string(), port)
                } else {
                    *guard = None;
                    ("stopped".to_string(), "not_loaded".to_string(), 0)
                }
            } else {
                ("stopped".to_string(), "not_loaded".to_string(), 0)
            }
        }
        Err(_) => ("running".to_string(), "loaded".to_string(), 0),
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
        "epic", "tension", "sadness", "comedy", "romance", "horror", "peaceful", "mystery",
    ])
}

// ─── V2 Pipeline endpoints ─────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct ExtractRequest {
    image: String, // base64-encoded image
}

#[derive(serde::Serialize)]
struct ExtractResponse {
    status: String,
    features: PageFeatures,
}

/// POST /api/extract — Layer 1: extract structured features from a single image.
async fn extract_handler(
    axum::extract::State(state): axum::extract::State<Arc<MoodApiState>>,
    axum::Json(payload): axum::Json<ExtractRequest>,
) -> Result<axum::Json<ExtractResponse>, (axum::http::StatusCode, axum::Json<ErrorResponse>)> {
    use base64::Engine;

    tracing::info!(
        "POST /api/extract — received image ({} bytes base64)",
        payload.image.len()
    );

    // Decode + resize image
    let image_bytes = base64::engine::general_purpose::STANDARD
        .decode(&payload.image)
        .map_err(|e| {
            err_response(
                axum::http::StatusCode::BAD_REQUEST,
                format!("Invalid base64 image: {}", e),
            )
        })?;

    let resized_b64 = inference::prepare_image(&image_bytes).map_err(|e| {
        err_response(
            axum::http::StatusCode::BAD_REQUEST,
            format!("Image processing failed: {}", e),
        )
    })?;

    let mut guard = state.llama_server.lock().await;
    let server = guard.as_mut().ok_or_else(|| {
        err_response(
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "llama-server not running".to_string(),
        )
    })?;

    let features = server.extract_structured(&resized_b64).await.map_err(|e| {
        err_response(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Extraction failed: {}", e),
        )
    })?;

    tracing::info!(
        "POST /api/extract — emotion={}, intensity={}, narrative={}",
        features.emotion,
        features.intensity,
        features.narrative
    );

    Ok(axum::Json(ExtractResponse {
        status: "ok".to_string(),
        features,
    }))
}

#[derive(serde::Deserialize)]
struct ClassifyBatchRequest {
    pages: Vec<ClassifyBatchPage>,
    #[serde(default)]
    categories: Option<Vec<CategoryDef>>,
}

#[derive(serde::Deserialize)]
struct ClassifyBatchPage {
    page: u32,
    features: PageFeatures,
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
struct CategoryDef {
    name: String,
    description: String,
}

#[derive(serde::Serialize)]
struct ClassifyBatchResponse {
    status: String,
    moods: Vec<ClassifyBatchResult>,
}

#[derive(serde::Serialize)]
struct ClassifyBatchResult {
    page: u32,
    mood: String,
}

/// POST /api/classify-batch — Layer 2: classify pages from their structured features.
async fn classify_batch_handler(
    axum::extract::State(state): axum::extract::State<Arc<MoodApiState>>,
    axum::Json(payload): axum::Json<ClassifyBatchRequest>,
) -> Result<axum::Json<ClassifyBatchResponse>, (axum::http::StatusCode, axum::Json<ErrorResponse>)>
{
    tracing::info!("POST /api/classify-batch — {} pages", payload.pages.len());

    if payload.pages.is_empty() {
        return Ok(axum::Json(ClassifyBatchResponse {
            status: "ok".to_string(),
            moods: Vec::new(),
        }));
    }

    // Build categories: user-provided or defaults
    let categories: Vec<(&str, &str)> = if let Some(ref cats) = payload.categories {
        cats.iter()
            .map(|c| (c.name.as_str(), c.description.as_str()))
            .collect()
    } else {
        inference::default_mood_categories()
    };

    let feat_refs: Vec<(u32, &PageFeatures)> = payload
        .pages
        .iter()
        .map(|p| (p.page, &p.features))
        .collect();

    let mut guard = state.llama_server.lock().await;
    let server = guard.as_mut().ok_or_else(|| {
        err_response(
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "llama-server not running".to_string(),
        )
    })?;

    let classified = server
        .classify_batch_from_features(&feat_refs, &categories)
        .await
        .map_err(|e| {
            err_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Classification failed: {}", e),
            )
        })?;

    drop(guard);

    let moods: Vec<ClassifyBatchResult> = classified
        .into_iter()
        .map(|(page, mood)| ClassifyBatchResult {
            page,
            mood: mood.as_str().to_string(),
        })
        .collect();

    tracing::info!(
        "POST /api/classify-batch — classified {} pages",
        moods.len()
    );

    Ok(axum::Json(ClassifyBatchResponse {
        status: "ok".to_string(),
        moods,
    }))
}
