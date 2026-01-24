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

fn main() {
    // Initialize app directories
    if let Err(e) = storage::init_app_directories() {
        eprintln!("Failed to initialize app directories: {}", e);
    }

    // Clean up any orphaned temp file from a previously interrupted export
    import_export::cleanup_interrupted_export();

    // Load config (or create default)
    let config = storage::load_config().unwrap_or_else(|e| {
        eprintln!("Failed to load config, using defaults: {}", e);
        types::AppConfig::default()
    });

    // Initialize audio engine with configured device
    let audio_engine = AudioEngineHandle::new(config.audio_device.clone()).unwrap_or_else(|e| {
        eprintln!("Failed to initialize audio engine: {}", e);
        panic!("Audio engine initialization failed: {}", e);
    });

    // Set initial master volume from config
    let _ = audio_engine.set_master_volume(config.master_volume);

    // Initialize key detector
    let key_detector = KeyDetector::new(config.key_cooldown, config.master_stop_shortcut.clone());

    // Set initial enabled state from config
    key_detector.set_enabled(config.key_detection_enabled);
    key_detector.set_key_detection_shortcut(config.key_detection_shortcut.clone());
    key_detector.set_auto_momentum_shortcut(config.auto_momentum_shortcut.clone());

    // Initialize YouTube cache
    let mut youtube_cache = youtube::YouTubeCache::new();
    if let Err(e) = youtube_cache.load_index() {
        eprintln!("Failed to load YouTube cache index: {}", e);
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
                            AudioEvent::Error { message } => {
                                eprintln!("[audio] Error: {}", message);
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
