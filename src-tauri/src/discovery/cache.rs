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
    #[serde(default)]
    pub cursor_index: usize,
    #[serde(default)]
    pub revealed_count: usize,
    #[serde(default = "default_visited_index")]
    pub visited_index: i32,
}

fn default_visited_index() -> i32 {
    -1
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

    /// Append new suggestions to an existing cache, deduplicating by video_id.
    pub fn append_suggestions(
        profile_id: &str,
        new_suggestions: Vec<DiscoverySuggestion>,
        seed_hash: &str,
    ) -> Result<(), String> {
        let mut data = Self::load(profile_id).unwrap_or_else(|| DiscoveryCacheData {
            profile_id: profile_id.to_string(),
            seed_hash: seed_hash.to_string(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            suggestions: Vec::new(),
            dismissed_ids: Vec::new(),
            cursor_index: 0,
            revealed_count: 0,
            visited_index: -1,
        });

        let existing_ids: HashSet<String> = data.suggestions.iter().map(|s| s.video_id.clone()).collect();
        let dismissed: HashSet<&String> = data.dismissed_ids.iter().collect();

        for s in new_suggestions {
            if !existing_ids.contains(&s.video_id) && !dismissed.contains(&s.video_id) {
                data.suggestions.push(s);
            }
        }

        data.seed_hash = seed_hash.to_string();
        data.generated_at = chrono::Utc::now().to_rfc3339();
        Self::save(&data)
    }

    /// Update only the cursor fields in an existing cache.
    pub fn save_cursor(
        profile_id: &str,
        cursor_index: usize,
        revealed_count: usize,
        visited_index: i32,
    ) -> Result<(), String> {
        let mut data = Self::load(profile_id)
            .ok_or_else(|| "No discovery cache found".to_string())?;

        data.cursor_index = cursor_index;
        data.revealed_count = revealed_count;
        data.visited_index = visited_index;
        Self::save(&data)
    }

    /// Replace the suggestion pool and cursor fields in an existing cache.
    pub fn update_pool(
        profile_id: &str,
        suggestions: Vec<DiscoverySuggestion>,
        cursor_index: usize,
        revealed_count: usize,
        visited_index: i32,
    ) -> Result<(), String> {
        let mut data = Self::load(profile_id)
            .ok_or_else(|| "No discovery cache found".to_string())?;

        data.suggestions = suggestions;
        data.cursor_index = cursor_index;
        data.revealed_count = revealed_count;
        data.visited_index = visited_index;
        Self::save(&data)
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
