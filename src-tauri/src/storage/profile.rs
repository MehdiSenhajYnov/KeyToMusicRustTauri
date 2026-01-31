use crate::storage::get_app_data_dir;
use crate::types::{Profile, ProfileId};
use std::fs;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSummary {
    pub id: ProfileId,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

pub fn list_profiles() -> Result<Vec<ProfileSummary>, String> {
    let profiles_dir = get_app_data_dir().join("profiles");

    if !profiles_dir.exists() {
        return Ok(Vec::new());
    }

    let mut summaries = Vec::new();

    for entry in fs::read_dir(&profiles_dir).map_err(|e| format!("Failed to read profiles dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            if let Ok(contents) = fs::read_to_string(&path) {
                if let Ok(profile) = serde_json::from_str::<Profile>(&contents) {
                    summaries.push(ProfileSummary {
                        id: profile.id,
                        name: profile.name,
                        created_at: profile.created_at,
                        updated_at: profile.updated_at,
                    });
                }
            }
        }
    }

    Ok(summaries)
}

pub fn create_profile(name: String) -> Result<Profile, String> {
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
    };

    save_profile(&profile)?;
    Ok(profile)
}

pub fn load_profile(id: String) -> Result<Profile, String> {
    let profile_path = get_app_data_dir().join("profiles").join(format!("{}.json", id));

    if !profile_path.exists() {
        return Err(format!("Profile not found: {}", id));
    }

    let contents = fs::read_to_string(&profile_path)
        .map_err(|e| format!("Failed to read profile: {}", e))?;

    serde_json::from_str(&contents).map_err(|e| format!("Failed to parse profile: {}", e))
}

pub fn save_profile(profile: &Profile) -> Result<(), String> {
    let profile_path = get_app_data_dir()
        .join("profiles")
        .join(format!("{}.json", profile.id));
    let tmp_path = profile_path.with_extension("json.tmp");

    let json = serde_json::to_string_pretty(profile)
        .map_err(|e| format!("Failed to serialize profile: {}", e))?;

    fs::write(&tmp_path, json).map_err(|e| format!("Failed to write profile temp file: {}", e))?;
    fs::rename(&tmp_path, &profile_path).map_err(|e| format!("Failed to rename profile file: {}", e))?;

    Ok(())
}

pub fn delete_profile(id: String) -> Result<(), String> {
    let profile_path = get_app_data_dir().join("profiles").join(format!("{}.json", id));

    if !profile_path.exists() {
        return Err(format!("Profile not found: {}", id));
    }

    fs::remove_file(&profile_path).map_err(|e| format!("Failed to delete profile: {}", e))?;

    Ok(())
}

/// Duplicate an existing profile with a new UUID and optionally a new name.
/// If new_name is None, the duplicated profile will be named "{original_name} (Copy)".
pub fn duplicate_profile(id: String, new_name: Option<String>) -> Result<Profile, String> {
    let source = load_profile(id)?;

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
    };

    save_profile(&new_profile)?;
    Ok(new_profile)
}
