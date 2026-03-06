use std::collections::HashMap;

use crate::mood::director::{MoodScores, NarrativeRole};
use crate::types::{MoodCategory, MoodIntensity};

/// A cached mood analysis result with full scores, intensity, and narrative role.
#[derive(Debug, Clone)]
pub struct CachedMoodEntry {
    pub mood: MoodCategory,
    pub intensity: MoodIntensity,
    pub scores: MoodScores,
    pub narrative_role: NarrativeRole,
}

/// Ephemeral in-memory cache for pre-calculated mood results.
/// Keyed by (chapter_path, page_index). Auto-clears when chapter changes.
pub struct MoodCache {
    entries: HashMap<(String, u32), CachedMoodEntry>,
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
    pub fn insert(
        &mut self,
        chapter: &str,
        page: u32,
        mood: MoodCategory,
        intensity: MoodIntensity,
        scores: MoodScores,
        narrative_role: NarrativeRole,
    ) {
        if self.current_chapter.as_deref() != Some(chapter) {
            tracing::info!(
                "MoodCache: chapter changed to '{}', clearing {} entries",
                chapter,
                self.entries.len()
            );
            self.entries.clear();
            self.current_chapter = Some(chapter.to_string());
        }
        self.entries.insert(
            (chapter.to_string(), page),
            CachedMoodEntry {
                mood,
                intensity,
                scores,
                narrative_role,
            },
        );
    }

    /// Look up a cached entry for (chapter, page).
    pub fn get(&self, chapter: &str, page: u32) -> Option<&CachedMoodEntry> {
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
