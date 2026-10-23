use crate::storage::get_app_data_dir;
use crate::types::{Profile, ProfileId, SoundSource};
use std::fs;
use std::sync::Mutex;

static PROFILE_WRITES: Mutex<()> = Mutex::new(());

fn profile_path(id: &str) -> Result<std::path::PathBuf, String> {
    if id.is_empty()
        || !id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
    {
        return Err("Invalid profile identifier".to_string());
    }
    Ok(get_app_data_dir()
        .join("profiles")
        .join(format!("{}.json", id)))
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSummary {
    pub id: ProfileId,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

/// List all profiles by extracting only the 4 summary fields (id, name, createdAt, updatedAt).
/// Uses serde_json::Value for partial parsing to avoid deserializing the large
/// sounds/tracks/keyBindings arrays that can be 50KB-1MB per profile.
pub fn list_profiles() -> Result<Vec<ProfileSummary>, String> {
    let profiles_dir = get_app_data_dir().join("profiles");

    if !profiles_dir.exists() {
        return Ok(Vec::new());
    }

    let mut summaries = Vec::new();

    for entry in
        fs::read_dir(&profiles_dir).map_err(|e| format!("Failed to read profiles dir: {}", e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            if let Ok(contents) = fs::read_to_string(&path) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&contents) {
                    let id = val.get("id").and_then(|v| v.as_str()).unwrap_or_default();
                    let name = val.get("name").and_then(|v| v.as_str()).unwrap_or_default();
                    let created_at = val
                        .get("createdAt")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default();
                    let updated_at = val
                        .get("updatedAt")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default();

                    // Skip entries that lack an id (not a valid profile)
                    if !id.is_empty() {
                        summaries.push(ProfileSummary {
                            id: id.to_string(),
                            name: name.to_string(),
                            created_at: created_at.to_string(),
                            updated_at: updated_at.to_string(),
                        });
                    }
                }
            }
        }
    }

    summaries.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then(a.id.cmp(&b.id))
    });
    Ok(summaries)
}

pub fn create_profile(name: String) -> Result<Profile, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Profile name cannot be empty".to_string());
    }
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let profile = Profile {
        id: id.clone(),
        name,
        created_at: now.clone(),
        updated_at: now,
        sounds: Vec::new(),
        tracks: Vec::new(),
        key_bindings: Vec::new(),
        disliked_videos: Vec::new(),
    };

    save_profile(&profile)?;
    Ok(profile)
}

pub fn load_profile(id: String) -> Result<Profile, String> {
    let profile_path = profile_path(&id)?;

    if !profile_path.exists() {
        return Err(format!("Profile not found: {}", id));
    }

    let contents =
        fs::read_to_string(&profile_path).map_err(|e| format!("Failed to read profile: {}", e))?;

    serde_json::from_str(&contents).map_err(|e| format!("Failed to parse profile: {}", e))
}

pub fn save_profile(profile: &Profile) -> Result<(), String> {
    let _guard = PROFILE_WRITES.lock().unwrap_or_else(|e| e.into_inner());
    // Discovery owns these fields and may have updated them since the UI loaded.
    let mut merged = profile.clone();
    if let Ok(existing) = load_profile(profile.id.clone()) {
        merged.disliked_videos = existing.disliked_videos;
        for sound in &mut merged.sounds {
            if let Some(saved) = existing.sounds.iter().find(|s| s.id == sound.id) {
                // A relocated or replaced file needs a fresh resolution.
                if sound.resolved_video_id.is_none() && same_source(&sound.source, &saved.source) {
                    sound.resolved_video_id = saved.resolved_video_id.clone();
                }
            }
        }
    }
    save_profile_unlocked(&merged)
}

fn same_source(a: &SoundSource, b: &SoundSource) -> bool {
    match (a, b) {
        (SoundSource::Local { path: a }, SoundSource::Local { path: b }) => a == b,
        (
            SoundSource::YouTube {
                url: a,
                cached_path: pa,
            },
            SoundSource::YouTube {
                url: b,
                cached_path: pb,
            },
        ) => a == b && pa == pb,
        _ => false,
    }
}

fn save_profile_unlocked(profile: &Profile) -> Result<(), String> {
    let path = profile_path(&profile.id)?;
    if profile.name.trim().is_empty() {
        return Err("Profile name cannot be empty".to_string());
    }
    let json = serde_json::to_vec_pretty(profile)
        .map_err(|e| format!("Failed to serialize profile: {}", e))?;
    super::atomic_write(&path, &json)
}

