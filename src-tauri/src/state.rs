use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, Mutex};

use crate::audio::analysis::WaveformCache;
use crate::audio::AudioEngineHandle;
use crate::keys::KeyDetector;
use crate::mood::cache::MoodCache;
use crate::mood::director::{DirectorConfig, MoodDirector};
use crate::mood::inference::LlamaServer;
use crate::storage;
use crate::types::AppConfig;
use crate::youtube::YouTubeCache;

/// Application state managed by Tauri.
/// Wrapped in Mutex for thread-safe access from multiple command handlers.
pub struct AppState {
    pub config: Mutex<AppConfig>,
    /// Audio engine initialized on a background thread after window creation.
    /// Commands that need audio should call `get_audio_engine()`.
    pub audio_engine: Arc<std::sync::OnceLock<AudioEngineHandle>>,
    pub key_detector: KeyDetector,
    pub youtube_cache: Arc<Mutex<YouTubeCache>>,
    pub waveform_cache: Arc<Mutex<WaveformCache>>,
    pub discovery_cancel: Arc<AtomicBool>,
    /// Shared Rayon thread pool (4 threads) for CPU-bound audio operations
    /// (waveform computation, duration reading). Limits concurrent CPU work.
    pub cpu_pool: Arc<rayon::ThreadPool>,
    /// Generation counter for profile loads. Incremented each time a profile
    /// is preloaded. CPU-bound batch operations check this to bail early
    /// when a newer load has started.
    pub profile_load_gen: Arc<AtomicU64>,
    /// Dirty flag for debounced config writes (flushed every 2s by background thread).
    pub config_dirty: Arc<AtomicBool>,
    /// Running llama-server instance for mood inference.
    pub llama_server: Arc<tokio::sync::Mutex<Option<LlamaServer>>>,
    /// Running mood API HTTP server handle.
    pub mood_api_server: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Ephemeral mood cache for pre-calculated page moods.
    pub mood_cache: Arc<Mutex<MoodCache>>,
    /// MoodDirector for contextual mood transitions (sliding window + hysteresis).
    pub mood_director: Arc<Mutex<MoodDirector>>,
}

impl AppState {
    pub fn new(config: AppConfig, key_detector: KeyDetector, youtube_cache: YouTubeCache) -> Self {
        let cache_path = storage::get_app_data_dir()
            .join("cache")
            .join("waveforms.json");
        let cpu_pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                .num_threads(4)
                .thread_name(|i| format!("cpu-pool-{}", i))
                .build()
                .expect("Failed to build CPU thread pool"),
        );
        tracing::info!("CPU thread pool created (4 threads)");
        let director_config = DirectorConfig {
            entry_threshold: config.mood_entry_threshold,
            exit_threshold: config.mood_exit_threshold,
            min_dwell_pages: config.mood_dwell_pages,
            window_size: config.mood_window_size,
        };
        Self {
            config: Mutex::new(config),
            audio_engine: Arc::new(std::sync::OnceLock::new()),
            key_detector,
            youtube_cache: Arc::new(Mutex::new(youtube_cache)),
            waveform_cache: Arc::new(Mutex::new(WaveformCache::new_with_disk(50, cache_path))),
            discovery_cancel: Arc::new(AtomicBool::new(false)),
            cpu_pool,
            profile_load_gen: Arc::new(AtomicU64::new(0)),
            config_dirty: Arc::new(AtomicBool::new(false)),
            llama_server: Arc::new(tokio::sync::Mutex::new(None)),
            mood_api_server: Arc::new(Mutex::new(None)),
            mood_cache: Arc::new(Mutex::new(MoodCache::new())),
            mood_director: Arc::new(Mutex::new(MoodDirector::new(director_config))),
        }
    }

    /// Get the audio engine, returning an error if not yet initialized.
    pub fn get_audio_engine(&self) -> Result<&AudioEngineHandle, String> {
        self.audio_engine
            .get()
            .ok_or_else(|| "Audio engine is still initializing".to_string())
    }

    /// Get a clone of the current config.
    pub fn get_config(&self) -> AppConfig {
        self.config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Update the config with a closure and return the new config.
    pub fn update_config<F>(&self, updater: F) -> AppConfig
    where
        F: FnOnce(&mut AppConfig),
    {
        let mut config = self.config.lock().unwrap_or_else(|e| e.into_inner());
        updater(&mut config);
        config.clone()
    }

    /// Mark config as dirty for debounced saving (flushed every 2s by background thread).
    pub fn schedule_config_save(&self) {
        self.config_dirty
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// Flush config to disk if dirty. Called periodically by background thread.
    pub fn flush_config(&self) -> Result<(), String> {
        if self
            .config_dirty
            .swap(false, std::sync::atomic::Ordering::Relaxed)
        {
            let config = self.get_config();
            crate::storage::save_config(&config)?;
        }
        Ok(())
    }
}
