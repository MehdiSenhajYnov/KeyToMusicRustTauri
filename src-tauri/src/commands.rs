use crate::audio::{self, analysis, buffer::BufferManager};
use crate::discovery;
use crate::import_export;
use crate::mood;
use crate::state::AppState;
use crate::storage;
use crate::types::{AppConfig, MomentumModifier, MoodCategory, Profile, Sound, SoundSource};
use crate::youtube;
use tauri::{Emitter, State};

#[cfg(target_os = "linux")]
use std::os::unix::fs::PermissionsExt;

// ─── Configuration Commands ─────────────────────────────────────────────────

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    Ok(state.get_config())
}

#[tauri::command]
pub fn update_config(state: State<'_, AppState>, updates: serde_json::Value) -> Result<(), String> {
    let config = state.update_config(|config| {
        if let Some(v) = updates.get("masterVolume").and_then(|v| v.as_f64()) {
            config.master_volume = v as f32;
        }
        if let Some(v) = updates.get("autoMomentum").and_then(|v| v.as_bool()) {
            config.auto_momentum = v;
        }
        if let Some(v) = updates.get("keyDetectionEnabled").and_then(|v| v.as_bool()) {
            config.key_detection_enabled = v;
        }
        if let Some(v) = updates.get("crossfadeDuration").and_then(|v| v.as_u64()) {
            config.crossfade_duration = v as u32;
        }
        if let Some(v) = updates.get("keyCooldown").and_then(|v| v.as_u64()) {
            config.key_cooldown = v as u32;
        }
        if let Some(v) = updates.get("stopAllShortcut").and_then(|v| v.as_array()) {
            let keys: Vec<String> = v
                .iter()
                .filter_map(|k| k.as_str().map(|s| s.to_string()))
                .collect();
            if !keys.is_empty() {
                config.stop_all_shortcut = keys;
            }
        }
        if let Some(v) = updates
            .get("autoMomentumShortcut")
            .and_then(|v| v.as_array())
        {
            config.auto_momentum_shortcut = v
                .iter()
                .filter_map(|k| k.as_str().map(|s| s.to_string()))
                .collect();
        }
        if let Some(v) = updates
            .get("keyDetectionShortcut")
            .and_then(|v| v.as_array())
        {
            config.key_detection_shortcut = v
                .iter()
                .filter_map(|k| k.as_str().map(|s| s.to_string()))
                .collect();
        }
        if updates.get("currentProfileId").is_some() {
            config.current_profile_id = updates
                .get("currentProfileId")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
        }
        if updates.get("audioDevice").is_some() {
            config.audio_device = updates.get("audioDevice").and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    v.as_str().map(|s| s.to_string())
                }
            });
        }
        if let Some(v) = updates.get("chordWindowMs").and_then(|v| v.as_u64()) {
            config.chord_window_ms = v as u32;
        }
        if let Some(v) = updates.get("momentumModifier").and_then(|v| v.as_str()) {
            config.momentum_modifier = match v {
                "Shift" => MomentumModifier::Shift,
                "Ctrl" => MomentumModifier::Ctrl,
                "Alt" => MomentumModifier::Alt,
                "None" => MomentumModifier::None,
                _ => MomentumModifier::Shift, // default fallback
            };
        }
        if let Some(v) = updates
            .get("playlistImportEnabled")
            .and_then(|v| v.as_bool())
        {
            config.playlist_import_enabled = v;
        }
        if let Some(v) = updates.get("moodAiEnabled").and_then(|v| v.as_bool()) {
            config.mood_ai_enabled = v;
        }
        if let Some(v) = updates.get("moodApiPort").and_then(|v| v.as_u64()) {
            config.mood_api_port = v as u16;
        }
        if let Some(v) = updates.get("moodEntryThreshold").and_then(|v| v.as_f64()) {
            config.mood_entry_threshold = v as f32;
        }
        if let Some(v) = updates.get("moodExitThreshold").and_then(|v| v.as_f64()) {
            config.mood_exit_threshold = v as f32;
        }
        if let Some(v) = updates.get("moodDwellPages").and_then(|v| v.as_u64()) {
            config.mood_dwell_pages = v as u32;
        }
        if let Some(v) = updates.get("moodWindowSize").and_then(|v| v.as_u64()) {
            config.mood_window_size = v as usize;
        }
    });

    // Sync audio device to audio engine
    if updates.get("audioDevice").is_some() {
        if let Ok(engine) = state.get_audio_engine() {
            let _ = engine.set_audio_device(config.audio_device.clone());
        }
    }

    // Sync master volume to audio engine
    if updates.get("masterVolume").is_some() {
        if let Ok(engine) = state.get_audio_engine() {
            let _ = engine.set_master_volume(config.master_volume);
        }
    }

    // Sync key detection settings
    if updates.get("keyDetectionEnabled").is_some() {
        state.key_detector.set_enabled(config.key_detection_enabled);
    }
    if updates.get("keyCooldown").is_some() {
        state.key_detector.set_cooldown(config.key_cooldown);
    }
    if updates.get("stopAllShortcut").is_some() {
        state
            .key_detector
            .set_stop_all_shortcut(config.stop_all_shortcut.clone());
    }
    if updates.get("autoMomentumShortcut").is_some() {
        state
            .key_detector
            .set_auto_momentum_shortcut(config.auto_momentum_shortcut.clone());
    }
    if updates.get("keyDetectionShortcut").is_some() {
        state
            .key_detector
            .set_key_detection_shortcut(config.key_detection_shortcut.clone());
    }
    if updates.get("chordWindowMs").is_some() {
        state.key_detector.set_chord_window(config.chord_window_ms);
    }

    // Reset mood director on profile switch (different narrative context)
    if updates.get("currentProfileId").is_some() {
        let mut director = state
            .mood_director
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        director.reset();
    }

    // Sync mood director config if any mood field changed
    if updates.get("moodEntryThreshold").is_some()
        || updates.get("moodExitThreshold").is_some()
        || updates.get("moodDwellPages").is_some()
        || updates.get("moodWindowSize").is_some()
    {
        let mut director = state
            .mood_director
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        director.update_config(mood::director::DirectorConfig {
            entry_threshold: config.mood_entry_threshold,
            exit_threshold: config.mood_exit_threshold,
            min_dwell_pages: config.mood_dwell_pages,
            window_size: config.mood_window_size,
        });
    }

    storage::save_config(&config)
}

/// Set the profile bindings for chord detection.
/// Called when profile changes or bindings are modified.
#[tauri::command]
pub fn set_profile_bindings(
    state: State<'_, AppState>,
    bindings: Vec<String>,
) -> Result<(), String> {
    state.key_detector.set_profile_bindings(&bindings);
    Ok(())
}

