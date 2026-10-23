//! Tauri IPC surface. Each command forwards to `keytomusic_core::service`;
//! only the native file pickers and the keyboard layout live here.

use std::collections::HashMap;
use std::sync::Arc;

use keytomusic_core::audio::{self, analysis::WaveformData};
use keytomusic_core::discovery::engine::DiscoverySuggestion;
use keytomusic_core::service::{
    self, discovery::DiscoveryCacheResponse, discovery::DislikedVideoInfo,
    input_access::LinuxInputAccessFixResult, input_access::LinuxInputAccessStatusPayload,
    library::InitialState, library::MissingSoundInfo, mood::MoodServiceStatus,
    playback::SoundPreloadEntry, waveform::WaveformBatchEntry, youtube::PredownloadResult,
};
use keytomusic_core::state::AppState;
use keytomusic_core::types::{AppConfig, Profile, Sound};
use keytomusic_core::{discovery, import_export, mood, storage, youtube};
use tauri::State;

type AppStateRef<'a> = State<'a, Arc<AppState>>;

// ─── Configuration Commands ─────────────────────────────────────────────────

#[tauri::command]
pub async fn get_keyboard_layout(app: tauri::AppHandle) -> Result<HashMap<String, String>, String> {
    crate::keyboard_layout::current(&app).await
}

#[tauri::command]
pub fn get_config(state: AppStateRef<'_>) -> Result<AppConfig, String> {
    Ok(state.get_config())
}

#[tauri::command]
pub async fn update_config(
    state: AppStateRef<'_>,
    updates: serde_json::Value,
) -> Result<(), String> {
    service::config::update_config(&state, updates).await
}

/// Set the profile bindings for chord detection.
/// Called when profile changes or bindings are modified.
#[tauri::command]
pub fn set_profile_bindings(state: AppStateRef<'_>, bindings: Vec<String>) -> Result<(), String> {
    state.key_detector.set_profile_bindings(&bindings);
    Ok(())
}

#[tauri::command]
pub fn flush_config(state: AppStateRef<'_>) -> Result<(), String> {
    state.flush_config()
}

// ─── Profile Commands ───────────────────────────────────────────────────────

#[tauri::command]
pub fn list_profiles() -> Result<Vec<storage::ProfileSummary>, String> {
    storage::list_profiles()
}

#[tauri::command]
pub fn create_profile(name: String) -> Result<Profile, String> {
    storage::create_profile(name)
}

#[tauri::command]
pub fn load_profile(id: String) -> Result<Profile, String> {
    storage::load_profile(id)
}

/// Run blocking filesystem work (profile writes, cache scans) off the async workers.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|e| format!("Background task failed: {}", e))?
}

#[tauri::command]
pub async fn save_profile(profile: Profile) -> Result<(), String> {
    blocking(move || storage::save_profile(&profile)).await
}

#[tauri::command]
pub async fn delete_profile(state: AppStateRef<'_>, id: String) -> Result<(), String> {
    let state = Arc::clone(&state);
    blocking(move || service::library::delete_profile(&state, id)).await
}

#[tauri::command]
pub fn duplicate_profile(id: String, new_name: Option<String>) -> Result<Profile, String> {
    storage::duplicate_profile(id, new_name)
}

// ─── Audio Commands ─────────────────────────────────────────────────────────

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Keep explicit IPC fields compatible with existing callers.
pub async fn play_sound(
    state: AppStateRef<'_>,
    track_id: String,
    sound_id: String,
    file_path: String,
    start_position: f64,
    sound_volume: f32,
    track_volume: Option<f32>,
) -> Result<(), String> {
    service::playback::play_sound(
        &state,
        track_id,
        sound_id,
        file_path,
        start_position,
        sound_volume,
        track_volume,
    )
    .await
}

#[tauri::command]
pub fn stop_sound(state: AppStateRef<'_>, track_id: String) -> Result<(), String> {
    state.get_audio_engine()?.stop_track(track_id)
}

#[tauri::command]
pub fn stop_all_sounds(state: AppStateRef<'_>) -> Result<(), String> {
    service::playback::stop_all_sounds(&state)
}

#[tauri::command]
pub fn set_master_volume(state: AppStateRef<'_>, volume: f32) -> Result<(), String> {
    service::config::set_master_volume(&state, volume);
    Ok(())
}

