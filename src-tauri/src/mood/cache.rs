use std::collections::HashMap;

use crate::types::MoodCategory;

/// Ephemeral in-memory cache for pre-calculated mood results.
/// Keyed by (chapter_path, page_index). Auto-clears when chapter changes.
pub struct MoodCache {
    entries: HashMap<(String, u32), MoodCategory>,
    current_chapter: Option<String>,
}

impl MoodCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            current_chapter: None,
        }
    }

    /// Insert a mood result. Auto-clears cache if chapter changed.
    pub fn insert(&mut self, chapter: &str, page: u32, mood: MoodCategory) {
        if self.current_chapter.as_deref() != Some(chapter) {
            tracing::info!(
                "MoodCache: chapter changed to '{}', clearing {} entries",
                chapter,
                self.entries.len()
            );
            self.entries.clear();
            self.current_chapter = Some(chapter.to_string());
        }
        self.entries.insert((chapter.to_string(), page), mood);
    }

    /// Look up a cached mood for (chapter, page).
    pub fn get(&self, chapter: &str, page: u32) -> Option<&MoodCategory> {
        self.entries.get(&(chapter.to_string(), page))
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.current_chapter = None;
    }

    /// Number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Current chapter path, if any.
    pub fn current_chapter(&self) -> Option<&str> {
        self.current_chapter.as_deref()
    }
}
