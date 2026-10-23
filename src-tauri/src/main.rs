// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod keyboard_layout;

use std::sync::Arc;

use commands::*;
use keytomusic_core::{mood, runtime, AppState, CoreEvent};
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

fn main() {
    let _log_guard = runtime::init_logging();
    tracing::info!("KeyToMusic starting up");

    #[cfg(target_os = "linux")]
    apply_linux_webkit_workarounds();

    runtime::init_directories();

    if std::env::var_os("KEYTOMUSIC_RUN_REALTEST_BENCHMARK").is_some() {
        tracing::info!("Running in RealTest benchmark CLI mode");
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime for RealTest benchmark");
        runtime.block_on(mood::director_realtest_suite::run());
        return;
    }

    // Audio engine and key detection start in the setup hook, once events can reach the webview.
    let app_state: Arc<AppState> = Arc::new(runtime::build_state(runtime::load_config()));
    runtime::spawn_background_maintenance(&app_state);

    tauri::Builder::default()
        // Allow rdev (global keyboard hook) to receive events even when window is focused
        .device_event_filter(DeviceEventFilter::Never)
        .plugin(tauri_plugin_shell::init())
        .manage(app_state.clone())
        .setup(move |app| {
            keyboard_layout::watch(app.handle());

            let app_handle = app.handle().clone();
            app_state.set_event_sink(move |event: CoreEvent| {
                let _ = app_handle.emit(event.name(), event.payload());
            });

            let error_sounds = app
                .path()
                .resource_dir()
                .map(|dir| {
                    vec![
                        dir.join("resources/sounds/error.mp3"),
                        dir.join("error.mp3"),
                    ]
                })
                .unwrap_or_default();
            runtime::spawn_audio_engine(&app_state, error_sounds);
            runtime::start_key_detection(&app_state);

            // Clear pressed keys on window focus change to prevent stuck modifiers (Alt+Tab)
            if let Some(window) = app.get_webview_window("main") {
                let key_detector = app_state.key_detector.clone();
                window.on_window_event(move |event| {
                    if matches!(event, tauri::WindowEvent::Focused(_)) {
                        key_detector.clear_pressed_keys();
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Config commands
            get_config,
            flush_config,
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
            suspend_key_detection,
            set_key_detection_typing,
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
            get_keyboard_layout,
            // Mood AI commands
            check_llama_server_installed,
            install_llama_server,
            check_mood_model_installed,
            install_mood_model,
            start_mood_server,
            stop_mood_server,
            get_mood_server_status,
            get_mood_service_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