#[tauri::command]
pub fn set_track_volume(
    state: AppStateRef<'_>,
    track_id: String,
    volume: f32,
) -> Result<(), String> {
    state.get_audio_engine()?.set_track_volume(track_id, volume)
}

#[tauri::command]
pub fn set_sound_volume(
    state: AppStateRef<'_>,
    track_id: String,
    sound_id: String,
    volume: f32,
) -> Result<(), String> {
    state
        .get_audio_engine()?
        .set_sound_volume(track_id, sound_id, volume)
}

#[tauri::command]
pub async fn get_audio_duration(state: AppStateRef<'_>, path: String) -> Result<f64, String> {
    service::playback::get_audio_duration(&state, path).await
}

#[tauri::command]
pub async fn preload_profile_sounds(
    state: AppStateRef<'_>,
    sounds: Vec<SoundPreloadEntry>,
) -> Result<HashMap<String, f64>, String> {
    service::playback::preload_profile_sounds(&state, sounds).await
}

// ─── Key Detection Commands ────────────────────────────────────────────────

#[tauri::command]
pub fn get_linux_input_access_status() -> Result<LinuxInputAccessStatusPayload, String> {
    Ok(service::input_access::linux_input_status())
}

#[tauri::command]
pub fn enable_linux_background_detection() -> Result<LinuxInputAccessFixResult, String> {
    service::input_access::enable_linux_background_detection()
}

#[tauri::command]
pub fn suspend_key_detection(state: AppStateRef<'_>, suspended: bool) {
    state.key_detector.set_suspended(suspended);
}

/// Typing in a text field: sound keys are muted, modifier shortcuts still work.
#[tauri::command]
pub fn set_key_detection_typing(state: AppStateRef<'_>, typing: bool) {
    state.key_detector.set_typing(typing);
}

#[tauri::command]
pub fn set_key_detection(state: AppStateRef<'_>, enabled: bool) -> Result<(), String> {
    service::config::set_key_detection(&state, enabled);
    Ok(())
}

#[tauri::command]
pub fn set_stop_all_shortcut(state: AppStateRef<'_>, keys: Vec<String>) -> Result<(), String> {
    service::config::set_stop_all_shortcut(&state, keys)
}

#[tauri::command]
pub fn set_key_cooldown(state: AppStateRef<'_>, cooldown_ms: u32) -> Result<(), String> {
    service::config::set_key_cooldown(&state, cooldown_ms)
}

// ─── Audio Device Commands ────────────────────────────────────────────────

#[tauri::command]
pub fn list_audio_devices() -> Vec<String> {
    audio::list_audio_devices()
}

#[tauri::command]
pub fn set_audio_device(state: AppStateRef<'_>, device: Option<String>) -> Result<(), String> {
    service::config::set_audio_device(&state, device)
}

// ─── Waveform Commands ────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_waveform(
    state: AppStateRef<'_>,
    path: String,
    num_points: usize,
) -> Result<WaveformData, String> {
    service::waveform::get_waveform(&state, path, num_points).await
}

#[tauri::command]
pub async fn get_waveforms_batch(
    state: AppStateRef<'_>,
    entries: Vec<WaveformBatchEntry>,
) -> Result<HashMap<String, WaveformData>, String> {
    service::waveform::get_waveforms_batch(&state, entries).await
}

// ─── YouTube Commands ─────────────────────────────────────────────────────

#[tauri::command]
pub async fn add_sound_from_youtube(
    state: AppStateRef<'_>,
    url: String,
    download_id: String,
) -> Result<Sound, String> {
    service::youtube::add_sound_from_youtube(&state, url, download_id).await
}

#[tauri::command]
pub async fn search_youtube(
    state: AppStateRef<'_>,
    query: String,
    max_results: u32,
) -> Result<Vec<youtube::search::YoutubeSearchResult>, String> {
    youtube::search::search_youtube(&query, max_results, state.youtube_cache.clone()).await
}

#[tauri::command]
pub async fn fetch_playlist(
    state: AppStateRef<'_>,
    url: String,
) -> Result<youtube::search::YoutubePlaylist, String> {
    youtube::search::fetch_playlist(&url, state.youtube_cache.clone()).await
}

#[tauri::command]
pub async fn get_youtube_stream_url(
    video_id: String,
) -> Result<youtube::search::StreamUrlResult, String> {
    youtube::search::get_stream_url(&video_id).await
}

#[tauri::command]
pub async fn check_yt_dlp_installed() -> Result<bool, String> {
    Ok(youtube::is_yt_dlp_installed().await)
}

