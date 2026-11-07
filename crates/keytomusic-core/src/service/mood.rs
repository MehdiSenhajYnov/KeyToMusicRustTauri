//! Mood AI service: while it is enabled, the local API for the extension
//! listens and the engine analyzes chapters with the configured model.

use std::path::PathBuf;

use crate::events::CoreEvent;
use crate::mood::chatgpt::{self, Account};
use crate::mood::provider::{AuthMode, ModelInfo};
use crate::mood::{engine::ChapterSnapshot, extension, provider, secret, server};
use crate::state::AppState;
use crate::types::{AppConfig, MoodAuth};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoodServiceStatus {
    pub enabled: bool,
    /// "chatgpt" (the user's ChatGPT plan) or "api_key".
    pub auth: String,
    /// Signed-in ChatGPT account, if any.
    pub account: Option<Account>,
    /// "running", "stopped" or "disabled".
    pub api: String,
    pub port: u16,
    pub base_url: String,
    pub model: String,
    /// The endpoint needs no key (local server).
    pub local: bool,
    /// Masked saved key (`sk-…abcd`), if any.
    pub api_key: Option<String>,
    /// Key (if needed) and model are set: analysis can run.
    pub configured: bool,
    /// What stops Mood AI (port taken, key refused…), if anything.
    pub error: Option<String>,
    pub chapter: Option<ChapterSnapshot>,
    /// Seconds since the browser extension last talked to the app (None:
    /// not since launch).
    pub extension_seen_secs: Option<u64>,
    /// Folder of the extension copy the app keeps up to date, and its version.
    pub extension_dir: Option<PathBuf>,
}

/// Where users review and limit what apps use of their ChatGPT plan.
pub const CHATGPT_USAGE_URL: &str = chatgpt::USAGE_URL;

/// Ready-made endpoints: (name, base URL, needs a key).
pub fn presets() -> &'static [(&'static str, &'static str, bool)] {
    provider::PRESETS
}

pub fn provider_settings(config: &AppConfig) -> provider::ProviderSettings {
    provider::ProviderSettings {
        auth: match config.mood_auth {
            MoodAuth::Chatgpt => AuthMode::ChatGpt {
                plan: chatgpt::global().can_use_plan(),
            },
            MoodAuth::ApiKey => AuthMode::ApiKey,
        },
        base_url: config.mood_base_url.clone(),
        model: config.mood_model.clone(),
        api_key: secret::load(&config.mood_base_url),
        image_detail: config.mood_image_detail,
    }
}

fn api_port(state: &AppState) -> Option<u16> {
    let mut guard = state
        .mood_api_server
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if guard
        .as_ref()
        .is_some_and(|(_, handle)| handle.is_finished())
    {
        guard.take();
    }
    guard.as_ref().map(|(port, _)| *port)
}

fn api_state(state: &AppState, config: &AppConfig) -> &'static str {
    match (config.mood_ai_enabled, api_port(state)) {
        (false, _) => "disabled",
        (true, Some(_)) => "running",
        (true, None) => "stopped",
    }
}

pub fn status(state: &AppState) -> MoodServiceStatus {
    let config = state.get_config();
    let settings = provider_settings(&config);
    let api_error = state
        .mood_api_error
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    MoodServiceStatus {
        enabled: config.mood_ai_enabled,
        auth: config.mood_auth.as_str().into(),
        account: chatgpt::global().account(),
        api: api_state(state, &config).into(),
        port: config.mood_api_port,
        local: settings.is_local(),
        api_key: settings.api_key.as_deref().map(secret::masked),
        configured: settings.is_configured(),
        error: api_error.or_else(|| state.mood.blocking_error()),
        chapter: state.mood.snapshot(),
        extension_seen_secs: state.mood.extension_seen().map(|d| d.as_secs()),
        extension_dir: extension::installed_dir(),
        base_url: settings.base_url,
        model: settings.model,
    }
}

/// Same as [`status`], for callers in async code.
pub async fn service_status(state: &AppState) -> MoodServiceStatus {
    status(state)
}

pub(crate) fn emit_api_status(state: &AppState) {
    let config = state.get_config();
    state.emit(CoreEvent::MoodApiStatus {
        status: api_state(state, &config).into(),
        enabled: config.mood_ai_enabled,
        port: config.mood_api_port,
    });
}

fn set_api_error(state: &AppState, error: Option<String>) {
    *state
        .mood_api_error
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = error;
}

fn stop_api(state: &AppState) {
    let previous = state
        .mood_api_server
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take();
    if let Some((_, handle)) = previous {
        handle.abort();
    }
}

