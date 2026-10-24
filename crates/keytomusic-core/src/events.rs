//! Events emitted by the core towards whichever UI is hosting it.
//!
//! The Tauri shell forwards them as named JSON events (see [`CoreEvent::name`]
//! and [`CoreEvent::payload`], which keep the historical wire format), while a
//! native shell can match on the enum directly.

use std::sync::Arc;

use crate::audio::engine::SoundEndedReason;
use crate::discovery::engine::DiscoverySuggestion;

#[derive(Debug, Clone)]
pub enum CoreEvent {
    SoundStarted {
        track_id: String,
        sound_id: String,
    },
    SoundEnded {
        track_id: String,
        sound_id: String,
        reason: SoundEndedReason,
    },
    PlaybackProgress {
        track_id: String,
        position: f64,
    },
    AudioError {
        message: String,
    },
    SoundNotFound {
        sound_id: String,
        path: String,
        track_id: String,
    },
    KeyPressed {
        key_code: String,
        with_shift: bool,
    },
    StopAllTriggered,
    /// Playback was paused or resumed as a whole.
    PauseChanged {
        paused: bool,
    },
    ToggleKeyDetection,
    ToggleAutoMomentum,
    KeyDetectionBackendWarning {
        message: String,
    },
    YoutubeDownloadProgress {
        download_id: String,
        status: String,
        progress: Option<f64>,
    },
    ExportProgress {
        current: usize,
        total: usize,
        filename: String,
    },
    WaveformProgress {
        path: String,
        points: Vec<f32>,
        duration: f64,
        sample_rate: u32,
    },
    DiscoveryResolving {
        count: usize,
    },
    DiscoveryStarted,
    DiscoveryProgress {
        current: usize,
        total: usize,
        seed_name: String,
    },
    DiscoveryPartial(Vec<DiscoverySuggestion>),
    DiscoveryComplete {
        count: usize,
    },
    MoodModelDownloadProgress {
        downloaded: u64,
        total: u64,
    },
    MoodServerStatus {
        status: String,
    },
    MoodApiStatus {
        status: String,
        enabled: bool,
        port: u16,
    },
    MoodDetected {
        mood: String,
        intensity: u8,
        source: String,
    },
    MoodCommitted {
        mood: String,
        intensity: u8,
        source: String,
        previous_mood: Option<String>,
        previous_intensity: Option<u8>,
        dwell_count: u32,
        mood_changed: bool,
        intensity_changed: bool,
    },
}

