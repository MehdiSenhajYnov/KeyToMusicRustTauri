use crate::audio::analysis::{self, WaveformData};
use crate::audio::buffer::BufferManager;
use crate::events::CoreEvent;
use crate::state::AppState;
use crate::types::{Sound, SoundSource};
use crate::youtube;

fn progress_callback(
    state: &AppState,
    download_id: String,
) -> youtube::downloader::ProgressCallback {
    let sink = state.event_sink();
    Box::new(move |status, progress| {
        sink.emit(CoreEvent::YoutubeDownloadProgress {
            download_id: download_id.clone(),
            status: status.to_string(),
            progress,
        });
    })
}

/// Download (or reuse the cached copy of) a YouTube video's audio and build a `Sound` for it.
pub async fn add_sound_from_youtube(
    state: &AppState,
    url: String,
    download_id: String,
) -> Result<Sound, String> {
    let cache = state.youtube_cache.clone();
    let on_progress = progress_callback(state, download_id);
    let entry = youtube::download_audio(&url, cache, Some(on_progress)).await?;

    tracing::info!("[cpu-pool] add_sound_from_youtube duration START");
    let cached_path = entry.cached_path.clone();
    let pool = state.cpu_pool.clone();
    let duration = tokio::task::spawn_blocking(move || {
        pool.install(|| BufferManager::get_audio_duration(&cached_path).unwrap_or(0.0))
    })
    .await
    .map_err(|e| format!("Duration task failed: {}", e))?;
    tracing::info!("[cpu-pool] add_sound_from_youtube duration END");

    Ok(Sound {
        id: uuid::Uuid::new_v4().to_string(),
        name: entry.title.clone(),
        source: SoundSource::YouTube {
            url: url.clone(),
            cached_path: entry.cached_path.clone(),
        },
        momentum: 0.0,
        volume: 1.0,
        duration,
        resolved_video_id: None,
    })
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PredownloadResult {
    pub video_id: String,
    pub cached_path: String,
    pub title: String,
    pub duration: f64,
    pub waveform: Option<WaveformData>,
}

/// Pre-download a suggestion's audio to cache WITHOUT adding it to the profile.
/// Returns cached path, duration, and waveform data in one call.
pub async fn predownload_suggestion(
    state: &AppState,
    url: String,
    video_id: String,
    download_id: String,
) -> Result<PredownloadResult, String> {
    let cache = state.youtube_cache.clone();
    let on_progress = progress_callback(state, download_id);
    let entry = youtube::download_audio(&url, cache, Some(on_progress)).await?;

    let cached_path = entry.cached_path.clone();

    // Compute waveform (includes duration + momentum detection) during predownload
    // so momentum is available immediately when the user navigates to this suggestion
    tracing::info!(
        "[cpu-pool] predownload_suggestion waveform START: {}",
        video_id
    );
    let waveform_path = cached_path.clone();
    let pool = state.cpu_pool.clone();
    let vid_log = video_id.clone();

    let (duration, waveform) = tokio::task::spawn_blocking(move || {
        pool.install(
            || match analysis::compute_waveform_sampled(&waveform_path, 80, None) {
                Ok(wf) => (wf.duration, Some(wf)),
                Err(e) => {
                    tracing::warn!("Waveform computation failed for {}: {}", vid_log, e);
                    let dur = BufferManager::get_audio_duration(&waveform_path).unwrap_or(0.0);
                    (dur, None)
                }
            },
        )
    })
    .await
    .map_err(|e| format!("Predownload task failed: {}", e))?;

    tracing::info!(
        "[cpu-pool] predownload_suggestion waveform END: {}",
        video_id
    );

    if let Some(ref wf) = waveform {
        let mut cache = state.waveform_cache.lock().unwrap();
        cache.insert(cached_path.clone(), wf.clone());
    }

    Ok(PredownloadResult {
        video_id,
        cached_path,
        title: entry.title,
        duration,
        waveform,
    })
}
