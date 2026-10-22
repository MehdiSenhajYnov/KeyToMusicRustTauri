use crate::events::CoreEvent;
use crate::mood;
use crate::state::AppState;
use crate::types::AppConfig;

#[derive(Debug, Clone, serde::Serialize)]
pub struct MoodServiceStatus {
    pub runtime: String,
    pub api: String,
    pub enabled: bool,
    pub port: u16,
}

fn emit_server_status(state: &AppState, status: &str) {
    state.emit(CoreEvent::MoodServerStatus {
        status: status.to_string(),
    });
}

pub(crate) fn api_task_status(state: &AppState, enabled: bool) -> String {
    let mut guard = state
        .mood_api_server
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let finished = guard.as_ref().map(|h| h.is_finished()).unwrap_or(false);
    if finished {
        guard.take();
    }
    if !enabled {
        "disabled".to_string()
    } else if guard.is_some() {
        "running".to_string()
    } else {
        "stopped".to_string()
    }
}

pub(crate) fn emit_api_status(state: &AppState) {
    let config = state.get_config();
    let status = api_task_status(state, config.mood_ai_enabled);
    state.emit(CoreEvent::MoodApiStatus {
        status,
        enabled: config.mood_ai_enabled,
        port: config.mood_api_port,
    });
}

/// "running" or "stopped"; clears a llama-server that exited on its own.
pub async fn runtime_status(state: &AppState) -> String {
    let mut guard = state.llama_server.lock().await;
    let is_running = guard.as_mut().map(|s| s.is_running()).unwrap_or(false);
    if guard.is_some() && !is_running {
        *guard = None;
        "stopped".to_string()
    } else if is_running {
        "running".to_string()
    } else {
        "stopped".to_string()
    }
}

async fn stop_api_server(state: &AppState) {
    let mut api_guard = state
        .mood_api_server
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if let Some(handle) = api_guard.take() {
        handle.abort();
    }
    drop(api_guard);
    emit_api_status(state);
}

async fn start_api_server(state: &AppState, port: u16) -> Result<(), String> {
    if runtime_status(state).await != "running" {
        return Err("Mood AI runtime is not running".to_string());
    }

    stop_api_server(state).await;

    let bind_probe = std::net::TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| format!("Failed to bind Mood API port {}: {}", port, e))?;
    drop(bind_probe);

    let llama_ref = state.llama_server.clone();
    let events = state.event_sink();
    let cache_ref = state.mood_cache.clone();

    let api_handle = tokio::spawn(async move {
        if let Err(e) = mood::server::start_api_server(port, llama_ref, events, cache_ref).await {
            tracing::error!("Mood API server error: {}", e);
        }
    });

    {
        let mut api_guard = state
            .mood_api_server
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *api_guard = Some(api_handle);
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(400))
        .build()
        .map_err(|e| format!("Failed to build HTTP client for Mood API check: {}", e))?;
    let status_url = format!("http://127.0.0.1:{}/api/status", port);

    for _ in 0..20 {
        if client
            .get(&status_url)
            .send()
            .await
            .map(|resp| resp.status().is_success())
            .unwrap_or(false)
        {
            emit_api_status(state);
            return Ok(());
        }

        let finished = {
            let mut api_guard = state
                .mood_api_server
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let finished = api_guard.as_ref().map(|h| h.is_finished()).unwrap_or(false);
            if finished {
                api_guard.take();
            }
            finished
        };

        if finished {
            break;
        }

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    stop_api_server(state).await;
    Err(format!(
        "Mood API failed to start on port {}. Check if the port is already in use.",
        port
    ))
}

async fn start_runtime(state: &AppState) -> Result<(), String> {
    if runtime_status(state).await == "running" {
        emit_server_status(state, "running");
        return Ok(());
    }

    let model_path = mood::llama_manager::get_model_path();
    let mmproj_path = mood::llama_manager::get_mmproj_path();
    if !model_path.exists() {
        return Err("LLM model not downloaded".to_string());
    }
    if !mmproj_path.exists() {
        return Err("Vision encoder (mmproj) not downloaded".to_string());
    }

    emit_server_status(state, "starting");

    let model_str = model_path.to_string_lossy().to_string();
    let mmproj_str = mmproj_path.to_string_lossy().to_string();
    let server = match mood::inference::LlamaServer::start_with_options(
        &model_str,
        &mmproj_str,
        mood::inference::LlamaServerStartOptions {
            reasoning_format: mood::inference::reasoning_format_from_env(),
            context_size: None,
            parallel_slots: None,
            gpu_layers: None,
            runtime_intent: Some(mood::winner::winner_runtime_intent()),
        },
    )
    .await
    {
        Ok(server) => server,
        Err(err) => {
            emit_server_status(state, "error");
            return Err(err);
        }
    };

    let mut guard = state.llama_server.lock().await;
    *guard = Some(server);
    drop(guard);

    emit_server_status(state, "running");
    Ok(())
}

async fn stop_runtime(state: &AppState) {
    stop_api_server(state).await;

    let mut guard = state.llama_server.lock().await;
    if let Some(mut server) = guard.take() {
        server.stop();
    }
    drop(guard);

    emit_server_status(state, "stopped");
}

pub(crate) async fn sync_service_config(
    state: &AppState,
    previous: &AppConfig,
    current: &AppConfig,
) -> Result<(), String> {
    if !current.mood_ai_enabled {
        stop_runtime(state).await;
        return Ok(());
    }

    let has_runtime_assets = mood::llama_manager::is_llama_server_installed()
        && mood::llama_manager::is_model_downloaded();
    if !has_runtime_assets {
        emit_server_status(state, &runtime_status(state).await);
        emit_api_status(state);
        return Ok(());
    }

    start_runtime(state).await?;

    let needs_api_restart = !previous.mood_ai_enabled
        || previous.mood_api_port != current.mood_api_port
        || api_task_status(state, true) != "running";
    if needs_api_restart {
        start_api_server(state, current.mood_api_port).await?;
    } else {
        emit_api_status(state);
    }

    Ok(())
}

/// Start llama-server and the extension API (Mood AI must be enabled).
pub async fn start_server(state: &AppState) -> Result<(), String> {
    let _lifecycle = state.mood_lifecycle.lock().await;
    let config = state.get_config();
    if !config.mood_ai_enabled {
        return Err("Enable Mood AI first to expose the browser extension API.".to_string());
    }

    start_runtime(state).await?;
    start_api_server(state, config.mood_api_port).await?;
    Ok(())
}

pub async fn stop_server(state: &AppState) {
    let _lifecycle = state.mood_lifecycle.lock().await;
    stop_runtime(state).await;
}

pub async fn service_status(state: &AppState) -> MoodServiceStatus {
    let config = state.get_config();
    MoodServiceStatus {
        runtime: runtime_status(state).await,
        api: api_task_status(state, config.mood_ai_enabled),
        enabled: config.mood_ai_enabled,
        port: config.mood_api_port,
    }
}

/// Download the vision model, reporting byte progress as events.
pub async fn install_model(state: &AppState) -> Result<String, String> {
    let sink = state.event_sink();
    let path = mood::llama_manager::download_model(move |downloaded, total| {
        sink.emit(CoreEvent::MoodModelDownloadProgress { downloaded, total });
    })
    .await?;
    Ok(path.to_string_lossy().to_string())
}

pub async fn install_llama_server() -> Result<String, String> {
    let path = mood::llama_manager::download_llama_server().await?;
    Ok(path.to_string_lossy().to_string())
}
