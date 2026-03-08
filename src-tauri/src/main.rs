// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod commands;
mod discovery;
mod import_export;
mod keys;
mod mood;
mod state;
mod storage;
mod types;
mod youtube;

use audio::{engine::AudioEvent, AudioEngineHandle};
use commands::*;
use keys::{KeyDetector, KeyEvent};
use state::AppState;
use std::time::Duration;
use tauri::{DeviceEventFilter, Emitter, Manager};

#[cfg(target_os = "linux")]
fn apply_linux_webkit_workarounds() {
    let is_wayland = std::env::var_os("WAYLAND_DISPLAY").is_some()
        || std::env::var("XDG_SESSION_TYPE")
            .map(|value| value.eq_ignore_ascii_case("wayland"))
            .unwrap_or(false);

    if is_wayland && std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        // WebKitGTK can crash on Wayland, especially on NVIDIA, unless dmabuf rendering is disabled.
        unsafe {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }
}

/// Initialize the tracing/logging system.
/// Logs are written to daily rolling files in `{app_data}/logs/`.
/// Returns the guard that must be kept alive for the duration of the program.
fn init_logging() -> tracing_appender::non_blocking::WorkerGuard {
    let logs_dir = storage::get_app_data_dir().join("logs");
    let _ = std::fs::create_dir_all(&logs_dir);

    let file_appender = tracing_appender::rolling::daily(&logs_dir, "keytomusic.log");
    let (non_blocking_file, guard) = tracing_appender::non_blocking(file_appender);

    // Write to both file and stderr (visible in `npm run tauri dev` terminal)
    use tracing_subscriber::fmt::writer::MakeWriterExt;
    let combined_writer = std::io::stderr.and(non_blocking_file);

    tracing_subscriber::fmt()
        .with_writer(combined_writer)
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

    #[cfg(target_os = "linux")]
    apply_linux_webkit_workarounds();

    // Initialize app directories
    if let Err(e) = storage::init_app_directories() {
        tracing::error!("Failed to initialize app directories: {}", e);
    }

    // Load config and clean up export temp in parallel
    let config = std::thread::scope(|s| {
        s.spawn(|| import_export::cleanup_interrupted_export());
        let config_handle = s.spawn(|| {
            storage::load_config().unwrap_or_else(|e| {
                tracing::warn!("Failed to load config, using defaults: {}", e);
                types::AppConfig::default()
            })
        });
        config_handle.join().unwrap()
    });

    // Initialize key detector
    let key_detector = KeyDetector::new(
        config.key_cooldown,
        config.stop_all_shortcut.clone(),
        config.chord_window_ms,
    );

    // Set initial enabled state from config
    key_detector.set_enabled(config.key_detection_enabled);
    key_detector.set_key_detection_shortcut(config.key_detection_shortcut.clone());
    key_detector.set_auto_momentum_shortcut(config.auto_momentum_shortcut.clone());

    // Initialize YouTube cache (load deferred to first access via ensure_loaded)
    let youtube_cache = youtube::YouTubeCache::new();

    // Save audio_device and master_volume before moving config into AppState
    let audio_device = config.audio_device.clone();
    let master_volume = config.master_volume;

    // Create app state (audio engine NOT yet initialized — deferred to setup hook)
    let app_state = AppState::new(config, key_detector.clone(), youtube_cache);

    // Defer YouTube cache cleanup + yt-dlp auto-update to a background thread (saves ~500ms startup)
    {
        let yt_cache = app_state.youtube_cache.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(5));

            // Auto-update yt-dlp (download if missing, re-download if > 7 days old)
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            if let Ok(rt) = rt {
                rt.block_on(youtube::ensure_yt_dlp_up_to_date());
            }

            if let Ok(mut cache) = yt_cache.lock() {
                cache.ensure_loaded();
                cache.verify_integrity();
                cache.cleanup_unused();
                cache.save_index().ok();
                tracing::info!("YouTube cache cleanup completed (deferred)");
            }
        });
    }

    tauri::Builder::default()
        // Allow rdev (global keyboard hook) to receive events even when window is focused
        .device_event_filter(DeviceEventFilter::Never)
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .setup(move |app| {
            // --- Deferred audio engine initialization (async, non-blocking) ---
            let audio_cell = {
                let state: tauri::State<'_, AppState> = app.state();
                state.audio_engine.clone()
            };
            let app_handle_audio_init = app.handle().clone();
            std::thread::spawn(move || {
                tracing::info!("Audio engine init starting (deferred)");
                match AudioEngineHandle::new(audio_device) {
                    Ok(engine) => {
                        let _ = engine.set_master_volume(master_volume);

                        // Resolve and set error sound path
                        if let Ok(resource_dir) = app_handle_audio_init.path().resource_dir() {
                            let candidates = [
                                resource_dir.join("resources/sounds/error.mp3"),
                                resource_dir.join("error.mp3"),
                            ];
                            if let Some(path) = candidates.into_iter().find(|p| p.exists()) {
                                let _ =
                                    engine.set_error_sound_path(path.to_string_lossy().to_string());
                                tracing::info!("Error sound loaded: {:?}", path);
                            }
                        }

                        // Start audio event forwarding thread
                        let event_rx = engine.event_rx.clone();
                        let app_handle_events = app_handle_audio_init.clone();
                        std::thread::spawn(move || {
                            let rx = event_rx.lock().unwrap();
                            loop {
                                match rx.recv_timeout(Duration::from_millis(50)) {
                                    Ok(event) => {
                                        emit_audio_event(&app_handle_events, event);
                                        while let Ok(event) = rx.try_recv() {
                                            emit_audio_event(&app_handle_events, event);
                                        }
                                    }
                                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                                }
                            }
                        });

                        // Store engine in OnceCell — commands can now use it
                        let _ = audio_cell.set(engine);
                        tracing::info!("Audio engine initialized (deferred)");
                    }
                    Err(e) => {
                        tracing::error!("Failed to initialize audio engine: {}", e);
                    }
                }
            });

            // Start key detection with Tauri event emitter
            let app_handle = app.handle().clone();
            // Need a reference to the audio_engine cell for StopAll
            let audio_cell_keys = {
                let state: tauri::State<'_, AppState> = app.state();
                state.audio_engine.clone()
            };

            key_detector.start(move |event| match event {
                KeyEvent::KeyPressed {
                    key_code,
                    with_shift,
                } => {
                    let _ = app_handle.emit(
                        "key_pressed",
                        serde_json::json!({
                            "keyCode": key_code,
                            "withShift": with_shift,
                        }),
                    );
                }
                KeyEvent::StopAll => {
                    if let Some(engine) = audio_cell_keys.get() {
                        let _ = engine.stop_all();
                    }
                    let _ = app_handle.emit("stop_all_triggered", serde_json::json!({}));
                }
                KeyEvent::ToggleKeyDetection => {
                    let _ = app_handle.emit("toggle_key_detection", serde_json::json!({}));
                }
                KeyEvent::ToggleAutoMomentum => {
                    let _ = app_handle.emit("toggle_auto_momentum", serde_json::json!({}));
                }
                KeyEvent::BackendWarning { message } => {
                    let _ = app_handle.emit(
                        "key_detection_backend_warning",
                        serde_json::json!({ "message": message }),
                    );
                }
            });

            // Clear pressed keys on window focus change to prevent stuck modifiers (Alt+Tab)
            {
                let key_detector_focus = key_detector.clone();
                if let Some(window) = app.get_webview_window("main") {
                    window.on_window_event(move |event| {
                        if matches!(event, tauri::WindowEvent::Focused(_)) {
                            key_detector_focus.clear_pressed_keys();
                        }
                    });
                }
            }

            // Config debounce flush thread (saves every 2s if dirty)
            {
                let app_handle_config = app.handle().clone();
                std::thread::spawn(move || loop {
                    std::thread::sleep(Duration::from_secs(2));
                    let state: tauri::State<'_, AppState> = app_handle_config.state();
                    if let Err(e) = state.flush_config() {
                        tracing::warn!("Failed to flush config: {}", e);
                    }
                });
            }

            // Waveform cache flush thread (saves every 5s if dirty)
            {
                let waveform_cache = {
                    let state: tauri::State<'_, AppState> = app.state();
                    state.waveform_cache.clone()
                };
                std::thread::spawn(move || loop {
                    std::thread::sleep(Duration::from_secs(5));
                    if let Ok(mut cache) = waveform_cache.lock() {
                        cache.flush_if_dirty();
                    }
                });
            }

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
            get_linux_input_access_status,
            enable_linux_background_detection,
            set_key_detection,
            set_stop_all_shortcut,
            set_key_cooldown,
            set_profile_bindings,
            // YouTube commands
            add_sound_from_youtube,
            search_youtube,
            fetch_playlist,
            get_youtube_stream_url,
            check_yt_dlp_installed,
            install_yt_dlp,
            check_ffmpeg_installed,
            install_ffmpeg,
            // Audio device commands
            list_audio_devices,
            set_audio_device,
            // Waveform commands
            get_waveform,
            get_waveforms_batch,
            // Import/Export commands
            export_profile,
            import_profile,
            pick_save_location,
            pick_ktm_file,
            cleanup_export_temp,
            cancel_export,
            // Discovery commands
            start_discovery,
            get_discovery_suggestions,
            dismiss_discovery,
            dislike_discovery,
            undislike_discovery,
            list_disliked_videos,
            cancel_discovery,
            predownload_suggestion,
            save_discovery_cursor,
            update_discovery_pool,
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
            // Startup command
            get_initial_state,
            // Mood AI commands
            check_llama_server_installed,
            install_llama_server,
            check_mood_model_installed,
            install_mood_model,
            start_mood_server,
            stop_mood_server,
            get_mood_server_status,
            get_mood_service_status,
            analyze_mood,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Emit an audio event to the frontend.
fn emit_audio_event(app_handle: &tauri::AppHandle, event: AudioEvent) {
    match event {
        AudioEvent::SoundStarted { track_id, sound_id } => {
            let _ = app_handle.emit(
                "sound_started",
                serde_json::json!({
                    "trackId": track_id,
                    "soundId": sound_id,
                }),
            );
        }
        AudioEvent::SoundEnded { track_id, sound_id } => {
            let _ = app_handle.emit(
                "sound_ended",
                serde_json::json!({
                    "trackId": track_id,
                    "soundId": sound_id,
                }),
            );
        }
        AudioEvent::PlaybackProgress { track_id, position } => {
            let _ = app_handle.emit(
                "playback_progress",
                serde_json::json!({
                    "trackId": track_id,
                    "position": position,
                }),
            );
        }
        AudioEvent::Error { message } => {
            tracing::error!("[audio] {}", message);
            let _ = app_handle.emit(
                "audio_error",
                serde_json::json!({
                    "message": message,
                }),
            );
        }
    }
}
