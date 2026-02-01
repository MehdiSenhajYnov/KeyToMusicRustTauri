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
}

impl YouTubeCache {
    pub fn new() -> Self {
        let cache_dir = get_app_data_dir().join("cache");
        let index_path = cache_dir.join("cache_index.json");
        Self {
            cache_dir,
            index_path,
            entries: HashMap::new(),
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
        for entry in index.entries {
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

        fs::write(&self.index_path, json)
            .map_err(|e| format!("Failed to write cache index: {}", e))?;

        Ok(())
    }

    /// Get a cached entry if it exists and the file is still present.
    pub fn get(&self, url: &str) -> Option<&CacheEntry> {
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
    /// Best-effort: errors are logged but not propagated.
    pub fn remove_entry_by_video_id(&mut self, video_id: &str) {
        let to_remove: Vec<String> = self
            .entries
            .iter()
            .filter(|(_, entry)| {
                Path::new(&entry.cached_path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s == video_id)
                    .unwrap_or(false)
            })
            .map(|(url, _)| url.clone())
            .collect();

        for url in &to_remove {
            if let Some(entry) = self.entries.get(url) {
                if let Err(e) = fs::remove_file(&entry.cached_path) {
                    tracing::debug!("Could not remove cache file for {}: {}", video_id, e);
                }
            }
            self.entries.remove(url);
        }

        if !to_remove.is_empty() {
            self.save_index().ok();
        }
    }

    /// Verify cache integrity: remove entries whose files are missing.
    pub fn verify_integrity(&mut self) {
        let stale_urls: Vec<String> = self
            .entries
            .iter()
            .filter(|(_, entry)| !Path::new(&entry.cached_path).exists())
            .map(|(url, _)| url.clone())
            .collect();

        for url in stale_urls {
            self.entries.remove(&url);
        }
    }

    /// Remove cache entries (and their files) that are not referenced by any profile.
    /// Scans all saved profiles to find which cached_paths are in use.
    /// Also removes untracked audio files in the cache directory (not in the index).
    pub fn cleanup_unused(&mut self) {
        let used_paths = collect_used_cached_paths();

        // 1. Clean up indexed entries no longer referenced by any profile
        let unused_urls: Vec<String> = self
            .entries
            .iter()
            .filter(|(_, entry)| !used_paths.contains(&entry.cached_path))
            .map(|(url, _)| url.clone())
            .collect();

        for url in &unused_urls {
            if let Some(entry) = self.entries.get(url) {
                let _ = fs::remove_file(&entry.cached_path);
            }
            self.entries.remove(url);
        }

        if !unused_urls.is_empty() {
            self.save_index().ok();
        }

        // 2. Clean up untracked files in the cache directory
        self.cleanup_untracked_files(&used_paths);
    }

    /// Remove audio files in the cache directory that are not tracked by the index
    /// and not referenced by any profile.
    fn cleanup_untracked_files(&self, used_paths: &HashSet<String>) {
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
            let ext = path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            if !audio_extensions.contains(&ext) {
                continue;
            }
            let path_str = path.to_string_lossy().to_string();
            if !indexed_paths.contains(&path_str) && !used_paths.contains(&path_str) {
                let _ = fs::remove_file(&path);
            }
        }
    }
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
                        if let Some(cached_path) = source.get("cachedPath").and_then(|p| p.as_str()) {
                            used.insert(cached_path.to_string());
                        }
                    }
                }
            }
        }
    }

    used
}
