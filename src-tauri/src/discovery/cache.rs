use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::engine::DiscoverySuggestion;
use crate::storage::config::get_app_data_dir;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryCacheData {
    pub profile_id: String,
    pub seed_hash: String,
    pub generated_at: String,
    pub suggestions: Vec<DiscoverySuggestion>,
    #[serde(default)]
    pub dismissed_ids: Vec<String>,
}

pub struct DiscoveryCache;

impl DiscoveryCache {
    fn cache_dir() -> PathBuf {
        get_app_data_dir().join("discovery")
    }

    fn cache_path(profile_id: &str) -> PathBuf {
        Self::cache_dir().join(format!("{}.json", profile_id))
    }

    pub fn load(profile_id: &str) -> Option<DiscoveryCacheData> {
        let path = Self::cache_path(profile_id);
        if !path.exists() {
            return None;
        }
        let content = fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn save(data: &DiscoveryCacheData) -> Result<(), String> {
        let dir = Self::cache_dir();
        fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create discovery cache dir: {}", e))?;

        let path = Self::cache_path(&data.profile_id);
        let json = serde_json::to_string_pretty(data)
            .map_err(|e| format!("Failed to serialize discovery cache: {}", e))?;

        // Atomic write
        let tmp_path = path.with_extension("json.tmp");
        fs::write(&tmp_path, &json)
            .map_err(|e| format!("Failed to write discovery cache: {}", e))?;
        fs::rename(&tmp_path, &path)
            .map_err(|e| format!("Failed to rename discovery cache: {}", e))?;

        Ok(())
    }

    pub fn delete(profile_id: &str) {
        let path = Self::cache_path(profile_id);
        let _ = fs::remove_file(path);
    }

    /// Compute a hash of sorted seed video IDs to detect when re-generation is needed.
    pub fn compute_seed_hash(seed_ids: &[String]) -> String {
        let mut sorted = seed_ids.to_vec();
        sorted.sort();
        // Simple hash: join and hash
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        sorted.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Dismiss a suggestion by video ID.
    pub fn dismiss(profile_id: &str, video_id: &str) -> Result<(), String> {
        let mut data = Self::load(profile_id)
            .ok_or_else(|| "No discovery cache found".to_string())?;

        let dismissed: HashSet<String> = data.dismissed_ids.iter().cloned().collect();
        if !dismissed.contains(video_id) {
            data.dismissed_ids.push(video_id.to_string());
        }

        // Also remove from suggestions
        data.suggestions.retain(|s| s.video_id != video_id);

        Self::save(&data)
    }
}
