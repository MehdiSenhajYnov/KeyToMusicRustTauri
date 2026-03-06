use std::sync::Arc;
use tokio::sync::Mutex;

use super::cache::MoodCache;
use super::director::{MoodDirector, MoodScores, NarrativeRole, PageAnalysis};
use super::inference::{self, HybridResult, LlamaServer, NarrativeContext, PageFeatures};
use crate::types::MoodCategory;

/// Buffer for incremental hybrid extraction results.
/// Accumulates HybridResults as pages are scrolled, stores fused moods.
pub struct DescriptionBuffer {
    pub pages: std::collections::BTreeMap<u32, HybridResult>,
    pub last_classify_at: u32,
    pub moods: std::collections::BTreeMap<u32, String>,
    pub fused_moods: std::collections::BTreeMap<u32, MoodCategory>,
}

impl DescriptionBuffer {
    pub fn new() -> Self {
        Self {
            pages: std::collections::BTreeMap::new(),
            last_classify_at: 0,
            moods: std::collections::BTreeMap::new(),
            fused_moods: std::collections::BTreeMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.pages.clear();
        self.last_classify_at = 0;
        self.moods.clear();
        self.fused_moods.clear();
    }
}

/// State shared with axum handlers.
pub struct MoodApiState {
    pub llama_server: Arc<Mutex<Option<LlamaServer>>>,
    pub app_handle: tauri::AppHandle,
    pub mood_cache: Arc<std::sync::Mutex<MoodCache>>,
    pub mood_director: Arc<std::sync::Mutex<MoodDirector>>,
    pub description_buffer: std::sync::Mutex<DescriptionBuffer>,
}

/// Start the HTTP API server for external tools.
pub async fn start_api_server(
    port: u16,
    llama_server: Arc<Mutex<Option<LlamaServer>>>,
    app_handle: tauri::AppHandle,
    mood_cache: Arc<std::sync::Mutex<MoodCache>>,
    mood_director: Arc<std::sync::Mutex<MoodDirector>>,
) -> Result<(), String> {
    use axum::routing::{get, post};
    use axum::Router;

    let state = Arc::new(MoodApiState {
        llama_server,
        app_handle,
        mood_cache,
        mood_director,
        description_buffer: std::sync::Mutex::new(DescriptionBuffer::new()),
    });

    let app = Router::new()
        .route("/api/analyze", post(analyze_handler))
        .route("/api/extract", post(extract_handler))
        .route("/api/classify-batch", post(classify_batch_handler))
        .route("/api/analyze-v2", post(analyze_v2_handler))
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

    // ─── Check cache first ──────────────────────────────────────────────────

    let cached_result: Option<(MoodCategory, MoodScores, NarrativeRole)> =
        if let (Some(chapter), Some(page)) = (&payload.chapter, payload.page) {
            let cache = state.mood_cache.lock().unwrap_or_else(|e| e.into_inner());
            cache.get(chapter, page).map(|entry| {
                tracing::info!(
                    "POST /api/analyze — cache hit for ({}, {}): {:?}",
                    chapter,
                    page,
                    entry.mood
                );
                (entry.mood, entry.scores.clone(), entry.narrative_role)
            })
        } else {
            None
        };

    let (mood, scores, narrative_role, was_cached) = if let Some((m, s, r)) = cached_result {
        (m, s, r, true)
    } else {
        // ─── Run inference ──────────────────────────────────────────────────

        // Decode + resize image
        let image_bytes = base64::engine::general_purpose::STANDARD
            .decode(&payload.image)
            .map_err(|e| {
                tracing::error!("POST /api/analyze — invalid base64: {}", e);
                err_response(
                    axum::http::StatusCode::BAD_REQUEST,
                    format!("Invalid base64 image: {}", e),
                )
            })?;

        let resized_b64 = inference::prepare_image(&image_bytes).map_err(|e| {
            tracing::error!("POST /api/analyze — image processing failed: {}", e);
            err_response(
                axum::http::StatusCode::BAD_REQUEST,
                format!("Image processing failed: {}", e),
            )
        })?;

        tracing::info!("POST /api/analyze — sending to llama-server...");

        // Build narrative context from director + cache
        let narrative_context = {
            let director = state
                .mood_director
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let previous_moods: Vec<String> = director
                .window_moods()
                .iter()
                .map(|s| s.to_string())
                .collect();
            let current_soundtrack = director.committed_mood().map(|m| m.as_str().to_string());
            let soundtrack_dwell = director.dwell_count();
            drop(director);

            // Look-ahead from cache
            let mut next_moods = Vec::new();
            if let (Some(chapter), Some(page)) = (&payload.chapter, payload.page) {
                let cache = state.mood_cache.lock().unwrap_or_else(|e| e.into_inner());
                for offset in 1..=3u32 {
                    if let Some(entry) = cache.get(chapter, page + offset) {
                        next_moods.push(entry.mood.as_str().to_string());
                    } else {
                        break;
                    }
                }
            }

            NarrativeContext {
                previous_moods,
                current_soundtrack,
                soundtrack_dwell,
                next_moods,
            }
        };

        let mut guard = state.llama_server.lock().await;
        let server = guard.as_mut().ok_or_else(|| {
            tracing::error!("POST /api/analyze — llama-server not running");
            err_response(
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                "llama-server not running".to_string(),
            )
        })?;

        let (inferred_scores, role) = server
            .analyze_mood_scored(&resized_b64, Some(&narrative_context))
            .await
            .map_err(|e| {
                tracing::error!("POST /api/analyze — inference failed: {}", e);
                err_response(
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Analysis failed: {}", e),
                )
            })?;

        // Drop the llama_server lock before continuing
        drop(guard);

        let dominant = inferred_scores.dominant();
        tracing::info!(
            "POST /api/analyze — scored result: dominant={:?}, role={:?}",
            dominant,
            role
        );

        // Store in cache if chapter + page provided
        if let (Some(chapter), Some(page)) = (&payload.chapter, payload.page) {
            let mut cache = state.mood_cache.lock().unwrap_or_else(|e| e.into_inner());
            cache.insert(
                chapter,
                page,
                dominant,
                crate::types::MoodIntensity::Medium,
                inferred_scores.clone(),
                role,
            );
            tracing::debug!(
                "POST /api/analyze — cached for ({}, {}), cache size: {}",
                chapter,
                page,
                cache.len()
            );
        }

        (dominant, inferred_scores, role, false)
    };

    let mood_str = mood.as_str().to_string();

    // ─── Precalculate: cache only, no director, no events ────────────────────
    if payload.precalculate {
        tracing::info!(
            "POST /api/analyze — precalculate done: {} (cached={})",
            mood_str,
            was_cached
        );
        return Ok(axum::Json(AnalyzeResponse {
            mood: mood_str,
            status: "ok".to_string(),
            cached: Some(was_cached),
            committed_mood: None,
            mood_changed: None,
            scores: Some(scores_to_map(&scores)),
            narrative_role: Some(narrative_role.as_str().to_string()),
            dwell_count: None,
        }));
    }

    // ─── Feed into MoodDirector ─────────────────────────────────────────────

    let analysis = PageAnalysis {
        scores: scores.clone(),
        intensity: crate::types::MoodIntensity::Medium, // TODO: use real intensity from VLM
        narrative_role,
        dominant_mood: mood,
    };

    let decision = {
        let mut director = state
            .mood_director
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        director.process(analysis, payload.chapter.as_deref())
    };

    // ─── Emit events ────────────────────────────────────────────────────────

    // Always emit mood_detected (raw mood for UI display)
    let _ = state.app_handle.emit(
        "mood_detected",
        serde_json::json!({
            "mood": mood_str,
            "source": "api",
            "scores": scores_to_map(&decision.raw_scores),
            "narrative_role": narrative_role.as_str(),
        }),
    );

    // Only emit mood_committed if mood actually changed
    if decision.mood_changed {
        let committed_str = decision.committed_mood.as_str().to_string();
        tracing::info!(
            "POST /api/analyze — mood_committed: {} (dwell={})",
            committed_str,
            decision.dwell_count
        );
        let _ = state.app_handle.emit(
            "mood_committed",
            serde_json::json!({
                "mood": committed_str,
                "source": "api",
                "previous_mood": decision.raw_mood.as_str(),
                "dwell_count": decision.dwell_count,
            }),
        );
    }

    Ok(axum::Json(AnalyzeResponse {
        mood: mood_str,
        status: "ok".to_string(),
        cached: Some(was_cached),
        committed_mood: Some(decision.committed_mood.as_str().to_string()),
        mood_changed: Some(decision.mood_changed),
        scores: Some(scores_to_map(&decision.raw_scores)),
        narrative_role: Some(narrative_role.as_str().to_string()),
        dwell_count: Some(decision.dwell_count),
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
}

/// Look up a pre-calculated mood from the cache.
/// On cache hit, feeds through the director and emits appropriate events.
async fn lookup_handler(
    axum::extract::State(state): axum::extract::State<Arc<MoodApiState>>,
    axum::Json(payload): axum::Json<LookupRequest>,
) -> axum::Json<LookupResponse> {
    use tauri::Emitter;

    let cache = state.mood_cache.lock().unwrap_or_else(|e| e.into_inner());
    let cached = cache.get(&payload.chapter, payload.page).cloned();
    drop(cache);

    if let Some(entry) = cached {
        let mood_str = entry.mood.as_str().to_string();

        tracing::info!(
            "POST /api/lookup — cache hit ({}, {}): {}",
            payload.chapter,
            payload.page,
            mood_str
        );

        // Feed through director
        let analysis = PageAnalysis {
            scores: entry.scores,
            intensity: entry.intensity,
            narrative_role: entry.narrative_role,
            dominant_mood: entry.mood,
        };

        let decision = {
            let mut director = state
                .mood_director
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            director.process(analysis, Some(&payload.chapter))
        };

        // Always emit raw mood
        let _ = state.app_handle.emit(
            "mood_detected",
            serde_json::json!({ "mood": mood_str, "source": "api" }),
        );

        // Only emit committed if changed
        if decision.mood_changed {
            let committed_str = decision.committed_mood.as_str().to_string();
            let _ = state.app_handle.emit(
                "mood_committed",
                serde_json::json!({ "mood": committed_str, "source": "api" }),
            );
        }

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

#[derive(serde::Deserialize)]
struct AnalyzeV2Request {
    images: Vec<AnalyzeV2Image>,
    #[serde(default)]
    #[allow(dead_code)]
    categories: Option<Vec<CategoryDef>>,
    #[serde(default)]
    chapter: Option<String>,
}

#[derive(serde::Deserialize)]
struct AnalyzeV2Image {
    page: u32,
    image: String, // base64
}

#[derive(serde::Serialize)]
struct AnalyzeV2Response {
    status: String,
    moods: Vec<AnalyzeV2PageResult>,
}

#[derive(serde::Serialize)]
struct AnalyzeV2PageResult {
    page: u32,
    mood: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    corrected_mood: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    committed_mood: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mood_changed: Option<bool>,
}

/// POST /api/analyze-v2 — V5 pipeline: VLM mood + VLM describe + LLM text correction.
///
/// Takes multiple images, runs VLM mood classification and description per page,
/// then corrects moods via text-only LLM batch, then feeds through MoodDirector.
async fn analyze_v2_handler(
    axum::extract::State(state): axum::extract::State<Arc<MoodApiState>>,
    axum::Json(payload): axum::Json<AnalyzeV2Request>,
) -> Result<axum::Json<AnalyzeV2Response>, (axum::http::StatusCode, axum::Json<ErrorResponse>)> {
    use base64::Engine;
    use tauri::Emitter;

    tracing::info!(
        "POST /api/analyze-v2 — {} images, chapter={:?}",
        payload.images.len(),
        payload.chapter
    );

    if payload.images.is_empty() {
        return Ok(axum::Json(AnalyzeV2Response {
            status: "ok".to_string(),
            moods: Vec::new(),
        }));
    }

    // Stage 1: Per-page VLM inference (mood + description)
    let mut page_results: Vec<(u32, MoodCategory, String)> = Vec::new(); // (page, mood, description)

    {
        let mut guard = state.llama_server.lock().await;
        let server = guard.as_mut().ok_or_else(|| {
            err_response(
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                "llama-server not running".to_string(),
            )
        })?;

        for img in &payload.images {
            let image_bytes = base64::engine::general_purpose::STANDARD
                .decode(&img.image)
                .map_err(|e| {
                    err_response(
                        axum::http::StatusCode::BAD_REQUEST,
                        format!("Invalid base64 for page {}: {}", img.page, e),
                    )
                })?;

            let resized_b64 = inference::prepare_image(&image_bytes).map_err(|e| {
                err_response(
                    axum::http::StatusCode::BAD_REQUEST,
                    format!("Image processing failed for page {}: {}", img.page, e),
                )
            })?;

            // Inference 1: dimensional mood classification
            let mood_tag = match server.analyze_mood(&resized_b64).await {
                Ok(m) => m,
                Err(e) => {
                    tracing::error!(
                        "POST /api/analyze-v2 — page {} mood inference failed: {}",
                        img.page,
                        e
                    );
                    continue;
                }
            };
            let mood = mood_tag.mood;

            // Inference 2: VLM page description
            let description = match server.describe_page(&resized_b64).await {
                Ok(d) => d,
                Err(e) => {
                    tracing::error!(
                        "POST /api/analyze-v2 — page {} description failed: {}",
                        img.page,
                        e
                    );
                    continue;
                }
            };

            tracing::info!(
                "POST /api/analyze-v2 — page {}: mood={:?}, desc={}...",
                img.page,
                mood,
                &description[..description.len().min(80)]
            );
            page_results.push((img.page, mood, description));
        }

        // Stage 2: LLM text correction (batch, still holding server lock)
        if page_results.len() >= 2 {
            let correction_input: Vec<(u32, &str, &str)> = page_results
                .iter()
                .map(|(p, m, d)| (*p, m.as_str(), d.as_str()))
                .collect();

            match server.correct_moods_batch(&correction_input).await {
                Ok(corrected) => {
                    let corrected_map: std::collections::HashMap<u32, MoodCategory> =
                        corrected.into_iter().collect();
                    // Apply corrections
                    for (page, mood, _) in page_results.iter_mut() {
                        if let Some(&corrected_mood) = corrected_map.get(page) {
                            if corrected_mood != *mood {
                                tracing::info!(
                                    "POST /api/analyze-v2 — page {} corrected: {:?} -> {:?}",
                                    page,
                                    mood,
                                    corrected_mood
                                );
                                *mood = corrected_mood;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "POST /api/analyze-v2 — batch correction failed, using raw moods: {}",
                        e
                    );
                }
            }
        }

        drop(guard);
    }

    if page_results.is_empty() {
        return Err(err_response(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "All image analyses failed".to_string(),
        ));
    }

    // Stage 3: Feed corrected moods through MoodDirector (in page order)
    page_results.sort_by_key(|(p, _, _)| *p);

    let mut results: Vec<AnalyzeV2PageResult> = Vec::new();

    for (page_num, mood, description) in &page_results {
        let analysis = PageAnalysis {
            scores: MoodScores::from_single(*mood),
            intensity: crate::types::MoodIntensity::Medium, // TODO: use real intensity
            narrative_role: super::director::NarrativeRole::Continuation,
            dominant_mood: *mood,
        };

        let decision = {
            let mut director = state
                .mood_director
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            director.process(analysis, payload.chapter.as_deref())
        };

        let mood_str = mood.as_str().to_string();

        // Emit events
        let _ = state.app_handle.emit(
            "mood_detected",
            serde_json::json!({ "mood": mood_str, "source": "api" }),
        );

        if decision.mood_changed {
            let committed_str = decision.committed_mood.as_str().to_string();
            let _ = state.app_handle.emit(
                "mood_committed",
                serde_json::json!({
                    "mood": committed_str,
                    "source": "api",
                    "dwell_count": decision.dwell_count,
                }),
            );
        }

        results.push(AnalyzeV2PageResult {
            page: *page_num,
            mood: mood_str,
            description: Some(description.clone()),
            corrected_mood: None, // Already applied inline
            committed_mood: Some(decision.committed_mood.as_str().to_string()),
            mood_changed: Some(decision.mood_changed),
        });
    }

    tracing::info!("POST /api/analyze-v2 — processed {} pages", results.len());

    Ok(axum::Json(AnalyzeV2Response {
        status: "ok".to_string(),
        moods: results,
    }))
}