#[tauri::command]
pub async fn install_yt_dlp() -> Result<(), String> {
    youtube::download_yt_dlp().await?;
    Ok(())
}

#[tauri::command]
pub async fn check_ffmpeg_installed() -> Result<bool, String> {
    Ok(youtube::is_ffmpeg_installed())
}

#[tauri::command]
pub async fn install_ffmpeg() -> Result<(), String> {
    youtube::download_ffmpeg().await?;
    Ok(())
}

// ─── Import/Export Commands ─────────────────────────────────────────────────

#[tauri::command]
pub async fn export_profile(
    state: AppStateRef<'_>,
    profile_id: String,
    output_path: String,
) -> Result<(), String> {
    service::library::export_profile(&state, profile_id, output_path).await
}

#[tauri::command]
pub async fn import_profile(state: AppStateRef<'_>, ktm_path: String) -> Result<String, String> {
    service::library::import_profile(&state, ktm_path).await
}

#[tauri::command]
pub fn cleanup_export_temp() {
    import_export::cleanup_interrupted_export();
}

#[tauri::command]
pub fn cancel_export() {
    import_export::cancel_export();
}

/// Run a blocking `rfd` dialog off the async runtime.
async fn file_dialog<T: Send + 'static>(
    show: impl FnOnce() -> T + Send + 'static,
) -> Result<T, String> {
    tokio::task::spawn_blocking(show)
        .await
        .map_err(|e| format!("File dialog failed: {}", e))
}

fn path_string(path: std::path::PathBuf) -> String {
    path.to_string_lossy().to_string()
}

#[tauri::command]
pub async fn pick_save_location(default_name: String) -> Result<Option<String>, String> {
    file_dialog(move || {
        rfd::FileDialog::new()
            .set_file_name(&default_name)
            .add_filter("KeyToMusic Profile", &["ktm"])
            .save_file()
            .map(path_string)
    })
    .await
}

#[tauri::command]
pub async fn pick_ktm_file() -> Result<Option<String>, String> {
    file_dialog(|| {
        rfd::FileDialog::new()
            .add_filter("KeyToMusic Profile", &["ktm"])
            .pick_file()
            .map(path_string)
    })
    .await
}

// ─── Legacy Import Commands ──────────────────────────────────────────────

/// Pick a legacy save JSON file.
#[tauri::command]
pub async fn pick_legacy_file() -> Result<Option<String>, String> {
    file_dialog(|| {
        rfd::FileDialog::new()
            .add_filter("Legacy Save", &["json"])
            .pick_file()
            .map(path_string)
    })
    .await
}

/// Import a legacy KeyToMusic save file and convert it to a new profile.
#[tauri::command]
pub async fn import_legacy_save(path: String) -> Result<Profile, String> {
    service::library::import_legacy_save(path)
}

// ─── Discovery Commands ──────────────────────────────────────────────────

#[tauri::command]
pub async fn start_discovery(
    state: AppStateRef<'_>,
    profile_id: String,
    exclude_ids: Vec<String>,
    background: Option<bool>,
) -> Result<Vec<DiscoverySuggestion>, String> {
    service::discovery::start_discovery(
        &state,
        profile_id,
        exclude_ids,
        background.unwrap_or(false),
    )
    .await
}

#[tauri::command]
pub fn get_discovery_suggestions(
    profile_id: String,
) -> Result<Option<DiscoveryCacheResponse>, String> {
    Ok(service::discovery::cached_suggestions(&profile_id))
}

#[tauri::command]
pub fn save_discovery_cursor(
    profile_id: String,
    cursor_index: usize,
    revealed_count: usize,
    visited_index: i32,
) -> Result<(), String> {
    discovery::cache::DiscoveryCache::save_cursor(
        &profile_id,
        cursor_index,
        revealed_count,
        visited_index,
    )
}

#[tauri::command]
pub fn update_discovery_pool(
    profile_id: String,
    suggestions: Vec<DiscoverySuggestion>,
    cursor_index: usize,
    revealed_count: usize,
    visited_index: i32,
) -> Result<(), String> {
    discovery::cache::DiscoveryCache::update_pool(
        &profile_id,
        suggestions,
        cursor_index,
        revealed_count,
        visited_index,
    )
}

#[tauri::command]
pub async fn dismiss_discovery(
    state: AppStateRef<'_>,
    profile_id: String,
    video_id: String,
) -> Result<(), String> {
    let state = Arc::clone(&state);
    blocking(move || service::discovery::dismiss(&state, &profile_id, video_id)).await
}