async fn start(state: &AppState, config: &AppConfig) -> Result<(), String> {
    state.mood.configure(provider_settings(config));
    // The extension the user loaded follows the app's version (it reloads
    // itself when it sees the newer one).
    let refreshed = tokio::task::spawn_blocking(extension::refresh_installed)
        .await
        .unwrap_or_default();
    for dir in refreshed {
        tracing::info!("Mood AI: extension updated in {}", dir.display());
    }
    if !state.mood.is_running() {
        state.mood.start(state.event_sink());
    }
    if api_port(state) == Some(config.mood_api_port) {
        return Ok(());
    }
    stop_api(state);
    match server::start(config.mood_api_port, state.mood.clone()).await {
        Ok(handle) => {
            *state
                .mood_api_server
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some((config.mood_api_port, handle));
            set_api_error(state, None);
            Ok(())
        }
        Err(error) => {
            set_api_error(state, Some(error.clone()));
            Err(error)
        }
    }
}

fn stop(state: &AppState) {
    stop_api(state);
    state.mood.stop();
    set_api_error(state, None);
}

/// Apply Mood AI settings after a config change.
pub(crate) async fn sync_service_config(
    state: &AppState,
    _previous: &AppConfig,
    current: &AppConfig,
) -> Result<(), String> {
    let result = if current.mood_ai_enabled {
        start(state, current).await
    } else {
        stop(state);
        Ok(())
    };
    emit_api_status(state);
    result
}

/// Start Mood AI at launch when it was left enabled. Errors (port taken) are
/// kept for the UIs instead of failing the launch.
pub async fn start_on_launch(state: &AppState) {
    let _lifecycle = state.mood_lifecycle.lock().await;
    let config = state.get_config();
    if !config.mood_ai_enabled {
        return;
    }
    if let Err(error) = start(state, &config).await {
        tracing::warn!("Mood AI could not start: {error}");
    }
    emit_api_status(state);
}

/// Save (or forget, with `None`) the API key and retry with it.
pub async fn set_api_key(state: &AppState, key: Option<String>) -> Result<(), String> {
    let _lifecycle = state.mood_lifecycle.lock().await;
    secret::save(key.as_deref())?;
    let config = state.get_config();
    state.mood.configure(provider_settings(&config));
    emit_api_status(state);
    Ok(())
}

/// Check the connection; returns the models the account or endpoint offers.
pub async fn test_connection(state: &AppState) -> Result<Vec<ModelInfo>, String> {
    let settings = provider_settings(&state.get_config());
    match settings.auth {
        AuthMode::ChatGpt { .. } if chatgpt::global().account().is_none() => {
            return Err("Connecte-toi d'abord avec ChatGPT".into())
        }
        AuthMode::ApiKey if !settings.is_local() && settings.api_key.is_none() => {
            return Err("Ajoute d'abord ta clé API".into())
        }
        _ => {}
    }
    provider::list_models(&settings)
        .await
        .map_err(|e| e.message)
}

fn emit_account(sink: &crate::events::SharedSink, error: Option<String>) {
    let account = chatgpt::global().account();
    sink.emit(CoreEvent::MoodAccount {
        email: account.as_ref().and_then(|a| a.email.clone()),
        plan_enabled: account.is_some_and(|a| a.plan_enabled),
        error,
    });
}

/// Start "Sign in with ChatGPT": returns the page to open in the browser.
/// When the browser comes back, Mood AI switches to the ChatGPT plan and a
/// `MoodAccount` event reports the result.
pub async fn chatgpt_sign_in(state: &AppState) -> Result<String, String> {
    let engine = state.mood.clone();
    let sink = state.event_sink();
    let config = state.get_config();
    chatgpt::global()
        .begin_sign_in(move |result| {
            let error = result.err().map(|e| e.message);
            let mut config = config;
            config.mood_auth = MoodAuth::Chatgpt;
            engine.configure(provider_settings(&config));
            emit_account(&sink, error);
        })
        .await
        .map_err(|e| e.message)
}

/// Sign out of ChatGPT (the session is revoked at OpenAI when reachable).
pub async fn chatgpt_sign_out(state: &AppState) -> Result<(), String> {
    let result = chatgpt::global().sign_out().await.map_err(|e| e.message);
    state.mood.configure(provider_settings(&state.get_config()));
    emit_account(&state.event_sink(), result.clone().err());
    result
}

/// Forget every scene already decided (they will be analyzed again).
pub fn clear_cache(state: &AppState) {
    state.mood.clear_cache();
}

/// Write the browser extension to `destination` (default: Downloads).
pub fn export_extension(destination: Option<PathBuf>) -> Result<PathBuf, String> {
    let dir = destination.unwrap_or_else(extension::default_destination);
    extension::export(&dir)
}

/// Default moods for the keys of `profile` nobody tagged: asked to the
/// configured model when there is one, guessed from the titles otherwise.
/// Nothing changes until the user applies them
/// ([`autotag::apply`](crate::mood::autotag::apply)).
pub async fn suggest_default_moods(
    state: &AppState,
    profile: &crate::types::Profile,
) -> crate::mood::autotag::AutoTagResult {
    let settings = provider_settings(&state.get_config());
    let settings = settings.is_configured().then_some(settings);
    crate::mood::autotag::suggest(settings.as_ref(), profile).await
}
