//! Profiles, import/export, missing files and data folders.

use std::collections::HashMap;

use crate::discovery;
use crate::events::CoreEvent;
use crate::import_export;
use crate::state::AppState;
use crate::storage;
use crate::types::{AppConfig, KeyBinding, LoopMode, Profile, Sound, SoundSource, Track};
use crate::youtube;

use super::input_access::is_wayland_session;

/// Delete a profile with its discovery cache, then drop YouTube files nothing references anymore.
pub fn delete_profile(state: &AppState, id: String) -> Result<(), String> {
    storage::delete_profile(id.clone())?;
    discovery::cache::DiscoveryCache::delete(&id);
    // Scan profiles before locking so downloads and lookups are not blocked meanwhile.
    let usage = youtube::cache::CacheUsage::collect();
    if let Ok(mut cache) = state.youtube_cache.lock() {
        cache.cleanup_unused_with(&usage);
    }
    Ok(())
}

/// Export a profile to `.ktm`, bundling known waveforms and emitting `ExportProgress`.
pub async fn export_profile(
    state: &AppState,
    profile_id: String,
    output_path: String,
) -> Result<(), String> {
    let waveforms = {
        let profile = storage::load_profile(profile_id.clone())?;
        let mut map = HashMap::new();
        if let Ok(mut cache) = state.waveform_cache.lock() {
            for sound in &profile.sounds {
                let path = sound_file_path(sound);
                if let Some(data) = cache.get(&path) {
                    map.insert(path, data.clone());
                }
            }
        }
        map
    };

    let sink = state.event_sink();
    tokio::task::spawn_blocking(move || {
        let progress_cb: import_export::export::ProgressCallback =
            Box::new(move |current, total, filename| {
                sink.emit(CoreEvent::ExportProgress {
                    current,
                    total,
                    filename: filename.to_string(),
                });
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

/// Import a `.ktm` file; returns the new profile id.
pub async fn import_profile(state: &AppState, ktm_path: String) -> Result<String, String> {
    let import_result =
        tokio::task::spawn_blocking(move || import_export::import_profile(&ktm_path))
            .await
            .map_err(|e| format!("Import task failed: {}", e))??;

    if !import_result.waveforms.is_empty() {
        if let Ok(mut cache) = state.waveform_cache.lock() {
            for (path, data) in import_result.waveforms {
                cache.insert(path, data);
            }
        }
    }

    let usage = tokio::task::spawn_blocking(youtube::cache::CacheUsage::collect)
        .await
        .map_err(|e| format!("Cache scan failed: {}", e))?;
    if let Ok(mut cache) = state.youtube_cache.lock() {
        cache.cleanup_unused_with(&usage);
    }

    Ok(import_result.profile_id)
}

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

/// Import a legacy KeyToMusic (Unity) save file and convert it to a new profile.
pub fn import_legacy_save(path: String) -> Result<Profile, String> {
    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read legacy save: {}", e))?;

    let legacy: LegacySave = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse legacy save: {}", e))?;

    let now = chrono::Utc::now().to_rfc3339();
    let profile_id = uuid::Uuid::new_v4().to_string();
    let track_id = uuid::Uuid::new_v4().to_string();

    let track = Track {
        id: track_id.clone(),
        name: "OST".to_string(),
        volume: 1.0,
    };

    let mut sounds: Vec<Sound> = Vec::new();
    let mut key_bindings: Vec<KeyBinding> = Vec::new();

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
            key_bindings.push(KeyBinding {
                key_code,
                track_id: track_id.clone(),
                sound_ids,
                loop_mode: LoopMode::Off,
                current_index: 0,
                name: None,
                mood: None,
                mood_intensity: None,
            });
        }
    }

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

/// File on disk a sound plays from.
pub fn sound_file_path(sound: &Sound) -> String {
    match &sound.source {
        SoundSource::Local { path } => path.clone(),
        SoundSource::YouTube { cached_path, .. } => cached_path.clone(),
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingSoundInfo {
    pub sound_id: String,
    pub sound_name: String,
    pub file_path: String,
    pub source_type: String,
}

/// Sounds of `profile` whose file is missing on disk.
pub fn verify_profile_sounds(profile: &Profile) -> Vec<MissingSoundInfo> {
    profile
        .sounds
        .iter()
        .filter_map(|sound| {
            let file_path = sound_file_path(sound);
            if std::path::Path::new(&file_path).exists() {
                return None;
            }
            let source_type = match &sound.source {
                SoundSource::Local { .. } => "local",
                SoundSource::YouTube { .. } => "youtube",
            };
            Some(MissingSoundInfo {
                sound_id: sound.id.clone(),
                sound_name: sound.name.clone(),
                file_path,
                source_type: source_type.to_string(),
            })
        })
        .collect()
}

pub fn logs_folder() -> String {
    storage::get_app_data_dir()
        .join("logs")
        .to_string_lossy()
        .to_string()
}

pub fn data_folder() -> String {
    storage::get_app_data_dir().to_string_lossy().to_string()
}

/// Reveal a folder in the platform file manager.
pub fn open_folder(path: &str) -> Result<(), String> {
    let path = std::path::Path::new(path);
    if !path.exists() {
        return Err("Folder does not exist".to_string());
    }
    #[cfg(target_os = "windows")]
    let opener = "explorer";
    #[cfg(target_os = "macos")]
    let opener = "open";
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let opener = "xdg-open";
    std::process::Command::new(opener)
        .arg(path)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputRuntime {
    pub is_linux: bool,
    pub is_wayland: bool,
    pub browser_key_fallback: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitialState {
    pub config: AppConfig,
    pub profiles: Vec<storage::ProfileSummary>,
    pub current_profile: Option<Profile>,
    pub input_runtime: InputRuntime,
    pub profile_error: Option<String>,
}

/// Config + profile list + current profile in one call.
pub fn initial_state(state: &AppState) -> Result<InitialState, String> {
    let config = state.get_config();
    let profiles = storage::list_profiles()?;
    let (current_profile, profile_error) = if let Some(ref id) = config.current_profile_id {
        match storage::load_profile(id.clone()) {
            Ok(profile) => (Some(profile), None),
            Err(error) => (None, Some(error)),
        }
    } else {
        (None, None)
    };
    let is_linux = cfg!(target_os = "linux");
    let is_wayland = is_linux && is_wayland_session();

    Ok(InitialState {
        config,
        profiles,
        current_profile,
        profile_error,
        input_runtime: InputRuntime {
            is_linux,
            is_wayland,
            browser_key_fallback: is_wayland,
        },
    })
}
