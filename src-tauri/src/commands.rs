use crate::audio::{self, buffer::BufferManager};
use crate::import_export;
use crate::state::AppState;
use crate::storage;
use crate::types::{AppConfig, Profile, Sound, SoundSource};
use crate::youtube;
use tauri::{Emitter, State};

// ─── Configuration Commands ─────────────────────────────────────────────────

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    Ok(state.get_config())
}

#[tauri::command]
pub fn update_config(
    state: State<'_, AppState>,
    updates: serde_json::Value,
) -> Result<(), String> {
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
        if let Some(v) = updates.get("masterStopShortcut").and_then(|v| v.as_array()) {
            let keys: Vec<String> = v
                .iter()
                .filter_map(|k| k.as_str().map(|s| s.to_string()))
                .collect();
            if !keys.is_empty() {
                config.master_stop_shortcut = keys;
            }
        }
        if let Some(v) = updates.get("autoMomentumShortcut").and_then(|v| v.as_array()) {
            config.auto_momentum_shortcut = v
                .iter()
                .filter_map(|k| k.as_str().map(|s| s.to_string()))
                .collect();
        }
        if let Some(v) = updates.get("keyDetectionShortcut").and_then(|v| v.as_array()) {
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
            config.audio_device = updates
                .get("audioDevice")
                .and_then(|v| {
                    if v.is_null() { None } else { v.as_str().map(|s| s.to_string()) }
                });
        }
    });

    // Sync audio device to audio engine
    if updates.get("audioDevice").is_some() {
        let _ = state.audio_engine.set_audio_device(config.audio_device.clone());
    }

    // Sync master volume to audio engine
    if updates.get("masterVolume").is_some() {
        let _ = state.audio_engine.set_master_volume(config.master_volume);
    }

    // Sync key detection settings
    if updates.get("keyDetectionEnabled").is_some() {
        state.key_detector.set_enabled(config.key_detection_enabled);
    }
    if updates.get("keyCooldown").is_some() {
        state.key_detector.set_cooldown(config.key_cooldown);
    }
    if updates.get("masterStopShortcut").is_some() {
        state.key_detector.set_master_stop_shortcut(config.master_stop_shortcut.clone());
    }
    if updates.get("autoMomentumShortcut").is_some() {
        state.key_detector.set_auto_momentum_shortcut(config.auto_momentum_shortcut.clone());
    }
    if updates.get("keyDetectionShortcut").is_some() {
        state.key_detector.set_key_detection_shortcut(config.key_detection_shortcut.clone());
    }

    storage::save_config(&config)
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
pub fn save_profile(state: State<'_, AppState>, profile: Profile) -> Result<(), String> {
    storage::save_profile(&profile)?;
    // Cleanup cached YouTube files no longer referenced by any profile
    if let Ok(mut cache) = state.youtube_cache.lock() {
        cache.cleanup_unused();
    }
    Ok(())
}

#[tauri::command]
pub fn delete_profile(state: State<'_, AppState>, id: String) -> Result<(), String> {
    storage::delete_profile(id)?;
    // Cleanup cached YouTube files no longer referenced by any profile
    if let Ok(mut cache) = state.youtube_cache.lock() {
        cache.cleanup_unused();
    }
    Ok(())
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
    let config = state.get_config();

    // Check file exists
    if !std::path::Path::new(&file_path).exists() {
        tracing::warn!("Sound file not found: {} (track: {}, sound: {})", file_path, track_id, sound_id);
        // Play error sound
        let _ = state.audio_engine.play_error_sound();
        // Emit sound_not_found event
        let _ = app.emit("sound_not_found", serde_json::json!({
            "soundId": sound_id,
            "path": file_path,
            "trackId": track_id,
        }));
        return Err(format!("Sound file not found: {}", file_path));
    }

    state.audio_engine.play_sound(
        track_id,
        sound_id,
        file_path,
        start_position,
        sound_volume,
        config.crossfade_duration,
    )
}

#[tauri::command]
pub fn stop_sound(state: State<'_, AppState>, track_id: String) -> Result<(), String> {
    state.audio_engine.stop_track(track_id)
}

#[tauri::command]
pub fn stop_all_sounds(state: State<'_, AppState>) -> Result<(), String> {
    state.audio_engine.stop_all()
}

#[tauri::command]
pub fn set_master_volume(state: State<'_, AppState>, volume: f32) -> Result<(), String> {
    // Update config
    let config = state.update_config(|config| {
        config.master_volume = volume;
    });
    storage::save_config(&config)?;

    // Update audio engine
    state.audio_engine.set_master_volume(volume)
}

#[tauri::command]
pub fn set_track_volume(
    state: State<'_, AppState>,
    track_id: String,
    volume: f32,
) -> Result<(), String> {
    state.audio_engine.set_track_volume(track_id, volume)
}

#[tauri::command]
pub fn set_sound_volume(
    state: State<'_, AppState>,
    track_id: String,
    sound_id: String,
    volume: f32,
) -> Result<(), String> {
    state.audio_engine.set_sound_volume(track_id, sound_id, volume)
}

#[tauri::command]
pub async fn get_audio_duration(path: String) -> Result<f64, String> {
    tokio::task::spawn_blocking(move || {
        BufferManager::get_audio_duration(&path)
    })
    .await
    .map_err(|e| format!("Task failed: {}", e))?
}

