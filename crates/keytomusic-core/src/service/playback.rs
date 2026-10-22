use std::collections::HashMap;

use crate::audio::buffer::BufferManager;
use crate::events::CoreEvent;
use crate::state::AppState;

/// Decode off the audio thread and start `file_path` on `track_id`.
/// A missing file plays the error sound and emits `SoundNotFound`.
#[allow(clippy::too_many_arguments)]
pub async fn play_sound(
    state: &AppState,
    track_id: String,
    sound_id: String,
    file_path: String,
    start_position: f64,
    sound_volume: f32,
    track_volume: Option<f32>,
) -> Result<(), String> {
    if !start_position.is_finite() || start_position < 0.0 {
        return Err("Invalid playback position".to_string());
    }
    if !sound_volume.is_finite() || !(0.0..=1.0).contains(&sound_volume) {
        return Err("Invalid sound volume".to_string());
    }
    let engine = state.get_audio_engine()?;
    let config = state.get_config();

    if !std::path::Path::new(&file_path).exists() {
        tracing::warn!(
            "Sound file not found: {} (track: {}, sound: {})",
            file_path,
            track_id,
            sound_id
        );
        let _ = engine.play_error_sound();
        state.emit(CoreEvent::SoundNotFound {
            sound_id,
            path: file_path.clone(),
            track_id,
        });
        return Err(format!("Sound file not found: {}", file_path));
    }

    // Apply the requested volume before decoding; a cancelled slow preparation
    // must never overwrite the volume of a newer playback request.
    engine.set_track_volume(
        track_id.clone(),
        track_volume.unwrap_or(1.0).clamp(0.0, 1.0),
    )?;
    // Pre-create the source off the audio thread (file I/O + probe + seek + first decode)
    let request = engine.begin_playback(&track_id);
    let decode_path = file_path.clone();
    let source = tokio::task::spawn_blocking(move || {
        crate::audio::symphonia_source::SymphoniaSource::new(&decode_path, start_position)
    })
    .await
    .map_err(|e| e.to_string())??;

    engine
        .play_sound_prepared(
            request,
            track_id,
            sound_id,
            file_path,
            start_position,
            source,
            sound_volume,
            config.crossfade_duration,
        )
        .await
}

pub fn stop_all_sounds(state: &AppState) -> Result<(), String> {
    state.get_audio_engine()?.stop_all()?;
    // Lets hosts reset mood/loop playback state.
    state.emit(CoreEvent::StopAllTriggered);
    Ok(())
}

pub async fn get_audio_duration(state: &AppState, path: String) -> Result<f64, String> {
    tracing::info!("[cpu-pool] get_audio_duration START: {}", path);
    let pool = state.cpu_pool.clone();
    let result = tokio::task::spawn_blocking(move || {
        pool.install(|| BufferManager::get_audio_duration(&path))
    })
    .await
    .map_err(|e| format!("Task failed: {}", e))?;
    tracing::info!("[cpu-pool] get_audio_duration END");
    result
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundPreloadEntry {
    pub sound_id: String,
    pub file_path: String,
    pub needs_duration: bool,
}

/// Batch compute durations for sounds that need it.
/// Uses shared CPU pool. Checks profile_load_gen to bail early on stale loads.
pub async fn preload_profile_sounds(
    state: &AppState,
    sounds: Vec<SoundPreloadEntry>,
) -> Result<HashMap<String, f64>, String> {
    use std::sync::atomic::Ordering;

    let pool = state.cpu_pool.clone();
    let gen_counter = state.profile_load_gen.clone();
    // Bump generation: any older preload_profile_sounds still running becomes stale
    let gen = gen_counter.fetch_add(1, Ordering::SeqCst) + 1;

    let count = sounds.iter().filter(|e| e.needs_duration).count();
    tracing::info!(
        "[cpu-pool] preload_profile_sounds START: {} sounds need duration (gen={})",
        count,
        gen
    );
    let result = tokio::task::spawn_blocking(move || {
        use rayon::prelude::*;
        use std::sync::Mutex;

        let needs_work: Vec<&SoundPreloadEntry> =
            sounds.iter().filter(|e| e.needs_duration).collect();

        if needs_work.is_empty() {
            return HashMap::new();
        }

        let durations: Mutex<HashMap<String, f64>> = Mutex::new(HashMap::new());

        // Process in chunks of 4 to avoid monopolizing the pool.
        // Check generation at each chunk boundary for early bail.
        for chunk in needs_work.chunks(4) {
            if gen_counter.load(Ordering::SeqCst) != gen {
                break;
            }
            pool.install(|| {
                chunk.par_iter().for_each(|entry| {
                    if gen_counter.load(Ordering::SeqCst) != gen {
                        return;
                    }

                    let path = std::path::Path::new(&entry.file_path);
                    if !path.exists() {
                        return;
                    }

                    if let Ok(dur) = BufferManager::get_audio_duration(&entry.file_path) {
                        if dur > 0.0 {
                            durations
                                .lock()
                                .unwrap()
                                .insert(entry.sound_id.clone(), dur);
                        }
                    }
                });
            });
        }

        durations.into_inner().unwrap()
    })
    .await
    .map_err(|e| format!("Preload task failed: {}", e));
    tracing::info!("[cpu-pool] preload_profile_sounds END (gen={})", gen);
    result
}
