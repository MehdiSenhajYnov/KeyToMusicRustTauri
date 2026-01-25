// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod commands;
mod errors;
mod import_export;
mod keys;
mod state;
mod storage;
mod types;
mod youtube;

use audio::{AudioEngineHandle, engine::AudioEvent};
use commands::*;
use keys::{KeyDetector, KeyEvent};
use state::AppState;
use tauri::{Emitter, Manager};
use std::time::Duration;

/// Initialize the tracing/logging system.
/// Logs are written to daily rolling files in `{app_data}/logs/`.
/// Returns the guard that must be kept alive for the duration of the program.
fn init_logging() -> tracing_appender::non_blocking::WorkerGuard {
    let logs_dir = storage::get_app_data_dir().join("logs");
    let _ = std::fs::create_dir_all(&logs_dir);

    let file_appender = tracing_appender::rolling::daily(&logs_dir, "keytomusic.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_ansi(false)
        .init();

    guard
}

fn main() {
    // Initialize logging
    let _log_guard = init_logging();
    tracing::info!("KeyToMusic starting up");

    // Initialize app directories
    if let Err(e) = storage::init_app_directories() {
        tracing::error!("Failed to initialize app directories: {}", e);
    }

    // Clean up any orphaned temp file from a previously interrupted export
    import_export::cleanup_interrupted_export();

    // Load config (or create default)
    let config = storage::load_config().unwrap_or_else(|e| {
        tracing::warn!("Failed to load config, using defaults: {}", e);
        types::AppConfig::default()
    });

    // Initialize audio engine with configured device
    let audio_engine = AudioEngineHandle::new(config.audio_device.clone()).unwrap_or_else(|e| {
        tracing::error!("Failed to initialize audio engine: {}", e);
        panic!("Audio engine initialization failed: {}", e);
    });

    // Set initial master volume from config
    let _ = audio_engine.set_master_volume(config.master_volume);

    // Initialize key detector
    let key_detector = KeyDetector::new(
        config.key_cooldown,
        config.master_stop_shortcut.clone(),
        config.chord_window_ms,
    );

    // Set initial enabled state from config
    key_detector.set_enabled(config.key_detection_enabled);
    key_detector.set_key_detection_shortcut(config.key_detection_shortcut.clone());
    key_detector.set_auto_momentum_shortcut(config.auto_momentum_shortcut.clone());

    // Initialize YouTube cache
    let mut youtube_cache = youtube::YouTubeCache::new();
    if let Err(e) = youtube_cache.load_index() {
        tracing::warn!("Failed to load YouTube cache index: {}", e);
    }
    youtube_cache.verify_integrity();
    youtube_cache.cleanup_unused();
    youtube_cache.save_index().ok();

    // Create app state
    let app_state = AppState::new(config, audio_engine, key_detector.clone(), youtube_cache);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .setup(move |app| {
            // Resolve and set error sound path
            let error_sound_path = if let Ok(resource_dir) = app.path().resource_dir() {
                // Try multiple possible locations for the bundled resource
                let candidates = [
                    resource_dir.join("resources/sounds/error.mp3"),
                    resource_dir.join("error.mp3"),
                ];
                candidates.into_iter().find(|p| p.exists())
            } else {
                None
            };
            if let Some(path) = error_sound_path {
                let state: tauri::State<'_, AppState> = app.state();
                let _ = state.audio_engine.set_error_sound_path(
                    path.to_string_lossy().to_string(),
                );
                tracing::info!("Error sound loaded: {:?}", path);
            } else {
                tracing::warn!("Error sound not found in resource directory");
            }

            // Start key detection with Tauri event emitter
            let app_handle = app.handle().clone();

            key_detector.start(move |event| {
                match event {
                    KeyEvent::KeyPressed { key_code, with_shift } => {
                        let _ = app_handle.emit("key_pressed", serde_json::json!({
                            "keyCode": key_code,
                            "withShift": with_shift,
                        }));
                    }
                    KeyEvent::MasterStop => {
                        let state: tauri::State<'_, AppState> = app_handle.state();
                        let _ = state.audio_engine.stop_all();
                        let _ = app_handle.emit("master_stop_triggered", serde_json::json!({}));
                    }
                    KeyEvent::ToggleKeyDetection => {
                        let _ = app_handle.emit("toggle_key_detection", serde_json::json!({}));
                    }
                    KeyEvent::ToggleAutoMomentum => {
                        let _ = app_handle.emit("toggle_auto_momentum", serde_json::json!({}));
                    }
                }
            });

            // Start audio event polling thread
            let app_handle_audio = app.handle().clone();
            let audio_events = {
                let state: tauri::State<'_, AppState> = app.state();
                state.audio_engine.events.clone()
            };

            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(Duration::from_millis(100));
                    let events: Vec<AudioEvent> = {
                        let mut lock = audio_events.lock().unwrap();
                        lock.drain(..).collect()
                    };
                    for event in events {
                        match event {
                            AudioEvent::SoundStarted { track_id, sound_id } => {
                                let _ = app_handle_audio.emit("sound_started", serde_json::json!({
                                    "trackId": track_id,
                                    "soundId": sound_id,
                                }));
                            }
                            AudioEvent::SoundEnded { track_id, sound_id } => {
                                let _ = app_handle_audio.emit("sound_ended", serde_json::json!({
                                    "trackId": track_id,
                                    "soundId": sound_id,
                                }));
                            }
                            AudioEvent::PlaybackProgress { track_id, position } => {
                                let _ = app_handle_audio.emit("playback_progress", serde_json::json!({
                                    "trackId": track_id,
                                    "position": position,
                                }));
                            }
                            AudioEvent::SoundNotFound { track_id, sound_id, file_path } => {
                                tracing::warn!("Sound not found: {} (track: {}, sound: {})", file_path, track_id, sound_id);
                                let _ = app_handle_audio.emit("sound_not_found", serde_json::json!({
                                    "soundId": sound_id,
                                    "path": file_path,
                                    "trackId": track_id,
                                }));
                            }
                            AudioEvent::Error { message } => {
                                tracing::error!("[audio] {}", message);
                                let _ = app_handle_audio.emit("audio_error", serde_json::json!({
                                    "message": message,
                                }));
                            }
                        }
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Config commands
            get_config,
            update_config,
            // Profile commands
            list_profiles,
            create_profile,
            load_profile,
            save_profile,
            delete_profile,
            duplicate_profile,
            // Audio commands
            play_sound,
            stop_sound,
            stop_all_sounds,
            set_master_volume,
            set_track_volume,
            set_sound_volume,
            get_audio_duration,
            preload_profile_sounds,
            // Key detection commands
            set_key_detection,
            set_master_stop_shortcut,
            set_key_cooldown,
            set_profile_bindings,
            // YouTube commands
            add_sound_from_youtube,
            check_yt_dlp_installed,
            install_yt_dlp,
            check_ffmpeg_installed,
            install_ffmpeg,
            // Audio device commands
            list_audio_devices,
            set_audio_device,
            // Import/Export commands
            export_profile,
            import_profile,
            pick_save_location,
            pick_ktm_file,
            cleanup_export_temp,
            cancel_export,
            // Legacy import commands
            pick_legacy_file,
            import_legacy_save,
            // Error handling commands
            verify_profile_sounds,
            pick_audio_file,
            pick_audio_files,
            get_logs_folder,
            get_data_folder,
            open_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
