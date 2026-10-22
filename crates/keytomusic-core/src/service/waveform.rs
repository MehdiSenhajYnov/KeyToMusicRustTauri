use std::collections::HashMap;

use crate::audio::analysis::{self, WaveformData};
use crate::events::CoreEvent;
use crate::state::AppState;

/// Cached waveform, or compute it while streaming `WaveformProgress` events.
pub async fn get_waveform(
    state: &AppState,
    path: String,
    num_points: usize,
) -> Result<WaveformData, String> {
    {
        let mut cache = state.waveform_cache.lock().unwrap();
        if let Some(data) = cache.get(&path) {
            return Ok(data.clone());
        }
    }

    tracing::info!("[cpu-pool] get_waveform START: {}", path);
    let path_clone = path.clone();
    let path_event = path.clone();
    let pool = state.cpu_pool.clone();

    // Partial results are forwarded from an async task so the decode thread never blocks on the sink.
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(Vec<f32>, f64, u32)>();
    let sink = state.event_sink();
    let forwarder = tokio::spawn(async move {
        while let Some((points, duration, sample_rate)) = rx.recv().await {
            sink.emit(CoreEvent::WaveformProgress {
                path: path_event.clone(),
                points,
                duration,
                sample_rate,
            });
        }
    });

    let result = tokio::task::spawn_blocking(move || {
        pool.install(|| {
            let cb = move |points: &[f32], dur: f64, sr: u32| {
                let _ = tx.send((points.to_vec(), dur, sr));
            };
            analysis::compute_waveform_sampled(&path_clone, num_points, Some(&cb))
        })
    })
    .await
    .map_err(|e| format!("Waveform task failed: {}", e))??;

    let _ = forwarder.await;
    tracing::info!("[cpu-pool] get_waveform END: {}", path);

    {
        let mut cache = state.waveform_cache.lock().unwrap();
        cache.insert(path, result.clone());
    }

    Ok(result)
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaveformBatchEntry {
    pub path: String,
    pub num_points: usize,
}

pub async fn get_waveforms_batch(
    state: &AppState,
    entries: Vec<WaveformBatchEntry>,
) -> Result<HashMap<String, WaveformData>, String> {
    let waveform_cache = state.waveform_cache.clone();

    let mut results: HashMap<String, WaveformData> = HashMap::new();
    let mut to_compute: Vec<WaveformBatchEntry> = Vec::new();

    {
        let mut cache = waveform_cache.lock().unwrap();
        for entry in &entries {
            if let Some(data) = cache.get(&entry.path) {
                results.insert(entry.path.clone(), data.clone());
            } else {
                to_compute.push(entry.clone());
            }
        }
    }

    if to_compute.is_empty() {
        return Ok(results);
    }

    let gen = state.profile_load_gen.clone();
    let current_gen = gen.load(std::sync::atomic::Ordering::SeqCst);
    tracing::info!(
        "[cpu-pool] get_waveforms_batch START: {} to compute (gen={})",
        to_compute.len(),
        current_gen
    );
    let pool = state.cpu_pool.clone();
    let computed = tokio::task::spawn_blocking(move || {
        use rayon::prelude::*;
        use std::sync::atomic::Ordering;
        use std::sync::Mutex;

        let new_results: Mutex<HashMap<String, WaveformData>> = Mutex::new(HashMap::new());

        // Process in chunks of 4 to avoid monopolizing the pool.
        // Check generation at each chunk boundary for early bail.
        for chunk in to_compute.chunks(4) {
            if gen.load(Ordering::SeqCst) != current_gen {
                break;
            }
            pool.install(|| {
                chunk.par_iter().for_each(|entry| {
                    if gen.load(Ordering::SeqCst) != current_gen {
                        return;
                    }
                    if let Ok(data) =
                        analysis::compute_waveform_sampled(&entry.path, entry.num_points, None)
                    {
                        new_results.lock().unwrap().insert(entry.path.clone(), data);
                    }
                });
            });
        }

        new_results.into_inner().unwrap()
    })
    .await
    .map_err(|e| format!("Waveform batch task failed: {}", e))?;

    {
        let mut cache = waveform_cache.lock().unwrap();
        for (path, data) in &computed {
            cache.insert(path.clone(), data.clone());
        }
    }

    results.extend(computed);
    tracing::info!("[cpu-pool] get_waveforms_batch END");
    Ok(results)
}