// ─── Profile Commands ───────────────────────────────────────────────────────

#[tauri::command]
pub fn list_profiles() -> Result<Vec<storage::ProfileSummary>, String> {
    storage::list_profiles()
}

#[tauri::command]
pub fn create_profile(name: String) -> Result<Profile, String> {
    storage::create_profile(name)
}

#[tauri::command]
pub fn load_profile(id: String) -> Result<Profile, String> {
    storage::load_profile(id)
}

#[tauri::command]
pub fn save_profile(profile: Profile) -> Result<(), String> {
    storage::save_profile(&profile)
}

#[tauri::command]
pub fn delete_profile(state: State<'_, AppState>, id: String) -> Result<(), String> {
    // Clean up discovery cache for this profile
    discovery::cache::DiscoveryCache::delete(&id);
    storage::delete_profile(id)?;
    // Cleanup cached YouTube files no longer referenced by any profile
    if let Ok(mut cache) = state.youtube_cache.lock() {
        cache.ensure_loaded();
        cache.cleanup_unused();
    }
    Ok(())
}

#[tauri::command]
pub fn duplicate_profile(id: String, new_name: Option<String>) -> Result<Profile, String> {
    storage::duplicate_profile(id, new_name)
}

// ─── Audio Commands ─────────────────────────────────────────────────────────

#[tauri::command]
pub fn play_sound(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_id: String,
    sound_id: String,
    file_path: String,
    start_position: f64,
    sound_volume: f32,
) -> Result<(), String> {
    let engine = state.get_audio_engine()?;
    let config = state.get_config();

    // Check file exists
    if !std::path::Path::new(&file_path).exists() {
        tracing::warn!(
            "Sound file not found: {} (track: {}, sound: {})",
            file_path,
            track_id,
            sound_id
        );
        // Play error sound
        let _ = engine.play_error_sound();
        // Emit sound_not_found event
        let _ = app.emit(
            "sound_not_found",
            serde_json::json!({
                "soundId": sound_id,
                "path": file_path,
                "trackId": track_id,
            }),
        );
        return Err(format!("Sound file not found: {}", file_path));
    }

    // Pre-create the source off the audio thread (file I/O + probe + seek + first decode)
    let source = crate::audio::symphonia_source::SymphoniaSource::new(&file_path, start_position)?;

    engine.play_sound_prepared(
        track_id,
        sound_id,
        file_path,
        start_position,
        source,
        sound_volume,
        config.crossfade_duration,
    )
}

#[tauri::command]
pub fn stop_sound(state: State<'_, AppState>, track_id: String) -> Result<(), String> {
    state.get_audio_engine()?.stop_track(track_id)
}

#[tauri::command]
pub fn stop_all_sounds(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.get_audio_engine()?.stop_all()?;
    // Emit stop_all_triggered so mood playback resets activeMoodRef
    let _ = app_handle.emit("stop_all_triggered", serde_json::json!({}));
    Ok(())
}

#[tauri::command]
pub fn set_master_volume(state: State<'_, AppState>, volume: f32) -> Result<(), String> {
    // Update config
    state.update_config(|config| {
        config.master_volume = volume;
    });
    state.schedule_config_save();

    // Update audio engine (graceful if not yet ready — will pick up from config on init)
    if let Ok(engine) = state.get_audio_engine() {
        let _ = engine.set_master_volume(volume);
    }
    Ok(())
}

#[tauri::command]
pub fn set_track_volume(
    state: State<'_, AppState>,
    track_id: String,
    volume: f32,
) -> Result<(), String> {
    state.get_audio_engine()?.set_track_volume(track_id, volume)
}

#[tauri::command]
pub fn set_sound_volume(
    state: State<'_, AppState>,
    track_id: String,
    sound_id: String,
    volume: f32,
) -> Result<(), String> {
    state
        .get_audio_engine()?
        .set_sound_volume(track_id, sound_id, volume)
}