#[tauri::command]
pub fn dislike_discovery(
    state: AppStateRef<'_>,
    profile_id: String,
    video_id: String,
) -> Result<(), String> {
    service::discovery::dislike(&state, &profile_id, video_id)
}

#[tauri::command]
pub fn undislike_discovery(profile_id: String, video_id: String) -> Result<(), String> {
    service::discovery::undislike(&profile_id, &video_id)
}

#[tauri::command]
pub async fn list_disliked_videos(profile_id: String) -> Result<Vec<DislikedVideoInfo>, String> {
    service::discovery::list_disliked_videos(profile_id).await
}

#[tauri::command]
pub fn cancel_discovery(state: AppStateRef<'_>) {
    service::discovery::cancel_discovery(&state);
}

/// Pre-download a suggestion's audio to cache WITHOUT adding it to the profile.
#[tauri::command]
pub async fn predownload_suggestion(
    state: AppStateRef<'_>,
    url: String,
    video_id: String,
    download_id: String,
) -> Result<PredownloadResult, String> {
    service::youtube::predownload_suggestion(&state, url, video_id, download_id).await
}

// ─── Error Handling Commands ──────────────────────────────────────────────

/// Verify that all sound files in a profile exist on disk.
#[tauri::command]
pub fn verify_profile_sounds(profile: Profile) -> Vec<MissingSoundInfo> {
    service::library::verify_profile_sounds(&profile)
}

const AUDIO_EXTENSIONS: &[&str] = &["mp3", "wav", "ogg", "flac", "m4a", "aac"];

/// Open a file picker dialog for selecting an audio file.
#[tauri::command]
pub async fn pick_audio_file() -> Result<Option<String>, String> {
    file_dialog(|| {
        rfd::FileDialog::new()
            .add_filter("Audio Files", AUDIO_EXTENSIONS)
            .pick_file()
            .map(path_string)
    })
    .await
}

/// Open a file picker dialog for selecting multiple audio files.
#[tauri::command]
pub async fn pick_audio_files() -> Result<Vec<String>, String> {
    file_dialog(|| {
        rfd::FileDialog::new()
            .add_filter("Audio Files", AUDIO_EXTENSIONS)
            .pick_files()
            .unwrap_or_default()
            .into_iter()
            .map(path_string)
            .collect::<Vec<_>>()
    })
    .await
}

#[tauri::command]
pub fn get_logs_folder() -> Result<String, String> {
    Ok(service::library::logs_folder())
}

#[tauri::command]
pub fn get_data_folder() -> Result<String, String> {
    Ok(service::library::data_folder())
}

#[tauri::command]
pub fn open_folder(path: String) -> Result<(), String> {
    service::library::open_folder(&path)
}

// ─── Startup Command ──────────────────────────────────────────────────────

/// Config + profile list + current profile in a single round-trip.
#[tauri::command]
pub fn get_initial_state(state: AppStateRef<'_>) -> Result<InitialState, String> {
    service::library::initial_state(&state)
}

// ─── Mood AI Commands ────────────────────────────────────────────────────────

#[tauri::command]
pub fn check_llama_server_installed() -> bool {
    mood::llama_manager::is_llama_server_installed()
}

#[tauri::command]
pub async fn install_llama_server() -> Result<String, String> {
    service::mood::install_llama_server().await
}

#[tauri::command]
pub fn check_mood_model_installed() -> bool {
    mood::llama_manager::is_model_downloaded()
}

#[tauri::command]
pub async fn install_mood_model(state: AppStateRef<'_>) -> Result<String, String> {
    service::mood::install_model(&state).await
}

#[tauri::command]
pub async fn start_mood_server(state: AppStateRef<'_>) -> Result<(), String> {
    service::mood::start_server(&state).await
}

#[tauri::command]
pub async fn stop_mood_server(state: AppStateRef<'_>) -> Result<(), String> {
    service::mood::stop_server(&state).await;
    Ok(())
}

#[tauri::command]
pub async fn get_mood_server_status(state: AppStateRef<'_>) -> Result<String, String> {
    Ok(service::mood::runtime_status(&state).await)
}

#[tauri::command]
pub async fn get_mood_service_status(state: AppStateRef<'_>) -> Result<MoodServiceStatus, String> {
    Ok(service::mood::service_status(&state).await)
}
