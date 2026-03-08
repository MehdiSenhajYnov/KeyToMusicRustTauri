use std::collections::HashMap;

use crate::mood::director::{MoodScores, NarrativeRole};
use crate::types::{MoodCategory, MoodIntensity};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CachedMoodSource {
    VisibleWindowAnalyze,
    ChapterPipeline,
}

/// A cached mood analysis result with full scores, intensity, and narrative role.
#[derive(Debug, Clone)]
pub struct CachedMoodEntry {
    pub mood: MoodCategory,
    pub intensity: MoodIntensity,
    pub scores: MoodScores,
    pub narrative_role: NarrativeRole,
    pub source: CachedMoodSource,
    pub finalized: bool,
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
        source: CachedMoodSource,
        finalized: bool,
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
                source,
                finalized,
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

    /// Sorted page indices for the current chapter cache.
    pub fn pages(&self) -> Vec<u32> {
        let Some(current_chapter) = self.current_chapter.as_deref() else {
            return Vec::new();
        };

        let mut pages = self
            .entries
            .keys()
            .filter_map(|(chapter, page)| {
                if chapter == current_chapter {
                    Some(*page)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        pages.sort_unstable();
        pages
    }

    /// Current chapter path, if any.
    pub fn current_chapter(&self) -> Option<&str> {
        self.current_chapter.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mood::director::{MoodScores, NarrativeRole};
    use crate::types::{MoodCategory, MoodIntensity};

    #[test]
    fn pages_returns_sorted_pages_for_current_chapter_only() {
        let mut cache = MoodCache::new();

        cache.insert(
            "/chapter-1",
            5,
            MoodCategory::Tension,
            MoodIntensity::Medium,
            MoodScores::new(),
            NarrativeRole::Continuation,
            CachedMoodSource::ChapterPipeline,
            true,
        );
        cache.insert(
            "/chapter-1",
            2,
            MoodCategory::Epic,
            MoodIntensity::High,
            MoodScores::new(),
            NarrativeRole::Continuation,
            CachedMoodSource::VisibleWindowAnalyze,
            true,
        );

        assert_eq!(cache.pages(), vec![2, 5]);

        cache.insert(
            "/chapter-2",
            1,
            MoodCategory::Comedy,
            MoodIntensity::Low,
            MoodScores::new(),
            NarrativeRole::Continuation,
            CachedMoodSource::ChapterPipeline,
            true,
        );

        assert_eq!(cache.pages(), vec![1]);
    }
}