/// Atomic read→modify→write: merges resolved_video_id fields on matching sounds
/// without touching any other profile data (name, bindings, tracks, etc.).
pub fn update_resolved_video_ids(
    profile_id: &str,
    updates: &[(String, String)], // Vec of (sound_id, video_id)
) -> Result<(), String> {
    let _guard = PROFILE_WRITES.lock().unwrap_or_else(|e| e.into_inner());
    let mut profile = load_profile(profile_id.to_string())?;

    for (sound_id, video_id) in updates {
        if let Some(sound) = profile.sounds.iter_mut().find(|s| s.id == *sound_id) {
            sound.resolved_video_id = Some(video_id.clone());
        }
    }

    profile.updated_at = chrono::Utc::now().to_rfc3339();
    save_profile_unlocked(&profile)
}

/// Atomic read→modify→write: adds or removes a single disliked video ID
/// without touching any other profile data.
pub fn update_disliked_videos(profile_id: &str, video_id: &str, add: bool) -> Result<(), String> {
    let _guard = PROFILE_WRITES.lock().unwrap_or_else(|e| e.into_inner());
    let mut profile = load_profile(profile_id.to_string())?;

    if add {
        if !profile.disliked_videos.contains(&video_id.to_string()) {
            profile.disliked_videos.push(video_id.to_string());
        }
    } else {
        profile.disliked_videos.retain(|id| id != video_id);
    }

    profile.updated_at = chrono::Utc::now().to_rfc3339();
    save_profile_unlocked(&profile)
}

pub fn delete_profile(id: String) -> Result<(), String> {
    let _guard = PROFILE_WRITES.lock().unwrap_or_else(|e| e.into_inner());
    let profile_path = profile_path(&id)?;

    if !profile_path.exists() {
        return Err(format!("Profile not found: {}", id));
    }

    fs::remove_file(&profile_path).map_err(|e| format!("Failed to delete profile: {}", e))?;

    Ok(())
}

/// Duplicate an existing profile with a new UUID and optionally a new name.
/// If new_name is None, the duplicated profile will be named "{original_name} (Copy)".
pub fn duplicate_profile(id: String, new_name: Option<String>) -> Result<Profile, String> {
    let mut source = load_profile(id)?;
    renew_profile_item_ids(&mut source);

    let new_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let name = new_name.unwrap_or_else(|| format!("{} (Copy)", source.name));

    let new_profile = Profile {
        id: new_id,
        name,
        created_at: now.clone(),
        updated_at: now,
        sounds: source.sounds,
        tracks: source.tracks,
        key_bindings: source.key_bindings,
        disliked_videos: Vec::new(),
    };

    save_profile(&new_profile)?;
    Ok(new_profile)
}

fn renew_profile_item_ids(profile: &mut Profile) {
    let sounds: std::collections::HashMap<_, _> = profile
        .sounds
        .iter()
        .map(|s| (s.id.clone(), uuid::Uuid::new_v4().to_string()))
        .collect();
    let tracks: std::collections::HashMap<_, _> = profile
        .tracks
        .iter()
        .map(|t| (t.id.clone(), uuid::Uuid::new_v4().to_string()))
        .collect();
    for sound in &mut profile.sounds {
        sound.id = sounds[&sound.id].clone();
    }
    for track in &mut profile.tracks {
        track.id = tracks[&track.id].clone();
    }
    for binding in &mut profile.key_bindings {
        if let Some(track) = tracks.get(&binding.track_id) {
            binding.track_id = track.clone();
        }
        binding.sound_ids = binding
            .sound_ids
            .iter()
            .filter_map(|id| sounds.get(id).cloned())
            .collect();
        binding.current_index = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profile_identifiers_cannot_escape_the_profiles_directory() {
        for invalid in [
            "",
            "../config",
            "..\\config",
            "/tmp/profile",
            "C:\\profile",
            "a/b",
        ] {
            assert!(profile_path(invalid).is_err());
        }
        assert!(profile_path("legacy-profile_1").is_ok());
    }
    #[test]
    fn duplicate_has_independent_sound_and_track_ids() {
        let mut profile: Profile = serde_json::from_value(serde_json::json!({
            "id":"p", "name":"P", "createdAt":"", "updatedAt":"",
            "tracks":[{"id":"t", "name":"T", "volume":0.2}],
            "sounds":[{"id":"s", "name":"S", "source":{"type":"local", "path":"shared.wav"}, "momentum":0, "volume":1, "duration":10}],
            "keyBindings":[{"keyCode":"KeyA", "trackId":"t", "soundIds":["s"], "loopMode":"single", "currentIndex":2}]
        })).unwrap();
        renew_profile_item_ids(&mut profile);
        assert_ne!(profile.tracks[0].id, "t");
        assert_ne!(profile.sounds[0].id, "s");
        assert_eq!(profile.key_bindings[0].track_id, profile.tracks[0].id);
        assert_eq!(profile.key_bindings[0].sound_ids[0], profile.sounds[0].id);
    }
}