impl CoreEvent {
    /// Wire name used by the Tauri frontend.
    pub fn name(&self) -> &'static str {
        match self {
            CoreEvent::SoundStarted { .. } => "sound_started",
            CoreEvent::SoundEnded { .. } => "sound_ended",
            CoreEvent::PlaybackProgress { .. } => "playback_progress",
            CoreEvent::AudioError { .. } => "audio_error",
            CoreEvent::SoundNotFound { .. } => "sound_not_found",
            CoreEvent::KeyPressed { .. } => "key_pressed",
            CoreEvent::StopAllTriggered => "stop_all_triggered",
            CoreEvent::PauseChanged { .. } => "pause_changed",
            CoreEvent::ToggleKeyDetection => "toggle_key_detection",
            CoreEvent::ToggleAutoMomentum => "toggle_auto_momentum",
            CoreEvent::KeyDetectionBackendWarning { .. } => "key_detection_backend_warning",
            CoreEvent::YoutubeDownloadProgress { .. } => "youtube_download_progress",
            CoreEvent::ExportProgress { .. } => "export_progress",
            CoreEvent::WaveformProgress { .. } => "waveform_progress",
            CoreEvent::DiscoveryResolving { .. } => "discovery_resolving",
            CoreEvent::DiscoveryStarted => "discovery_started",
            CoreEvent::DiscoveryProgress { .. } => "discovery_progress",
            CoreEvent::DiscoveryPartial(_) => "discovery_partial",
            CoreEvent::DiscoveryComplete { .. } => "discovery_complete",
            CoreEvent::MoodModelDownloadProgress { .. } => "mood_model_download_progress",
            CoreEvent::MoodServerStatus { .. } => "mood_server_status",
            CoreEvent::MoodApiStatus { .. } => "mood_api_status",
            CoreEvent::MoodDetected { .. } => "mood_detected",
            CoreEvent::MoodCommitted { .. } => "mood_committed",
        }
    }

    /// JSON payload in the exact shape the Tauri frontend expects.
    pub fn payload(&self) -> serde_json::Value {
        use serde_json::json;
        match self {
            CoreEvent::SoundStarted { track_id, sound_id } => {
                json!({ "trackId": track_id, "soundId": sound_id })
            }
            CoreEvent::SoundEnded {
                track_id,
                sound_id,
                reason,
            } => json!({ "trackId": track_id, "soundId": sound_id, "reason": reason.as_str() }),
            CoreEvent::PlaybackProgress { track_id, position } => {
                json!({ "trackId": track_id, "position": position })
            }
            CoreEvent::AudioError { message }
            | CoreEvent::KeyDetectionBackendWarning { message } => {
                json!({ "message": message })
            }
            CoreEvent::SoundNotFound {
                sound_id,
                path,
                track_id,
            } => json!({ "soundId": sound_id, "path": path, "trackId": track_id }),
            CoreEvent::KeyPressed {
                key_code,
                with_shift,
            } => json!({ "keyCode": key_code, "withShift": with_shift }),
            CoreEvent::PauseChanged { paused } => json!({ "paused": paused }),
            CoreEvent::StopAllTriggered
            | CoreEvent::ToggleKeyDetection
            | CoreEvent::ToggleAutoMomentum
            | CoreEvent::DiscoveryStarted => json!({}),
            CoreEvent::YoutubeDownloadProgress {
                download_id,
                status,
                progress,
            } => json!({ "downloadId": download_id, "status": status, "progress": progress }),
            CoreEvent::ExportProgress {
                current,
                total,
                filename,
            } => json!({ "current": current, "total": total, "filename": filename }),
            CoreEvent::WaveformProgress {
                path,
                points,
                duration,
                sample_rate,
            } => json!({
                "path": path,
                "points": points,
                "duration": duration,
                "sampleRate": sample_rate,
            }),
            CoreEvent::DiscoveryResolving { count } | CoreEvent::DiscoveryComplete { count } => {
                json!({ "count": count })
            }
            CoreEvent::DiscoveryProgress {
                current,
                total,
                seed_name,
            } => json!({ "current": current, "total": total, "seedName": seed_name }),
            CoreEvent::DiscoveryPartial(suggestions) => {
                serde_json::to_value(suggestions).unwrap_or(serde_json::Value::Null)
            }
            CoreEvent::MoodModelDownloadProgress { downloaded, total } => {
                json!({ "downloaded": downloaded, "total": total })
            }
            CoreEvent::MoodServerStatus { status } => json!({ "status": status }),
            CoreEvent::MoodApiStatus {
                status,
                enabled,
                port,
            } => json!({ "status": status, "enabled": enabled, "port": port }),
            CoreEvent::MoodDetected {
                mood,
                intensity,
                source,
            } => json!({ "mood": mood, "intensity": intensity, "source": source }),
            CoreEvent::MoodCommitted {
                mood,
                intensity,
                source,
                previous_mood,
                previous_intensity,
                dwell_count,
                mood_changed,
                intensity_changed,
            } => json!({
                "mood": mood,
                "intensity": intensity,
                "source": source,
                "previous_mood": previous_mood,
                "previous_intensity": previous_intensity,
                "dwell_count": dwell_count,
                "mood_changed": mood_changed,
                "intensity_changed": intensity_changed,
            }),
        }
    }
}

/// Receiver of core events. Implementations must be cheap and non-blocking:
/// events are emitted from audio, key-detection and tokio threads.
pub trait EventSink: Send + Sync + 'static {
    fn emit(&self, event: CoreEvent);
}

/// Sink used until the host UI installs its own.
pub struct NoopSink;

impl EventSink for NoopSink {
    fn emit(&self, event: CoreEvent) {
        tracing::debug!(
            "Dropped core event before a sink was installed: {}",
            event.name()
        );
    }
}

impl<F> EventSink for F
where
    F: Fn(CoreEvent) + Send + Sync + 'static,
{
    fn emit(&self, event: CoreEvent) {
        self(event)
    }
}

pub type SharedSink = Arc<dyn EventSink>;