// ─── Sound Pre-loading ─────────────────────────────────────────────────────

/// Batch compute durations for sounds that need it.
/// Uses parallel threads for speed. Returns a map of soundId -> duration.
#[tauri::command]
pub async fn preload_profile_sounds(
    sounds: Vec<SoundPreloadEntry>,
) -> Result<std::collections::HashMap<String, f64>, String> {
    tokio::task::spawn_blocking(move || {
        use std::sync::Mutex;

        let durations: Mutex<std::collections::HashMap<String, f64>> =
            Mutex::new(std::collections::HashMap::new());

        // Only process sounds that actually need duration
        let needs_work: Vec<&SoundPreloadEntry> = sounds
            .iter()
            .filter(|e| e.needs_duration)
            .collect();

        if needs_work.is_empty() {
            return durations.into_inner().unwrap();
        }

        // Process in parallel using scoped threads (2 threads)
        std::thread::scope(|scope| {
            let chunk_size = (needs_work.len() / 2).max(1);
            for chunk in needs_work.chunks(chunk_size) {
                let durations = &durations;

                scope.spawn(move || {
                    for entry in chunk {
                        let path = std::path::Path::new(&entry.file_path);
                        if !path.exists() {
                            continue;
                        }

                        if let Ok(dur) = BufferManager::get_audio_duration(&entry.file_path) {
                            if dur > 0.0 {
                                durations.lock().unwrap().insert(entry.sound_id.clone(), dur);
                            }
                        }
                    }
                });
            }
        });

        durations.into_inner().unwrap()
    })
    .await
    .map_err(|e| format!("Preload task failed: {}", e))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SoundPreloadEntry {
    pub sound_id: String,
    pub file_path: String,
    pub needs_duration: bool,
}

// ─── Key Detection Commands ────────────────────────────────────────────────

#[tauri::command]
pub fn set_key_detection(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    state.key_detector.set_enabled(enabled);

    // Also update config
    let config = state.update_config(|config| {
        config.key_detection_enabled = enabled;
    });
    storage::save_config(&config)
}

#[tauri::command]
pub fn set_master_stop_shortcut(
    state: State<'_, AppState>,
    keys: Vec<String>,
) -> Result<(), String> {
    if keys.len() < 2 {
        return Err("Master stop shortcut must have at least 2 keys".to_string());
    }

    // Update the detector
    state.key_detector.set_master_stop_shortcut(keys.clone());

    // Update config
    let config = state.update_config(|config| {
        config.master_stop_shortcut = keys;
    });
    storage::save_config(&config)
}

#[tauri::command]
pub fn set_key_cooldown(state: State<'_, AppState>, cooldown_ms: u32) -> Result<(), String> {
    if cooldown_ms > 5000 {
        return Err("Cooldown must be at most 5000ms".to_string());
    }

    // Update the detector
    state.key_detector.set_cooldown(cooldown_ms);

    // Update config
    let config = state.update_config(|config| {
        config.key_cooldown = cooldown_ms;
    });
    storage::save_config(&config)
}

// ─── Audio Device Commands ────────────────────────────────────────────────

#[tauri::command]
pub fn list_audio_devices() -> Vec<String> {
    audio::list_audio_devices()
}

#[tauri::command]
pub fn set_audio_device(
    state: State<'_, AppState>,
    device: Option<String>,
) -> Result<(), String> {
    // Update audio engine
    state.audio_engine.set_audio_device(device.clone())?;

    // Update config
    let config = state.update_config(|config| {
        config.audio_device = device;
    });
    storage::save_config(&config)
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
        let _ = app_handle.emit("youtube_download_progress", serde_json::json!({
            "downloadId": did,
            "status": status,
            "progress": progress,
        }));
    });

    let entry = youtube::download_audio(&url, cache, Some(on_progress)).await?;

    // Compute duration
    let cached_path = entry.cached_path.clone();
    let duration = tokio::task::spawn_blocking(move || {
        BufferManager::get_audio_duration(&cached_path).unwrap_or(0.0)
    })
    .await
    .map_err(|e| format!("Duration task failed: {}", e))?;

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
    };

    Ok(sound)
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
    profile_id: String,
    output_path: String,
) -> Result<(), String> {
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
        import_export::export_profile(&profile_id, &output_path, Some(progress_cb))
    })
    .await
    .map_err(|e| format!("Export task failed: {}", e))?
}

#[tauri::command]
pub async fn import_profile(
    state: State<'_, AppState>,
    ktm_path: String,
) -> Result<String, String> {
    let result = tokio::task::spawn_blocking(move || {
        import_export::import_profile(&ktm_path)
    })
    .await
    .map_err(|e| format!("Import task failed: {}", e))??;

    // Cleanup unused cache entries after import
    if let Ok(mut cache) = state.youtube_cache.lock() {
        cache.cleanup_unused();
    }

    Ok(result)
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
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read legacy save: {}", e))?;

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
        currently_playing: None,
        playback_position: 0.0,
        is_playing: false,
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
    };

    storage::save_profile(&profile)?;

    tracing::info!("Imported legacy save as profile '{}' with {} sounds and {} key bindings",
        profile.name, profile.sounds.len(), profile.key_bindings.len());

    Ok(profile)
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
            SoundSource::YouTube { cached_path, .. } => (cached_path.clone(), "youtube".to_string()),
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
