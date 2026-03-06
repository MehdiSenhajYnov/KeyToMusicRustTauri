use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::storage::config::get_app_data_dir;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheEntry {
    pub url: String,
    pub cached_path: String,
    pub title: String,
    pub downloaded_at: String,
    pub file_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheIndex {
    pub entries: Vec<CacheEntry>,
}

pub struct YouTubeCache {
    pub cache_dir: PathBuf,
    pub index_path: PathBuf,
    pub entries: HashMap<String, CacheEntry>, // url -> entry
    /// Secondary index: video_id -> url for O(1) lookup by video ID.
    /// Maintained alongside `entries` to avoid O(n) scans in remove_entry_by_video_id().
    video_id_index: HashMap<String, String>,
    /// Whether the index has been loaded from disk (lazy loading).
    loaded: bool,
}

impl YouTubeCache {
    pub fn new() -> Self {
        let cache_dir = get_app_data_dir().join("cache");
        let index_path = cache_dir.join("cache_index.json");
        Self {
            cache_dir,
            index_path,
            entries: HashMap::new(),
            video_id_index: HashMap::new(),
            loaded: false,
        }
    }

    /// Ensure the cache index is loaded from disk. No-op if already loaded.
    pub fn ensure_loaded(&mut self) {
        if !self.loaded {
            if let Err(e) = self.load_index() {
                tracing::warn!("Failed to load YouTube cache index: {}", e);
            }
            self.loaded = true;
        }
    }

    pub fn load_index(&mut self) -> Result<(), String> {
        if !self.index_path.exists() {
            return Ok(());
        }

        let contents = fs::read_to_string(&self.index_path)
            .map_err(|e| format!("Failed to read cache index: {}", e))?;

        let index: CacheIndex = serde_json::from_str(&contents)
            .map_err(|e| format!("Failed to parse cache index: {}", e))?;

        self.entries.clear();
        self.video_id_index.clear();
        for entry in index.entries {
            if let Some(vid) = video_id_from_path(&entry.cached_path) {
                self.video_id_index.insert(vid, entry.url.clone());
            }
            self.entries.insert(entry.url.clone(), entry);
        }

        Ok(())
    }

    pub fn save_index(&self) -> Result<(), String> {
        fs::create_dir_all(&self.cache_dir)
            .map_err(|e| format!("Failed to create cache dir: {}", e))?;

        let index = CacheIndex {
            entries: self.entries.values().cloned().collect(),
        };

        let json = serde_json::to_string_pretty(&index)
            .map_err(|e| format!("Failed to serialize cache index: {}", e))?;

        let tmp_path = self.index_path.with_extension("json.tmp");
        fs::write(&tmp_path, &json)
            .map_err(|e| format!("Failed to write cache index tmp: {}", e))?;
        // On Windows, rename fails if dest exists; remove first
        let _ = fs::remove_file(&self.index_path);
        fs::rename(&tmp_path, &self.index_path)
            .map_err(|e| format!("Failed to rename cache index: {}", e))?;

        Ok(())
    }

    /// Get a cached entry if it exists and the file is still present.
    pub fn get(&mut self, url: &str) -> Option<&CacheEntry> {
        self.ensure_loaded();
        if let Some(entry) = self.entries.get(url) {
            if Path::new(&entry.cached_path).exists() {
                return Some(entry);
            }
        }
        None
    }

    /// Add a new cache entry.
    pub fn add_entry(
        &mut self,
        url: String,
        cached_path: String,
        title: String,
        file_size: u64,
    ) -> CacheEntry {
        self.ensure_loaded();
        if let Some(vid) = video_id_from_path(&cached_path) {
            self.video_id_index.insert(vid, url.clone());
        }
        let entry = CacheEntry {
            url: url.clone(),
            cached_path,
            title,
            downloaded_at: chrono::Utc::now().to_rfc3339(),
            file_size,
        };
        self.entries.insert(url, entry.clone());
        entry
    }

    /// Remove cache entry (and file) for a specific video ID.
    /// Uses the video_id_index for O(1) lookup instead of scanning all entries.
    /// Best-effort: errors are logged but not propagated.
    pub fn remove_entry_by_video_id(&mut self, video_id: &str) {
        self.ensure_loaded();
        let url = match self.video_id_index.remove(video_id) {
            Some(url) => url,
            None => return,
        };

        if let Some(entry) = self.entries.get(&url) {
            if let Err(e) = fs::remove_file(&entry.cached_path) {
                tracing::debug!("Could not remove cache file for {}: {}", video_id, e);
            }
        }
        self.entries.remove(&url);

        self.save_index().ok();
    }

    /// Verify cache integrity: remove entries whose files are missing.
    pub fn verify_integrity(&mut self) {
        let stale: Vec<(String, String)> = self
            .entries
            .iter()
            .filter(|(_, entry)| !Path::new(&entry.cached_path).exists())
            .map(|(url, entry)| (url.clone(), entry.cached_path.clone()))
            .collect();

        for (url, cached_path) in stale {
            self.entries.remove(&url);
            if let Some(vid) = video_id_from_path(&cached_path) {
                self.video_id_index.remove(&vid);
            }
        }
    }

    /// Remove cache entries (and their files) that are not referenced by any profile
    /// or active discovery suggestion.
    /// Scans all saved profiles and discovery caches to find which entries are in use.
    /// Also removes untracked audio files in the cache directory (not in the index).
    pub fn cleanup_unused(&mut self) {
        let used_paths = collect_used_cached_paths();
        let discovery_vids = collect_discovery_video_ids();

        // 1. Clean up indexed entries no longer referenced by any profile or discovery
        let unused_urls: Vec<String> = self
            .entries
            .iter()
            .filter(|(_, entry)| {
                if used_paths.contains(&entry.cached_path) {
                    return false;
                }
                // Protect files still referenced by discovery suggestions
                if let Some(vid) = video_id_from_path(&entry.cached_path) {
                    if discovery_vids.contains(&vid) {
                        return false;
                    }
                }
                true
            })
            .map(|(url, _)| url.clone())
            .collect();

        for url in &unused_urls {
            if let Some(entry) = self.entries.get(url) {
                if let Some(vid) = video_id_from_path(&entry.cached_path) {
                    self.video_id_index.remove(&vid);
                }
                let _ = fs::remove_file(&entry.cached_path);
            }
            self.entries.remove(url);
        }

        if !unused_urls.is_empty() {
            self.save_index().ok();
        }

        // 2. Clean up untracked files in the cache directory
        self.cleanup_untracked_files(&used_paths, &discovery_vids);
    }

    /// Remove audio files in the cache directory that are not tracked by the index
    /// and not referenced by any profile or discovery suggestion.
    fn cleanup_untracked_files(
        &self,
        used_paths: &HashSet<String>,
        discovery_vids: &HashSet<String>,
    ) {
        let dir_entries = match fs::read_dir(&self.cache_dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        let audio_extensions = ["m4a", "mp3", "opus", "webm", "ogg", "wav", "flac", "aac"];
        let indexed_paths: HashSet<String> = self
            .entries
            .values()
            .map(|e| e.cached_path.clone())
            .collect();

        for entry in dir_entries.flatten() {
            let path = entry.path();
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if !audio_extensions.contains(&ext) {
                continue;
            }
            let path_str = path.to_string_lossy().to_string();
            if indexed_paths.contains(&path_str) || used_paths.contains(&path_str) {
                continue;
            }
            // Protect untracked files whose video_id matches a discovery suggestion
            if let Some(vid) = video_id_from_path(&path_str) {
                if discovery_vids.contains(&vid) {
                    continue;
                }
            }
            let _ = fs::remove_file(&path);
        }
    }
}

/// Extract a video ID from a cached file path.
/// Cache files are stored as `{video_id}.m4a` (or other extensions).
fn video_id_from_path(cached_path: &str) -> Option<String> {
    Path::new(cached_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
}

/// Scan all profile JSON files and collect every cached_path from YouTube sound sources.
fn collect_used_cached_paths() -> HashSet<String> {
    let mut used = HashSet::new();
    let profiles_dir = get_app_data_dir().join("profiles");

    if !profiles_dir.exists() {
        return used;
    }

    let entries = match fs::read_dir(&profiles_dir) {
        Ok(e) => e,
        Err(_) => return used,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        let contents = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let profile: serde_json::Value = match serde_json::from_str(&contents) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if let Some(sounds) = profile.get("sounds").and_then(|s| s.as_array()) {
            for sound in sounds {
                if let Some(source) = sound.get("source") {
                    if source.get("type").and_then(|t| t.as_str()) == Some("youtube") {
                        if let Some(cached_path) = source.get("cachedPath").and_then(|p| p.as_str())
                        {
                            used.insert(cached_path.to_string());
                        }
                    }
                }
            }
        }
    }

    used
}

/// Scan all discovery cache JSON files and collect every video_id from suggestions.
/// These video IDs should be protected from cleanup since the user may preview or add them.
fn collect_discovery_video_ids() -> HashSet<String> {
    let mut ids = HashSet::new();
    let discovery_dir = get_app_data_dir().join("discovery");

    if !discovery_dir.exists() {
        return ids;
    }

    let entries = match fs::read_dir(&discovery_dir) {
        Ok(e) => e,
        Err(_) => return ids,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        let contents = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let cache: serde_json::Value = match serde_json::from_str(&contents) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if let Some(suggestions) = cache.get("suggestions").and_then(|s| s.as_array()) {
            for suggestion in suggestions {
                if let Some(video_id) = suggestion.get("videoId").and_then(|v| v.as_str()) {
                    ids.insert(video_id.to_string());
                }
            }
        }
    }

    ids
}