#[tauri::command]
pub async fn get_audio_duration(state: State<'_, AppState>, path: String) -> Result<f64, String> {
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

// ─── Sound Pre-loading ─────────────────────────────────────────────────────

/// Batch compute durations for sounds that need it.
/// Uses shared CPU pool. Checks profile_load_gen to bail early on stale loads.
#[tauri::command]
pub async fn preload_profile_sounds(
    state: State<'_, AppState>,
    sounds: Vec<SoundPreloadEntry>,
) -> Result<std::collections::HashMap<String, f64>, String> {
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

        // Only process sounds that actually need duration
        let needs_work: Vec<&SoundPreloadEntry> =
            sounds.iter().filter(|e| e.needs_duration).collect();

        if needs_work.is_empty() {
            return std::collections::HashMap::new();
        }

        let durations: Mutex<std::collections::HashMap<String, f64>> =
            Mutex::new(std::collections::HashMap::new());

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

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundPreloadEntry {
    pub sound_id: String,
    pub file_path: String,
    pub needs_duration: bool,
}

// ─── Key Detection Commands ────────────────────────────────────────────────

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinuxInputAccessStatusPayload {
    pub supported: bool,
    pub session_type: String,
    pub background_detection_available: bool,
    pub can_auto_fix: bool,
    pub relogin_recommended: bool,
    pub accessible_keyboard_devices: Vec<String>,
    pub keyboard_candidates: Vec<String>,
    pub message: Option<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinuxInputAccessFixResult {
    pub success: bool,
    pub message: String,
    pub status: LinuxInputAccessStatusPayload,
}

#[cfg(target_os = "linux")]
fn is_wayland_session_for_linux_input() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
        || std::env::var("XDG_SESSION_TYPE")
            .map(|value| value.eq_ignore_ascii_case("wayland"))
            .unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn linux_input_status_payload() -> LinuxInputAccessStatusPayload {
    if is_wayland_session_for_linux_input() {
        let status = crate::keys::linux_listener::get_linux_input_access_status();
        LinuxInputAccessStatusPayload {
            supported: status.supported,
            session_type: status.session_type,
            background_detection_available: status.background_detection_available,
            can_auto_fix: status.can_auto_fix,
            relogin_recommended: status.relogin_recommended,
            accessible_keyboard_devices: status.accessible_keyboard_devices,
            keyboard_candidates: status.keyboard_candidates,
            message: status.message,
        }
    } else {
        LinuxInputAccessStatusPayload {
            supported: true,
            session_type: "x11".to_string(),
            background_detection_available: true,
            can_auto_fix: false,
            relogin_recommended: false,
            accessible_keyboard_devices: Vec::new(),
            keyboard_candidates: Vec::new(),
            message: Some("Background detection uses the X11 hook in this session.".to_string()),
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn linux_input_status_payload() -> LinuxInputAccessStatusPayload {
    LinuxInputAccessStatusPayload {
        supported: false,
        session_type: "other".to_string(),
        background_detection_available: false,
        can_auto_fix: false,
        relogin_recommended: false,
        accessible_keyboard_devices: Vec::new(),
        keyboard_candidates: Vec::new(),
        message: Some(
            "Automatic Linux background-input setup is only available on Linux.".to_string(),
        ),
    }
}

#[tauri::command]
pub fn get_linux_input_access_status() -> Result<LinuxInputAccessStatusPayload, String> {
    Ok(linux_input_status_payload())
}

#[tauri::command]
pub fn enable_linux_background_detection() -> Result<LinuxInputAccessFixResult, String> {
    #[cfg(not(target_os = "linux"))]
    {
        return Err("Automatic background-input setup is only available on Linux".to_string());
    }

    #[cfg(target_os = "linux")]
    {
        if !is_wayland_session_for_linux_input() {
            let status = linux_input_status_payload();
            return Ok(LinuxInputAccessFixResult {
                success: true,
                message: "No extra setup is needed on X11.".to_string(),
                status,
            });
        }

        let current_status = linux_input_status_payload();
        if current_status.background_detection_available {
            return Ok(LinuxInputAccessFixResult {
                success: true,
                message: "Background detection is already available.".to_string(),
                status: current_status,
            });
        }

        let temp_path = std::env::temp_dir().join(format!(
            "keytomusic-enable-wayland-input-{}.sh",
            std::process::id()
        ));

        let script = r#"#!/bin/sh
set -eu
RULE_PATH="/etc/udev/rules.d/70-keytomusic-background-input.rules"
cat > "$RULE_PATH" <<'EOF'
ACTION!="remove", SUBSYSTEM=="input", KERNEL=="event*", ENV{ID_INPUT_KEYBOARD}=="1", TAG+="uaccess"
EOF
chmod 0644 "$RULE_PATH"
udevadm control --reload-rules
udevadm trigger --subsystem-match=input --action=change
"#;

        std::fs::write(&temp_path, script)
            .map_err(|e| format!("Failed to prepare setup helper: {}", e))?;
        std::fs::set_permissions(&temp_path, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| format!("Failed to mark setup helper executable: {}", e))?;

        let output = std::process::Command::new("pkexec")
            .arg("/bin/sh")
            .arg(&temp_path)
            .output();

        let _ = std::fs::remove_file(&temp_path);

        let output = match output {
            Ok(output) => output,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(
                    "Automatic setup requires pkexec, but it is not installed on this system."
                        .to_string(),
                )
            }
            Err(e) => return Err(format!("Failed to launch automatic setup: {}", e)),
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let detail = if !stderr.is_empty() {
                stderr
            } else if !stdout.is_empty() {
                stdout
            } else {
                "The permission request was cancelled or denied.".to_string()
            };
            return Err(format!("Automatic setup failed: {}", detail));
        }

        let status = linux_input_status_payload();
        Ok(LinuxInputAccessFixResult {
            success: true,
            message: if status.background_detection_available {
                "Background detection access has been enabled.".to_string()
            } else {
                "System access rules were installed. The app will keep retrying automatically; if background detection still does not recover, sign out and back in once.".to_string()
            },
            status,
        })
    }
}

#[tauri::command]
pub fn set_key_detection(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    state.key_detector.set_enabled(enabled);

    // Also update config
    state.update_config(|config| {
        config.key_detection_enabled = enabled;
    });
    state.schedule_config_save();
    Ok(())
}

#[tauri::command]
pub fn set_stop_all_shortcut(state: State<'_, AppState>, keys: Vec<String>) -> Result<(), String> {
    if keys.len() < 2 {
        return Err("Stop all shortcut must have at least 2 keys".to_string());
    }

    // Update the detector
    state.key_detector.set_stop_all_shortcut(keys.clone());

    // Update config
    state.update_config(|config| {
        config.stop_all_shortcut = keys;
    });
    state.schedule_config_save();
    Ok(())
}

#[tauri::command]
pub fn set_key_cooldown(state: State<'_, AppState>, cooldown_ms: u32) -> Result<(), String> {
    if cooldown_ms > 5000 {
        return Err("Cooldown must be at most 5000ms".to_string());
    }

    // Update the detector
    state.key_detector.set_cooldown(cooldown_ms);

    // Update config
    state.update_config(|config| {
        config.key_cooldown = cooldown_ms;
    });
    state.schedule_config_save();
    Ok(())
}

// ─── Audio Device Commands ────────────────────────────────────────────────

#[tauri::command]
pub fn list_audio_devices() -> Vec<String> {
    audio::list_audio_devices()
}

#[tauri::command]
pub fn set_audio_device(state: State<'_, AppState>, device: Option<String>) -> Result<(), String> {
    // Update audio engine
    state.get_audio_engine()?.set_audio_device(device.clone())?;

    // Update config
    state.update_config(|config| {
        config.audio_device = device;
    });
    state.schedule_config_save();
    Ok(())
}

// ─── Waveform Commands ────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_waveform(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    path: String,
    num_points: usize,
) -> Result<analysis::WaveformData, String> {
    // Check cache first
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

    // Channel for streaming partial results to the frontend
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(Vec<f32>, f64, u32)>();

    // Forwarder: reads from channel and emits Tauri events
    let app_clone = app.clone();
    let forwarder = tokio::spawn(async move {
        while let Some((points, duration, sample_rate)) = rx.recv().await {
            let _ = app_clone.emit(
                "waveform_progress",
                serde_json::json!({
                    "path": &path_event,
                    "points": points,
                    "duration": duration,
                    "sampleRate": sample_rate,
                }),
            );
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

    // Cache result
    {
        let mut cache = state.waveform_cache.lock().unwrap();
        cache.insert(path, result.clone());
    }

    Ok(result)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaveformBatchEntry {
    pub path: String,
    pub num_points: usize,
}

#[tauri::command]
pub async fn get_waveforms_batch(
    state: State<'_, AppState>,
    entries: Vec<WaveformBatchEntry>,
) -> Result<std::collections::HashMap<String, analysis::WaveformData>, String> {
    let waveform_cache = state.waveform_cache.clone();

    // Separate cached from uncached
    let mut results: std::collections::HashMap<String, analysis::WaveformData> =
        std::collections::HashMap::new();
    let mut to_compute: Vec<WaveformBatchEntry> = Vec::new();

    {
        let mut cache = waveform_cache.lock().unwrap();
        for entry in &entries {
            if let Some(data) = cache.get(&entry.path) {
                results.insert(entry.path.clone(), data.clone());
            } else {
                to_compute.push(WaveformBatchEntry {
                    path: entry.path.clone(),
                    num_points: entry.num_points,
                });
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

        let new_results: Mutex<std::collections::HashMap<String, analysis::WaveformData>> =
            Mutex::new(std::collections::HashMap::new());

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

    // Cache computed results
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

// ─── YouTube Commands ─────────────────────────────────────────────────────

#[tauri::command]
pub async fn add_sound_from_youtube(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    url: String,
    download_id: String,
) -> Result<Sound, String> {
    let cache = state.youtube_cache.clone();

    let app_handle = app.clone();
    let did = download_id.clone();
    let on_progress: youtube::downloader::ProgressCallback = Box::new(move |status, progress| {
        let _ = app_handle.emit(
            "youtube_download_progress",
            serde_json::json!({
                "downloadId": did,
                "status": status,
                "progress": progress,
            }),
        );
    });

    let entry = youtube::download_audio(&url, cache, Some(on_progress)).await?;

    // Compute duration
    tracing::info!("[cpu-pool] add_sound_from_youtube duration START");
    let cached_path = entry.cached_path.clone();
    let pool = state.cpu_pool.clone();
    let duration = tokio::task::spawn_blocking(move || {
        pool.install(|| BufferManager::get_audio_duration(&cached_path).unwrap_or(0.0))
    })
    .await
    .map_err(|e| format!("Duration task failed: {}", e))?;
    tracing::info!("[cpu-pool] add_sound_from_youtube duration END");

    let sound = Sound {
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
    };

    Ok(sound)
}

#[tauri::command]
pub async fn search_youtube(
    state: State<'_, AppState>,
    query: String,
    max_results: u32,
) -> Result<Vec<youtube::search::YoutubeSearchResult>, String> {
    let cache = state.youtube_cache.clone();
    youtube::search::search_youtube(&query, max_results, cache).await
}

#[tauri::command]
pub async fn fetch_playlist(
    state: State<'_, AppState>,
    url: String,
) -> Result<youtube::search::YoutubePlaylist, String> {
    let cache = state.youtube_cache.clone();
    youtube::search::fetch_playlist(&url, cache).await
}

#[tauri::command]
pub async fn get_youtube_stream_url(
    video_id: String,
) -> Result<youtube::search::StreamUrlResult, String> {
    youtube::search::get_stream_url(&video_id).await
}

#[tauri::command]
pub async fn check_yt_dlp_installed() -> Result<bool, String> {
    Ok(youtube::is_yt_dlp_installed().await)
}

#[tauri::command]
pub async fn install_yt_dlp() -> Result<(), String> {
    youtube::download_yt_dlp().await?;
    Ok(())
}

#[tauri::command]
pub async fn check_ffmpeg_installed() -> Result<bool, String> {
    Ok(youtube::is_ffmpeg_installed())
}

#[tauri::command]
pub async fn install_ffmpeg() -> Result<(), String> {
    youtube::download_ffmpeg().await?;
    Ok(())
}

// ─── Import/Export Commands ─────────────────────────────────────────────────

#[tauri::command]
pub async fn export_profile(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    profile_id: String,
    output_path: String,
) -> Result<(), String> {
    // Collect waveform data for all sounds in the profile
    let waveforms = {
        let profile = storage::load_profile(profile_id.clone())?;
        let mut map = std::collections::HashMap::new();
        if let Ok(mut cache) = state.waveform_cache.lock() {
            for sound in &profile.sounds {
                let path = match &sound.source {
                    SoundSource::Local { path } => path.clone(),
                    SoundSource::YouTube { cached_path, .. } => cached_path.clone(),
                };
                if let Some(data) = cache.get(&path) {
                    map.insert(path, data.clone());
                }
            }
        }
        map
    };

    let app_handle = app.clone();
    tokio::task::spawn_blocking(move || {
        let progress_cb: import_export::export::ProgressCallback =
            Box::new(move |current, total, filename| {
                let _ = app_handle.emit(
                    "export_progress",
                    serde_json::json!({
                        "current": current,
                        "total": total,
                        "filename": filename,
                    }),
                );
            });
        let waveforms_opt = if waveforms.is_empty() {
            None
        } else {
            Some(waveforms)
        };
        import_export::export_profile(&profile_id, &output_path, waveforms_opt, Some(progress_cb))
    })
    .await
    .map_err(|e| format!("Export task failed: {}", e))?
}

#[tauri::command]
pub async fn import_profile(
    state: State<'_, AppState>,
    ktm_path: String,
) -> Result<String, String> {
    let import_result =
        tokio::task::spawn_blocking(move || import_export::import_profile(&ktm_path))
            .await
            .map_err(|e| format!("Import task failed: {}", e))??;

    // Inject imported waveforms into cache
    if !import_result.waveforms.is_empty() {
        if let Ok(mut cache) = state.waveform_cache.lock() {
            for (path, data) in import_result.waveforms {
                cache.insert(path, data);
            }
        }
    }

    // Cleanup unused cache entries after import
    if let Ok(mut cache) = state.youtube_cache.lock() {
        cache.ensure_loaded();
        cache.cleanup_unused();
    }

    Ok(import_result.profile_id)
}

#[tauri::command]
pub async fn pick_save_location(default_name: String) -> Result<Option<String>, String> {
    let result = tokio::task::spawn_blocking(move || {
        rfd::FileDialog::new()
            .set_file_name(&default_name)
            .add_filter("KeyToMusic Profile", &["ktm"])
            .save_file()
            .map(|p| p.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| format!("File dialog failed: {}", e))?;

    Ok(result)
}

#[tauri::command]
pub fn cleanup_export_temp() {
    import_export::cleanup_interrupted_export();
}

#[tauri::command]
pub fn cancel_export() {
    import_export::cancel_export();
}

#[tauri::command]
pub async fn pick_ktm_file() -> Result<Option<String>, String> {
    let result = tokio::task::spawn_blocking(move || {
        rfd::FileDialog::new()
            .add_filter("KeyToMusic Profile", &["ktm"])
            .pick_file()
            .map(|p| p.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| format!("File dialog failed: {}", e))?;

    Ok(result)
}

// ─── Legacy Import Commands ──────────────────────────────────────────────

#[derive(serde::Deserialize)]
#[allow(non_snake_case)]
struct LegacySoundInfo {
    uniqueId: String,
    soundPath: String,
    soundName: String,
    soundMomentum: f64,
}

#[derive(serde::Deserialize)]
#[allow(non_snake_case)]
struct LegacyKeyEntry {
    Key: u32,
    #[allow(dead_code)]
    UserKeyChar: String,
    SoundInfos: Vec<LegacySoundInfo>,
}

#[derive(serde::Deserialize)]
#[allow(non_snake_case)]
struct LegacySave {
    Sounds: Vec<LegacyKeyEntry>,
}

/// Convert a legacy Windows virtual key code to a web KeyCode string.
fn vk_to_keycode(vk: u32) -> Option<String> {
    match vk {
        65..=90 => {
            let ch = (b'A' + (vk - 65) as u8) as char;
            Some(format!("Key{}", ch))
        }
        48..=57 => {
            let ch = (b'0' + (vk - 48) as u8) as char;
            Some(format!("Digit{}", ch))
        }
        112..=123 => {
            let num = vk - 111;
            Some(format!("F{}", num))
        }
        // OEM keys (common on various keyboard layouts)
        186 => Some("Semicolon".to_string()),
        187 => Some("Equal".to_string()),
        188 => Some("Comma".to_string()),
        189 => Some("Minus".to_string()),
        190 => Some("Period".to_string()),
        191 => Some("Slash".to_string()),
        192 => Some("Backquote".to_string()),
        219 => Some("BracketLeft".to_string()),
        220 => Some("Backslash".to_string()),
        221 => Some("BracketRight".to_string()),
        222 => Some("Quote".to_string()),
        32 => Some("Space".to_string()),
        13 => Some("Enter".to_string()),
        _ => None,
    }
}

/// Pick a legacy save JSON file.
#[tauri::command]
pub async fn pick_legacy_file() -> Result<Option<String>, String> {
    let result = tokio::task::spawn_blocking(move || {
        rfd::FileDialog::new()
            .add_filter("Legacy Save", &["json"])
            .pick_file()
            .map(|p| p.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| format!("File dialog failed: {}", e))?;

    Ok(result)
}

/// Import a legacy KeyToMusic save file and convert it to a new profile.
#[tauri::command]
pub async fn import_legacy_save(path: String) -> Result<Profile, String> {
    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read legacy save: {}", e))?;

    let legacy: LegacySave = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse legacy save: {}", e))?;

    let now = chrono::Utc::now().to_rfc3339();
    let profile_id = uuid::Uuid::new_v4().to_string();
    let track_id = uuid::Uuid::new_v4().to_string();

    // Create a default track
    let track = crate::types::Track {
        id: track_id.clone(),
        name: "OST".to_string(),
        volume: 1.0,
    };

    let mut sounds: Vec<Sound> = Vec::new();
    let mut key_bindings: Vec<crate::types::KeyBinding> = Vec::new();

    for entry in &legacy.Sounds {
        let key_code = match vk_to_keycode(entry.Key) {
            Some(kc) => kc,
            None => {
                tracing::warn!("Skipping unknown legacy key code: {}", entry.Key);
                continue;
            }
        };

        let mut sound_ids: Vec<String> = Vec::new();

        for info in &entry.SoundInfos {
            let sound = Sound {
                id: info.uniqueId.clone(),
                name: info.soundName.clone(),
                source: SoundSource::Local {
                    path: info.soundPath.replace('/', "\\"),
                },
                momentum: info.soundMomentum,
                volume: 1.0,
                duration: 0.0, // Will be computed on load
                resolved_video_id: None,
            };
            sound_ids.push(sound.id.clone());
            sounds.push(sound);
        }

        if !sound_ids.is_empty() {
            key_bindings.push(crate::types::KeyBinding {
                key_code,
                track_id: track_id.clone(),
                sound_ids,
                loop_mode: crate::types::LoopMode::Off,
                current_index: 0,
                name: None,
                mood: None,
                mood_intensity: None,
            });
        }
    }

    // Derive profile name from filename
    let file_name = std::path::Path::new(&path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Legacy Import".to_string());

    let profile = Profile {
        id: profile_id,
        name: format!("{} (Legacy)", file_name),
        created_at: now.clone(),
        updated_at: now,
        sounds,
        tracks: vec![track],
        key_bindings,
        disliked_videos: Vec::new(),
    };

    storage::save_profile(&profile)?;

    tracing::info!(
        "Imported legacy save as profile '{}' with {} sounds and {} key bindings",
        profile.name,
        profile.sounds.len(),
        profile.key_bindings.len()
    );

    Ok(profile)
}

// ─── Discovery Commands ──────────────────────────────────────────────────

#[tauri::command]
pub async fn start_discovery(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    profile_id: String,
    exclude_ids: Vec<String>,
    background: Option<bool>,
) -> Result<Vec<discovery::engine::DiscoverySuggestion>, String> {
    use std::sync::atomic::Ordering;

    let is_background = background.unwrap_or(false);

    // Reset cancel flag
    state.discovery_cancel.store(false, Ordering::Relaxed);

    // Load the profile to extract seeds (YouTube + local)
    let profile = storage::load_profile(profile_id.clone())?;

    let mut seeds = Vec::new();
    let mut existing_ids = Vec::new();
    let mut unresolved_locals: Vec<(String, String, String)> = Vec::new(); // (sound_id, path, name)

    for sound in &profile.sounds {
        match &sound.source {
            SoundSource::YouTube { url, .. } => {
                if let Some(video_id) = youtube::downloader::extract_video_id(url) {
                    seeds.push(discovery::engine::SeedInfo {
                        video_id: video_id.clone(),
                        sound_name: sound.name.clone(),
                    });
                    existing_ids.push(video_id);
                }
            }
            SoundSource::Local { path } => {
                if let Some(ref video_id) = sound.resolved_video_id {
                    seeds.push(discovery::engine::SeedInfo {
                        video_id: video_id.clone(),
                        sound_name: sound.name.clone(),
                    });
                } else {
                    unresolved_locals.push((sound.id.clone(), path.clone(), sound.name.clone()));
                }
            }
        }
    }

    // Resolve unresolved local sounds via YouTube search
    if !unresolved_locals.is_empty() {
        let count = unresolved_locals.len();
        if !is_background {
            let _ = app.emit(
                "discovery_resolving",
                serde_json::json!({
                    "count": count,
                }),
            );
        }
        tracing::info!("Resolving {} local sounds for discovery seeds", count);

        let youtube_cache = state.youtube_cache.clone();
        let resolved =
            discovery::engine::resolve_local_seeds(unresolved_locals, youtube_cache).await;

        // Add resolved seeds and collect updates for targeted persist
        let mut resolved_updates: Vec<(String, String)> = Vec::new();
        for r in &resolved {
            if let Some(sound) = profile.sounds.iter().find(|s| s.id == r.sound_id) {
                seeds.push(discovery::engine::SeedInfo {
                    video_id: r.video_id.clone(),
                    sound_name: sound.name.clone(),
                });
                resolved_updates.push((r.sound_id.clone(), r.video_id.clone()));
            }
        }

        // Persist resolved video IDs with targeted update (avoids overwriting concurrent profile changes)
        if !resolved_updates.is_empty() {
            if let Err(e) = storage::update_resolved_video_ids(&profile_id, &resolved_updates) {
                tracing::warn!("Failed to save resolved video IDs: {}", e);
            }
        }
    }

    if seeds.is_empty() {
        return Err(
            "No seeds found in profile (no YouTube sounds and no resolvable local sounds)"
                .to_string(),
        );
    }

    // Preserve dismissed_ids from previous cache so dismissed suggestions don't reappear
    let previous_dismissed = discovery::cache::DiscoveryCache::load(&profile_id)
        .map(|d| d.dismissed_ids)
        .unwrap_or_default();

    // Exclude profile sounds, previously dismissed, disliked, and caller-specified IDs
    let mut all_excluded = existing_ids.clone();
    all_excluded.extend(previous_dismissed.iter().cloned());
    for vid in &profile.disliked_videos {
        all_excluded.push(vid.clone());
    }
    all_excluded.extend(exclude_ids.into_iter());

    let yt_dlp_bin = match youtube::downloader::get_yt_dlp_bin() {
        Ok(path) => path,
        Err(_) => youtube::download_yt_dlp().await?,
    };
    let cancel_flag = state.discovery_cancel.clone();

    if !is_background {
        let _ = app.emit("discovery_started", serde_json::json!({}));
    }

    let engine = discovery::engine::DiscoveryEngine::new(cancel_flag.clone());

    let app_progress = app.clone();
    let app_partial = app.clone();
    let bg = is_background;
    let suggestions = engine
        .generate_suggestions(
            seeds.clone(),
            all_excluded,
            yt_dlp_bin,
            move |current, total, seed_name| {
                if !bg {
                    let _ = app_progress.emit(
                        "discovery_progress",
                        serde_json::json!({
                            "current": current,
                            "total": total,
                            "seedName": seed_name,
                        }),
                    );
                }
            },
            move |partial_suggestions| {
                if !bg {
                    let _ = app_partial.emit("discovery_partial", partial_suggestions);
                }
            },
        )
        .await;

    if cancel_flag.load(Ordering::Relaxed) {
        if !is_background {
            let _ = app.emit(
                "discovery_error",
                serde_json::json!({
                    "message": "Discovery cancelled",
                }),
            );
        }
        return Err("Discovery cancelled".to_string());
    }

    // Cache results
    let seed_ids: Vec<String> = seeds.iter().map(|s| s.video_id.clone()).collect();
    let seed_hash = discovery::cache::DiscoveryCache::compute_seed_hash(&seed_ids);

    if is_background {
        // Background mode: append to existing cache without replacing
        discovery::cache::DiscoveryCache::append_suggestions(
            &profile_id,
            suggestions.clone(),
            &seed_hash,
        )
        .ok();
    } else {
        let cache_data = discovery::cache::DiscoveryCacheData {
            profile_id: profile_id.clone(),
            seed_hash,
            generated_at: chrono::Utc::now().to_rfc3339(),
            suggestions: suggestions.clone(),
            dismissed_ids: previous_dismissed,
            cursor_index: 0,
            revealed_count: 0,
            visited_index: -1,
        };
        discovery::cache::DiscoveryCache::save(&cache_data).ok();

        let _ = app.emit(
            "discovery_complete",
            serde_json::json!({
                "count": suggestions.len(),
            }),
        );
    }

    Ok(suggestions)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryCacheResponse {
    pub suggestions: Vec<discovery::engine::DiscoverySuggestion>,
    pub cursor_index: usize,
    pub revealed_count: usize,
    pub visited_index: i32,
}

#[tauri::command]
pub fn get_discovery_suggestions(
    profile_id: String,
) -> Result<Option<DiscoveryCacheResponse>, String> {
    Ok(
        discovery::cache::DiscoveryCache::load(&profile_id).map(|d| DiscoveryCacheResponse {
            suggestions: d.suggestions,
            cursor_index: d.cursor_index,
            revealed_count: d.revealed_count,
            visited_index: d.visited_index,
        }),
    )
}

#[tauri::command]
pub fn save_discovery_cursor(
    profile_id: String,
    cursor_index: usize,
    revealed_count: usize,
    visited_index: i32,
) -> Result<(), String> {
    discovery::cache::DiscoveryCache::save_cursor(
        &profile_id,
        cursor_index,
        revealed_count,
        visited_index,
    )
}

#[tauri::command]
pub fn update_discovery_pool(
    profile_id: String,
    suggestions: Vec<discovery::engine::DiscoverySuggestion>,
    cursor_index: usize,
    revealed_count: usize,
    visited_index: i32,
) -> Result<(), String> {
    discovery::cache::DiscoveryCache::update_pool(
        &profile_id,
        suggestions,
        cursor_index,
        revealed_count,
        visited_index,
    )
}

#[tauri::command]
pub fn dismiss_discovery(
    state: State<'_, AppState>,
    profile_id: String,
    video_id: String,
) -> Result<(), String> {
    discovery::cache::DiscoveryCache::dismiss(&profile_id, &video_id)?;

    // Clean up the cached audio file in background (best-effort)
    let cache = state.youtube_cache.clone();
    let vid = video_id.clone();
    std::thread::spawn(move || {
        if let Ok(mut cache) = cache.lock() {
            cache.ensure_loaded();
            cache.remove_entry_by_video_id(&vid);
        }
    });

    Ok(())
}

#[tauri::command]
pub fn dislike_discovery(
    state: State<'_, AppState>,
    profile_id: String,
    video_id: String,
) -> Result<(), String> {
    storage::update_disliked_videos(&profile_id, &video_id, true)?;

    // Also dismiss from current discovery cache
    let _ = discovery::cache::DiscoveryCache::dismiss(&profile_id, &video_id);

    // Clean up cached audio in background (best-effort)
    let cache = state.youtube_cache.clone();
    let vid = video_id.clone();
    std::thread::spawn(move || {
        if let Ok(mut cache) = cache.lock() {
            cache.ensure_loaded();
            cache.remove_entry_by_video_id(&vid);
        }
    });

    Ok(())
}

#[tauri::command]
pub fn undislike_discovery(profile_id: String, video_id: String) -> Result<(), String> {
    storage::update_disliked_videos(&profile_id, &video_id, false)?;

    Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DislikedVideoInfo {
    pub video_id: String,
    pub title: String,
    pub channel: String,
    pub duration: f64,
    pub url: String,
}

#[tauri::command]
pub async fn list_disliked_videos(profile_id: String) -> Result<Vec<DislikedVideoInfo>, String> {
    use futures::stream::{self, StreamExt};

    let profile = storage::load_profile(profile_id)?;

    if profile.disliked_videos.is_empty() {
        return Ok(Vec::new());
    }

    let yt_dlp_bin = match youtube::downloader::get_yt_dlp_bin() {
        Ok(path) => path,
        Err(_) => youtube::download_yt_dlp().await?,
    };

    let results: Vec<DislikedVideoInfo> = stream::iter(profile.disliked_videos.iter().cloned())
        .map(|video_id| {
            let bin = yt_dlp_bin.clone();
            async move {
                let url = youtube::downloader::canonical_url(&video_id);

                let mut cmd = youtube::downloader::yt_dlp_command(&bin);
                cmd.arg(&url)
                    .arg("--dump-json")
                    .arg("--no-download")
                    .arg("--no-warnings")
                    .arg("--no-check-certificates")
                    .arg("--no-playlist")
                    .arg("--socket-timeout")
                    .arg("10")
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped());

                if let Ok(child) = cmd.spawn() {
                    if let Ok(Ok(out)) = tokio::time::timeout(
                        std::time::Duration::from_secs(10),
                        child.wait_with_output(),
                    )
                    .await
                    {
                        if out.status.success() {
                            let stdout = String::from_utf8_lossy(&out.stdout);
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                                let title = json
                                    .get("title")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Unknown")
                                    .to_string();
                                let channel = json
                                    .get("channel")
                                    .or_else(|| json.get("uploader"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let duration =
                                    json.get("duration").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                return DislikedVideoInfo {
                                    video_id,
                                    title,
                                    channel,
                                    duration,
                                    url,
                                };
                            }
                        }
                    }
                }

                // Fallback: minimal info
                DislikedVideoInfo {
                    video_id: video_id.clone(),
                    title: format!("Video {}", video_id),
                    channel: String::new(),
                    duration: 0.0,
                    url,
                }
            }
        })
        .buffer_unordered(5)
        .collect()
        .await;

    Ok(results)
}

#[tauri::command]
pub fn cancel_discovery(state: State<'_, AppState>) {
    use std::sync::atomic::Ordering;
    state.discovery_cancel.store(true, Ordering::Relaxed);
}

// ─── Pre-download Commands ───────────────────────────────────────────────

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PredownloadResult {
    pub video_id: String,
    pub cached_path: String,
    pub title: String,
    pub duration: f64,
    pub waveform: Option<analysis::WaveformData>,
}

/// Pre-download a suggestion's audio to cache WITHOUT adding it to the profile.
/// Returns cached path, duration, and waveform data in one call.
#[tauri::command]
pub async fn predownload_suggestion(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    url: String,
    video_id: String,
    download_id: String,
) -> Result<PredownloadResult, String> {
    let cache = state.youtube_cache.clone();

    let app_handle = app.clone();
    let did = download_id.clone();
    let on_progress: youtube::downloader::ProgressCallback = Box::new(move |status, progress| {
        let _ = app_handle.emit(
            "youtube_download_progress",
            serde_json::json!({
                "downloadId": did,
                "status": status,
                "progress": progress,
            }),
        );
    });

    let entry = youtube::download_audio(&url, cache, Some(on_progress)).await?;

    let cached_path = entry.cached_path.clone();

    // Compute waveform (includes duration + momentum detection) during predownload
    // so momentum is available immediately when the user navigates to this suggestion
    tracing::info!(
        "[cpu-pool] predownload_suggestion waveform START: {}",
        video_id
    );
    let waveform_path = cached_path.clone();
    let cache_path = cached_path.clone();
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

    // Cache waveform for future get_waveform calls
    if let Some(ref wf) = waveform {
        let mut cache = state.waveform_cache.lock().unwrap();
        cache.insert(cache_path, wf.clone());
    }

    Ok(PredownloadResult {
        video_id,
        cached_path,
        title: entry.title,
        duration,
        waveform,
    })
}

// ─── Error Handling Commands ──────────────────────────────────────────────

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingSoundInfo {
    pub sound_id: String,
    pub sound_name: String,
    pub file_path: String,
    pub source_type: String,
}

/// Verify that all sound files in a profile exist on disk.
/// Returns a list of missing sounds.
#[tauri::command]
pub fn verify_profile_sounds(profile: Profile) -> Vec<MissingSoundInfo> {
    let mut missing = Vec::new();

    for sound in &profile.sounds {
        let (file_path, source_type) = match &sound.source {
            SoundSource::Local { path } => (path.clone(), "local".to_string()),
            SoundSource::YouTube { cached_path, .. } => {
                (cached_path.clone(), "youtube".to_string())
            }
        };

        if !std::path::Path::new(&file_path).exists() {
            missing.push(MissingSoundInfo {
                sound_id: sound.id.clone(),
                sound_name: sound.name.clone(),
                file_path,
                source_type,
            });
        }
    }

    missing
}

/// Open a file picker dialog for selecting an audio file.
#[tauri::command]
pub async fn pick_audio_file() -> Result<Option<String>, String> {
    let result = tokio::task::spawn_blocking(move || {
        rfd::FileDialog::new()
            .add_filter("Audio Files", &["mp3", "wav", "ogg", "flac", "m4a", "aac"])
            .pick_file()
            .map(|p| p.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| format!("File dialog failed: {}", e))?;

    Ok(result)
}

/// Open a file picker dialog for selecting multiple audio files.
#[tauri::command]
pub async fn pick_audio_files() -> Result<Vec<String>, String> {
    let result = tokio::task::spawn_blocking(move || {
        rfd::FileDialog::new()
            .add_filter("Audio Files", &["mp3", "wav", "ogg", "flac", "m4a", "aac"])
            .pick_files()
            .unwrap_or_default()
            .into_iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect::<Vec<_>>()
    })
    .await
    .map_err(|e| format!("File dialog failed: {}", e))?;

    Ok(result)
}

/// Get the path to the logs folder.
#[tauri::command]
pub fn get_logs_folder() -> Result<String, String> {
    let logs_dir = storage::get_app_data_dir().join("logs");
    Ok(logs_dir.to_string_lossy().to_string())
}

#[tauri::command]
pub fn get_data_folder() -> Result<String, String> {
    let data_dir = storage::get_app_data_dir();
    Ok(data_dir.to_string_lossy().to_string())
}

#[tauri::command]
pub fn open_folder(path: String) -> Result<(), String> {
    let path = std::path::Path::new(&path);
    if !path.exists() {
        return Err("Folder does not exist".to_string());
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ─── Startup Command ──────────────────────────────────────────────────────

/// Unified initial state command — replaces 3 sequential IPC calls with 1.
/// Returns config + profile list + current profile (if any) in a single round-trip.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputRuntime {
    pub is_linux: bool,
    pub is_wayland: bool,
    pub browser_key_fallback: bool,
}

fn is_wayland_session() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
        || std::env::var("XDG_SESSION_TYPE")
            .map(|value| value.eq_ignore_ascii_case("wayland"))
            .unwrap_or(false)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitialState {
    pub config: AppConfig,
    pub profiles: Vec<storage::ProfileSummary>,
    pub current_profile: Option<Profile>,
    pub input_runtime: InputRuntime,
}

#[tauri::command]
pub fn get_initial_state(state: State<'_, AppState>) -> Result<InitialState, String> {
    let config = state.get_config();
    let profiles = storage::list_profiles()?;
    let current_profile = if let Some(ref id) = config.current_profile_id {
        storage::load_profile(id.clone()).ok()
    } else {
        None
    };
    let is_linux = cfg!(target_os = "linux");
    let is_wayland = is_linux && is_wayland_session();

    Ok(InitialState {
        config,
        profiles,
        current_profile,
        input_runtime: InputRuntime {
            is_linux,
            is_wayland,
            browser_key_fallback: is_wayland,
        },
    })
}

// ─── Mood AI Commands ────────────────────────────────────────────────────────

#[tauri::command]
pub fn check_llama_server_installed() -> bool {
    mood::llama_manager::is_llama_server_installed()
}

#[tauri::command]
pub async fn install_llama_server() -> Result<String, String> {
    let path = mood::llama_manager::download_llama_server().await?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn check_mood_model_installed() -> bool {
    mood::llama_manager::is_model_downloaded()
}

#[tauri::command]
pub async fn install_mood_model(app_handle: tauri::AppHandle) -> Result<String, String> {
    let path = mood::llama_manager::download_model(move |downloaded, total| {
        let _ = app_handle.emit(
            "mood_model_download_progress",
            serde_json::json!({ "downloaded": downloaded, "total": total }),
        );
    })
    .await?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn start_mood_server(
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let model_path = mood::llama_manager::get_model_path();
    let mmproj_path = mood::llama_manager::get_mmproj_path();
    if !model_path.exists() {
        return Err("LLM model not downloaded".to_string());
    }
    if !mmproj_path.exists() {
        return Err("Vision encoder (mmproj) not downloaded".to_string());
    }

    let _ = app_handle.emit(
        "mood_server_status",
        serde_json::json!({ "status": "starting" }),
    );

    let model_str = model_path.to_string_lossy().to_string();
    let mmproj_str = mmproj_path.to_string_lossy().to_string();
    let server = mood::inference::LlamaServer::start_with_options(
        &model_str,
        &mmproj_str,
        mood::inference::LlamaServerStartOptions {
            reasoning_format: mood::inference::reasoning_format_from_env(),
            context_size: None,
            parallel_slots: None,
            gpu_layers: None,
            runtime_intent: Some(mood::inference::LlamaRuntimeIntent::AppDefault),
        },
    )
    .await?;

    let _ = app_handle.emit(
        "mood_server_status",
        serde_json::json!({ "status": "running" }),
    );

    let mut guard = state.llama_server.lock().await;
    *guard = Some(server);

    // Start the HTTP API server if mood AI is enabled
    let config = state.get_config();
    if config.mood_ai_enabled {
        let llama_ref = state.llama_server.clone();
        let port = config.mood_api_port;
        let handle = app_handle.clone();
        let cache_ref = state.mood_cache.clone();
        let director_ref = state.mood_director.clone();
        let api_handle = tokio::spawn(async move {
            if let Err(e) =
                mood::server::start_api_server(port, llama_ref, handle, cache_ref, director_ref)
                    .await
            {
                tracing::error!("Mood API server error: {}", e);
            }
        });
        let mut api_guard = state
            .mood_api_server
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *api_guard = Some(api_handle);
    }

    Ok(())
}

#[tauri::command]
pub async fn stop_mood_server(
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    // Stop API server
    {
        let mut api_guard = state
            .mood_api_server
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(handle) = api_guard.take() {
            handle.abort();
        }
    }

    // Stop llama-server
    {
        let mut guard = state.llama_server.lock().await;
        if let Some(mut server) = guard.take() {
            server.stop();
        }
    }

    // Reset director state
    {
        let mut director = state
            .mood_director
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        director.reset();
    }

    let _ = app_handle.emit(
        "mood_server_status",
        serde_json::json!({ "status": "stopped" }),
    );

    Ok(())
}

#[tauri::command]
pub async fn get_mood_server_status(state: State<'_, AppState>) -> Result<String, String> {
    let mut guard = state.llama_server.lock().await;
    let is_running = guard.as_mut().map(|s| s.is_running()).unwrap_or(false);
    if guard.is_some() && !is_running {
        *guard = None;
        Ok("stopped".to_string())
    } else if is_running {
        Ok("running".to_string())
    } else {
        Ok("stopped".to_string())
    }
}

#[tauri::command]
pub async fn analyze_mood(
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
    image_path: String,
) -> Result<MoodCategory, String> {
    let image_data =
        std::fs::read(&image_path).map_err(|e| format!("Failed to read image: {}", e))?;

    let image_b64 = mood::inference::prepare_image(&image_data)?;

    // Build narrative context from director
    let narrative_context = {
        let director = state
            .mood_director
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let previous_moods: Vec<String> = director
            .window_moods()
            .iter()
            .map(|s| s.to_string())
            .collect();
        let current_soundtrack = director.committed_mood().map(|m| m.as_str().to_string());
        let soundtrack_dwell = director.dwell_count();
        mood::inference::NarrativeContext {
            previous_moods,
            current_soundtrack,
            soundtrack_dwell,
            next_moods: Vec::new(), // No cache look-ahead for local analysis
        }
    };

    let mut guard = state.llama_server.lock().await;
    let server = guard
        .as_mut()
        .ok_or_else(|| "Mood server not running".to_string())?;

    let (scores, narrative_role) = server
        .analyze_mood_scored(&image_b64, Some(&narrative_context))
        .await?;

    let dominant_mood = scores.dominant();
    let mood_str = dominant_mood.as_str().to_string();

    // Feed into director
    let decision = {
        let analysis = mood::director::PageAnalysis {
            scores,
            intensity: crate::types::MoodIntensity::Medium, // TODO: use real intensity
            narrative_role,
            dominant_mood,
        };
        let mut director = state
            .mood_director
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        director.process(analysis, None)
    };

    // Always emit raw mood for UI
    let _ = app_handle.emit(
        "mood_detected",
        serde_json::json!({ "mood": mood_str, "source": "local" }),
    );

    // Emit committed mood only if changed
    if decision.mood_changed {
        let committed_str = decision.committed_mood.as_str().to_string();
        let _ = app_handle.emit(
            "mood_committed",
            serde_json::json!({
                "mood": committed_str,
                "source": "local",
                "previous_mood": mood_str,
                "dwell_count": decision.dwell_count,
            }),
        );
    }

    Ok(decision.committed_mood)
}
