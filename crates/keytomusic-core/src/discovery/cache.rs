use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

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
    /// Suggestions the user already browsed past; later runs skip them.
    #[serde(default)]
    pub seen_ids: Vec<String>,
}

/// Every read-modify-write of a cache file goes through this lock, so a
/// cursor save can't overwrite a background append (and vice versa).
static CACHE_WRITES: Mutex<()> = Mutex::new(());

/// Most recent `seen_ids` kept; older ones may be suggested again.
const MAX_SEEN: usize = 300;

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
        let _guard = CACHE_WRITES.lock().unwrap_or_else(|e| e.into_inner());
        Self::write(data)
    }

    /// Load, change and save a profile's cache under the write lock.
    fn modify(
        profile_id: &str,
        change: impl FnOnce(&mut DiscoveryCacheData),
    ) -> Result<(), String> {
        let _guard = CACHE_WRITES.lock().unwrap_or_else(|e| e.into_inner());
        let mut data =
            Self::load(profile_id).ok_or_else(|| "No discovery cache found".to_string())?;
        change(&mut data);
        Self::write(&data)
    }

    fn write(data: &DiscoveryCacheData) -> Result<(), String> {
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
        let _guard = CACHE_WRITES.lock().unwrap_or_else(|e| e.into_inner());
        let mut data = Self::load(profile_id).unwrap_or_else(|| DiscoveryCacheData {
            profile_id: profile_id.to_string(),
            seed_hash: seed_hash.to_string(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            suggestions: Vec::new(),
            dismissed_ids: Vec::new(),
            cursor_index: 0,
            revealed_count: 0,
            visited_index: -1,
            seen_ids: Vec::new(),
        });

        let existing_ids: HashSet<String> = data
            .suggestions
            .iter()
            .map(|s| s.video_id.clone())
            .collect();
        let dismissed: HashSet<&String> = data.dismissed_ids.iter().collect();

        let fresh: Vec<DiscoverySuggestion> = new_suggestions
            .into_iter()
            .filter(|s| !existing_ids.contains(&s.video_id) && !dismissed.contains(&s.video_id))
            .collect();
        data.suggestions.extend(fresh);

        data.seed_hash = seed_hash.to_string();
        data.generated_at = chrono::Utc::now().to_rfc3339();
        Self::write(&data)
    }

    /// Update only the cursor fields in an existing cache.
    pub fn save_cursor(
        profile_id: &str,
        cursor_index: usize,
        revealed_count: usize,
        visited_index: i32,
    ) -> Result<(), String> {
        Self::modify(profile_id, |data| {
            data.cursor_index = cursor_index;
            data.revealed_count = revealed_count;
            data.visited_index = visited_index;
        })
    }

    /// Replace the suggestion pool and cursor fields in an existing cache.
    pub fn update_pool(
        profile_id: &str,
        suggestions: Vec<DiscoverySuggestion>,
        cursor_index: usize,
        revealed_count: usize,
        visited_index: i32,
    ) -> Result<(), String> {
        Self::modify(profile_id, |data| {
            data.suggestions = suggestions;
            data.cursor_index = cursor_index;
            data.revealed_count = revealed_count;
            data.visited_index = visited_index;
        })
    }

    /// Remember suggestions the user browsed past so later runs skip them.
    pub fn mark_seen(profile_id: &str, video_ids: &[String]) -> Result<(), String> {
        Self::modify(profile_id, |data| {
            for id in video_ids {
                if !data.seen_ids.contains(id) {
                    data.seen_ids.push(id.clone());
                }
            }
            let overflow = data.seen_ids.len().saturating_sub(MAX_SEEN);
            data.seen_ids.drain(..overflow);
        })
    }

    /// Drop a suggestion that was added to the profile (not a dismissal).
    pub fn remove_suggestion(profile_id: &str, video_id: &str) -> Result<(), String> {
        Self::modify(profile_id, |data| {
            data.suggestions.retain(|s| s.video_id != video_id)
        })
    }

    /// Dismiss a suggestion by video ID.
    pub fn dismiss(profile_id: &str, video_id: &str) -> Result<(), String> {
        Self::modify(profile_id, |data| {
            if !data.dismissed_ids.iter().any(|id| id == video_id) {
                data.dismissed_ids.push(video_id.to_string());
            }
            // Also remove from suggestions
            data.suggestions.retain(|s| s.video_id != video_id);
        })
    }

    /// Replace the suggestions after a foreground run, keeping dismissals and
    /// seen ids recorded while it ran.
    pub fn replace_suggestions(
        profile_id: &str,
        suggestions: Vec<DiscoverySuggestion>,
        seed_hash: &str,
    ) -> Result<(), String> {
        let _guard = CACHE_WRITES.lock().unwrap_or_else(|e| e.into_inner());
        let previous = Self::load(profile_id);
        let data = DiscoveryCacheData {
            profile_id: profile_id.to_string(),
            seed_hash: seed_hash.to_string(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            suggestions,
            dismissed_ids: previous
                .as_ref()
                .map(|d| d.dismissed_ids.clone())
                .unwrap_or_default(),
            cursor_index: 0,
            revealed_count: 0,
            visited_index: -1,
            seen_ids: previous.map(|d| d.seen_ids).unwrap_or_default(),
        };
        Self::write(&data)
    }
}
