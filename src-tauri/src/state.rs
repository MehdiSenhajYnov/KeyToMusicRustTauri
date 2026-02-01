use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use crate::audio::AudioEngineHandle;
use crate::audio::analysis::WaveformCache;
use crate::keys::KeyDetector;
use crate::types::AppConfig;
use crate::youtube::YouTubeCache;

/// Application state managed by Tauri.
/// Wrapped in Mutex for thread-safe access from multiple command handlers.
pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub audio_engine: AudioEngineHandle,
    pub key_detector: KeyDetector,
    pub youtube_cache: Arc<Mutex<YouTubeCache>>,
    pub waveform_cache: Arc<Mutex<WaveformCache>>,
    pub discovery_cancel: Arc<AtomicBool>,
}

impl AppState {
    pub fn new(config: AppConfig, audio_engine: AudioEngineHandle, key_detector: KeyDetector, youtube_cache: YouTubeCache) -> Self {
        Self {
            config: Mutex::new(config),
            audio_engine,
            key_detector,
            youtube_cache: Arc::new(Mutex::new(youtube_cache)),
            waveform_cache: Arc::new(Mutex::new(WaveformCache::new(50))),
            discovery_cancel: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get a clone of the current config.
    pub fn get_config(&self) -> AppConfig {
        self.config.lock().unwrap().clone()
    }

    /// Update the config with a closure and return the new config.
    pub fn update_config<F>(&self, updater: F) -> AppConfig
    where
        F: FnOnce(&mut AppConfig),
    {
        let mut config = self.config.lock().unwrap();
        updater(&mut config);
        config.clone()
    }
}
