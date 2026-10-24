//! Process startup shared by every host: logging, config, state and the
//! background services (audio engine, key detection, periodic flushes).
//!
//! Typical host sequence:
//! ```ignore
//! let _guard = runtime::init_logging();
//! runtime::init_directories();
//! let state = Arc::new(runtime::build_state(runtime::load_config()));
//! state.set_event_sink(my_sink);           // once the UI can receive events
//! runtime::spawn_audio_engine(&state, error_sound_candidates);
//! runtime::start_key_detection(&state);
//! runtime::spawn_background_maintenance(&state);
//! ```

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::audio::engine::AudioEvent;
use crate::audio::AudioEngineHandle;
use crate::events::{CoreEvent, EventSink};
use crate::keys::{KeyDetector, KeyEvent};
use crate::state::AppState;
use crate::types::AppConfig;
use crate::{import_export, storage, youtube};

/// Initialize tracing: daily rolling files in `{app_data}/logs/` plus stderr.
/// Keep the returned guard alive for the whole process.
pub fn init_logging() -> tracing_appender::non_blocking::WorkerGuard {
    let logs_dir = storage::get_app_data_dir().join("logs");
    let _ = std::fs::create_dir_all(&logs_dir);

    let file_appender = tracing_appender::rolling::daily(&logs_dir, "keytomusic.log");
    let (non_blocking_file, guard) = tracing_appender::non_blocking(file_appender);

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

pub fn init_directories() {
    if let Err(e) = storage::init_app_directories() {
        tracing::error!("Failed to initialize app directories: {}", e);
    }
}

/// Load the config (defaults on failure) while cleaning up an interrupted export in parallel.
pub fn load_config() -> AppConfig {
    std::thread::scope(|s| {
        s.spawn(import_export::cleanup_interrupted_export);
        let config_handle = s.spawn(|| {
            storage::load_config().unwrap_or_else(|e| {
                tracing::warn!("Failed to load config, using defaults: {}", e);
                // Keep the unreadable file before the defaults get saved over it.
                let path = storage::get_app_data_dir().join("config.json");
                if path.exists() {
                    if let Err(e) = std::fs::copy(&path, path.with_extension("json.bad")) {
                        tracing::warn!("Failed to back up config.json: {}", e);
                    }
                }
                AppConfig::default()
            })
        });
        config_handle.join().unwrap()
    })
}

/// Build the shared state. The audio engine is not started yet (see [`spawn_audio_engine`]).
pub fn build_state(config: AppConfig) -> AppState {
    let key_detector = KeyDetector::new(
        config.key_cooldown,
        config.stop_all_shortcut.clone(),
        config.chord_window_ms,
    );
    key_detector.set_enabled(config.key_detection_enabled);
    key_detector.set_key_detection_shortcut(config.key_detection_shortcut.clone());
    key_detector.set_auto_momentum_shortcut(config.auto_momentum_shortcut.clone());
    key_detector.set_pause_shortcut(config.pause_shortcut.clone());

    // Index load deferred to first access via ensure_loaded
    let youtube_cache = youtube::YouTubeCache::new();

    AppState::new(config, key_detector, youtube_cache)
}

fn forward_audio_event(sink: &dyn EventSink, event: AudioEvent) {
    let event = match event {
        AudioEvent::SoundStarted { track_id, sound_id } => {
            CoreEvent::SoundStarted { track_id, sound_id }
        }
        AudioEvent::SoundEnded {
            track_id,
            sound_id,
            reason,
        } => CoreEvent::SoundEnded {
            track_id,
            sound_id,
            reason,
        },
        AudioEvent::PlaybackProgress { track_id, position } => {
            CoreEvent::PlaybackProgress { track_id, position }
        }
        AudioEvent::PauseChanged { paused } => CoreEvent::PauseChanged { paused },
        AudioEvent::Error { message } => {
            tracing::error!("[audio] {}", message);
            CoreEvent::AudioError { message }
        }
    };
    sink.emit(event);
}

/// Start the audio engine on a background thread and forward its events to the sink.
/// The first existing path of `error_sound_candidates` is used as the "missing file" sound.
pub fn spawn_audio_engine(state: &Arc<AppState>, error_sound_candidates: Vec<PathBuf>) {
    let state = state.clone();
    std::thread::spawn(move || {
        tracing::info!("Audio engine init starting (deferred)");
        let config = state.get_config();
        match AudioEngineHandle::new(config.audio_device.clone()) {
            Ok(engine) => {
                let _ = engine.set_master_volume(config.master_volume);
                engine.set_stop_fade(config.stop_fade_ms);

                if let Some(path) = error_sound_candidates.into_iter().find(|p| p.exists()) {
                    let _ = engine.set_error_sound_path(path.to_string_lossy().to_string());
                    tracing::info!("Error sound loaded: {:?}", path);
                }

                let event_rx = engine.event_rx.clone();
                let sink = state.event_sink();
                std::thread::spawn(move || {
                    let rx = event_rx.lock().unwrap();
                    loop {
                        match rx.recv_timeout(Duration::from_millis(50)) {
                            Ok(event) => {
                                forward_audio_event(sink.as_ref(), event);
                                while let Ok(event) = rx.try_recv() {
                                    forward_audio_event(sink.as_ref(), event);
                                }
                            }
                            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                        }
                    }
                });

                let _ = state.audio_engine.set(engine);
                tracing::info!("Audio engine initialized (deferred)");
            }
            Err(e) => {
                tracing::error!("Failed to initialize audio engine: {}", e);
            }
        }
    });
}

/// Start global key capture. Stop-all is handled here directly; other key events
/// are forwarded to the sink.
pub fn start_key_detection(state: &Arc<AppState>) {
    let audio_cell = state.audio_engine.clone();
    let sink = state.event_sink();
    state.key_detector.start(move |event| match event {
        KeyEvent::KeyPressed {
            key_code,
            with_shift,
        } => sink.emit(CoreEvent::KeyPressed {
            key_code,
            with_shift,
        }),
        KeyEvent::StopAll => {
            if let Some(engine) = audio_cell.get() {
                let _ = engine.stop_all();
            }
            sink.emit(CoreEvent::StopAllTriggered);
        }
        // The engine reports the new state itself (PauseChanged).
        KeyEvent::TogglePause => {
            if let Some(engine) = audio_cell.get() {
                let _ = engine.set_paused(!engine.is_paused());
            }
        }
        KeyEvent::ToggleKeyDetection => sink.emit(CoreEvent::ToggleKeyDetection),
        KeyEvent::ToggleAutoMomentum => sink.emit(CoreEvent::ToggleAutoMomentum),
        KeyEvent::BackendWarning { message } => {
            sink.emit(CoreEvent::KeyDetectionBackendWarning { message })
        }
    });
}

/// Periodic config (2s) and waveform cache (5s) flushes, plus the deferred
/// yt-dlp update and YouTube cache cleanup 5s after startup.
pub fn spawn_background_maintenance(state: &Arc<AppState>) {
    {
        let state = state.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_secs(2));
            if let Err(e) = state.flush_config() {
                tracing::warn!("Failed to flush config: {}", e);
            }
        });
    }

    {
        let waveform_cache = state.waveform_cache.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_secs(5));
            if let Ok(mut cache) = waveform_cache.lock() {
                cache.flush_if_dirty();
            }
        });
    }

    {
        let yt_cache = state.youtube_cache.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(5));

            // Update an existing installation; first installation is requested by a YouTube action.
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            if let Ok(rt) = rt {
                if youtube::yt_dlp_manager::find_yt_dlp().is_some() {
                    rt.block_on(youtube::ensure_yt_dlp_up_to_date());
                }
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
}
