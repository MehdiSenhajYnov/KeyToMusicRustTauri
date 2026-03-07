use serde::{Deserialize, Serialize};

use crate::types::{BaseMood, MoodIntensity};

// ─── Types ──────────────────────────────────────────────────────────────────

/// Decimal scores for all 8 base moods (indexed by BaseMood::index()).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoodScores {
    pub scores: [f32; 8],
}

impl MoodScores {
    pub fn new() -> Self {
        Self { scores: [0.0; 8] }
    }

    /// Build scores from a single dominant mood (1.0 for that mood, 0.0 for others).
    pub fn from_single(mood: BaseMood) -> Self {
        let mut scores = [0.0f32; 8];
        scores[mood.index()] = 1.0;
        Self { scores }
    }

    /// Get the score for a mood category.
    pub fn get(&self, mood: BaseMood) -> f32 {
        self.scores[mood.index()]
    }

    /// Set the score for a mood category.
    pub fn set(&mut self, mood: BaseMood, value: f32) {
        self.scores[mood.index()] = value;
    }

    /// Return the mood with the highest score.
    pub fn dominant(&self) -> BaseMood {
        let mut best_idx = 0;
        let mut best_val = self.scores[0];
        for (i, &v) in self.scores.iter().enumerate().skip(1) {
            if v > best_val {
                best_val = v;
                best_idx = i;
            }
        }
        BaseMood::from_index(best_idx)
    }
}

/// Narrative role of a page relative to the story flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NarrativeRole {
    Continuation,
    Escalation,
    DeEscalation,
    Transition,
    Climax,
}

impl NarrativeRole {
    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "continuation" => Some(NarrativeRole::Continuation),
            "escalation" => Some(NarrativeRole::Escalation),
            "de_escalation" | "de-escalation" => Some(NarrativeRole::DeEscalation),
            "transition" => Some(NarrativeRole::Transition),
            "climax" => Some(NarrativeRole::Climax),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            NarrativeRole::Continuation => "continuation",
            NarrativeRole::Escalation => "escalation",
            NarrativeRole::DeEscalation => "de_escalation",
            NarrativeRole::Transition => "transition",
            NarrativeRole::Climax => "climax",
        }
    }
}

/// A single page analysis result from the VLM.
#[derive(Debug, Clone)]
pub struct PageAnalysis {
    pub scores: MoodScores,
    pub intensity: MoodIntensity,
    pub narrative_role: NarrativeRole,
    pub dominant_mood: BaseMood,
}

/// Configuration for the MoodDirector.
#[derive(Debug, Clone)]
pub struct DirectorConfig {
    pub entry_threshold: f32,
    pub exit_threshold: f32,
    pub min_dwell_pages: u32,
    pub window_size: usize,
}

/// Decision output from MoodDirector::process().
#[derive(Debug, Clone)]
pub struct DirectorDecision {
    pub raw_mood: BaseMood,
    pub raw_intensity: MoodIntensity,
    pub committed_mood: BaseMood,
    pub committed_intensity: MoodIntensity,
    pub mood_changed: bool,
    pub intensity_changed: bool,
    pub raw_scores: MoodScores,
    pub narrative_role: NarrativeRole,
    pub window_scores: MoodScores,
    pub dwell_count: u32,
}

// ─── Transition Matrix ──────────────────────────────────────────────────────

/// Transition plausibility from mood A (row) to mood B (column).
/// Higher = more narratively plausible. Diagonal = 1.0 (persistence always plausible).
/// Indices match BaseMood::index() ordering:
///   0=epic, 1=tension, 2=sadness, 3=comedy, 4=romance,
///   5=horror, 6=peaceful, 7=mystery
const TRANSITION_MATRIX: [[f32; 8]; 8] = [
    //  EPI   TEN   SAD   COM   ROM   HOR   PEA   MYS
    [1.0, 0.7, 0.5, 0.2, 0.2, 0.4, 0.3, 0.3], // epic →
    [0.9, 1.0, 0.5, 0.3, 0.3, 0.7, 0.4, 0.8], // tension →
    [0.3, 0.5, 1.0, 0.2, 0.6, 0.3, 0.7, 0.4], // sadness →
    [0.3, 0.3, 0.3, 1.0, 0.7, 0.2, 0.8, 0.3], // comedy →
    [0.2, 0.3, 0.6, 0.7, 1.0, 0.2, 0.8, 0.3], // romance →
    [0.4, 0.8, 0.5, 0.1, 0.1, 1.0, 0.3, 0.7], // horror →
    [0.4, 0.4, 0.6, 0.8, 0.7, 0.3, 1.0, 0.5], // peaceful →
    [0.4, 0.8, 0.4, 0.3, 0.3, 0.7, 0.4, 1.0], // mystery →
];

// ─── MoodDirector ───────────────────────────────────────────────────────────

pub struct MoodDirector {
    config: DirectorConfig,
    window: Vec<PageAnalysis>,
    committed_mood: Option<BaseMood>,
    committed_intensity: Option<MoodIntensity>,
    dwell_counter: u32,
    current_chapter: Option<String>,
}

impl MoodDirector {
    pub fn new(config: DirectorConfig) -> Self {
        Self {
            config,
            window: Vec::new(),
            committed_mood: None,
            committed_intensity: None,
            dwell_counter: 0,
            current_chapter: None,
        }
    }

    /// Main decision algorithm. Feed a page analysis and get a decision.
    pub fn process(&mut self, analysis: PageAnalysis, chapter: Option<&str>) -> DirectorDecision {
        // 1. Auto-reset if chapter changed
        let chapter_str = chapter.map(|s| s.to_string());
        if chapter_str != self.current_chapter {
            tracing::info!(
                "MoodDirector: chapter changed ({:?} → {:?}), resetting",
                self.current_chapter,
                chapter_str
            );
            self.reset();
            self.current_chapter = chapter_str;
        }

        let raw_mood = analysis.dominant_mood;
        let raw_intensity = analysis.intensity;
        let raw_scores = analysis.scores.clone();
        let narrative_role = analysis.narrative_role;

        // 2. Push into sliding window
        self.window.push(analysis);
        if self.window.len() > self.config.window_size {
            self.window.remove(0);
        }

        // Compute smoothed intensity from window
        let smoothed_intensity = self.compute_smoothed_intensity();

        // 3. First page → commit immediately
        if self.committed_mood.is_none() {
            self.committed_mood = Some(raw_mood);
            self.committed_intensity = Some(raw_intensity);
            self.dwell_counter = 1;
            let window_scores = self.compute_weighted_scores();
            tracing::info!(
                "MoodDirector: first page, committing {:?} {:?} immediately",
                raw_mood,
                raw_intensity
            );
            return DirectorDecision {
                raw_mood,
                raw_intensity,
                committed_mood: raw_mood,
                committed_intensity: raw_intensity,
                mood_changed: true,
                intensity_changed: false,
                raw_scores,
                narrative_role,
                window_scores,
                dwell_count: 1,
            };
        }

        let committed = self.committed_mood.unwrap();
        let prev_intensity = self.committed_intensity.unwrap_or(MoodIntensity::Medium);

        // 4. Weighted scores of the window
        let mut window_scores = self.compute_weighted_scores();

        // 5. Apply transition matrix
        for mood in BaseMood::ALL {
            let transition_weight = TRANSITION_MATRIX[committed.index()][mood.index()];
            let score = window_scores.get(mood);
            window_scores.set(mood, score * transition_weight);
        }

        // 6. Best candidate
        let candidate = window_scores.dominant();
        let candidate_score = window_scores.get(candidate);
        let current_score = window_scores.get(committed);

        // 7. Dwell counter
        self.dwell_counter += 1;

        // Effective dwell requirement
        let mut effective_dwell = self.config.min_dwell_pages;

        // 8. Climax override: reduce dwell by 1
        if narrative_role == NarrativeRole::Climax {
            effective_dwell = effective_dwell.saturating_sub(1).max(1);
        }

        // 9. Check for mood change
        let should_change = if raw_mood != committed && raw_scores.get(raw_mood) > 0.85 {
            tracing::info!(
                "MoodDirector: strong override — raw {:?} score {:.2} > 0.85",
                raw_mood,
                raw_scores.get(raw_mood)
            );
            true
        } else if candidate == committed {
            false
        } else {
            current_score < self.config.exit_threshold
                && candidate_score > self.config.entry_threshold
                && self.dwell_counter >= effective_dwell
        };

        if should_change {
            let new_mood = if raw_mood != committed && raw_scores.get(raw_mood) > 0.85 {
                raw_mood
            } else {
                candidate
            };
            tracing::info!(
                "MoodDirector: mood change {:?} → {:?} (candidate_score={:.2}, current_score={:.2}, dwell={})",
                committed,
                new_mood,
                candidate_score,
                current_score,
                self.dwell_counter,
            );
            self.committed_mood = Some(new_mood);
            self.committed_intensity = Some(smoothed_intensity);
            self.dwell_counter = 1;
            DirectorDecision {
                raw_mood,
                raw_intensity,
                committed_mood: new_mood,
                committed_intensity: smoothed_intensity,
                mood_changed: true,
                intensity_changed: smoothed_intensity != prev_intensity,
                raw_scores,
                narrative_role,
                window_scores,
                dwell_count: 1,
            }
        } else {
            // Check for intensity change even when mood doesn't change
            let intensity_changed = smoothed_intensity != prev_intensity;
            if intensity_changed {
                self.committed_intensity = Some(smoothed_intensity);
                tracing::info!(
                    "MoodDirector: intensity change {:?} → {:?} (mood stays {:?})",
                    prev_intensity,
                    smoothed_intensity,
                    committed,
                );
            }

            tracing::debug!(
                "MoodDirector: no mood change — committed={:?}, candidate={:?} (c_score={:.2}, cur_score={:.2}, dwell={}/{})",
                committed,
                candidate,
                candidate_score,
                current_score,
                self.dwell_counter,
                effective_dwell,
            );
            DirectorDecision {
                raw_mood,
                raw_intensity,
                committed_mood: committed,
                committed_intensity: smoothed_intensity,
                mood_changed: false,
                intensity_changed,
                raw_scores,
                narrative_role,
                window_scores,
                dwell_count: self.dwell_counter,
            }
        }
    }

    /// Compute exponentially weighted average of window scores.
    /// Most recent page gets highest weight.
    fn compute_weighted_scores(&self) -> MoodScores {
        if self.window.is_empty() {
            return MoodScores::new();
        }

        let base_weights: &[f32] = &[0.10, 0.15, 0.20, 0.25, 0.30];
        let n = self.window.len();

        let weights = if n <= base_weights.len() {
            &base_weights[base_weights.len() - n..]
        } else {
            base_weights
        };

        let weight_sum: f32 = weights.iter().sum();

        let mut result = MoodScores::new();
        for (i, analysis) in self.window.iter().enumerate() {
            let w = if i < weights.len() {
                weights[i] / weight_sum
            } else {
                0.05 / weight_sum
            };
            for mood in BaseMood::ALL {
                let current = result.get(mood);
                result.set(mood, current + analysis.scores.get(mood) * w);
            }
        }

        result
    }

    /// Compute smoothed intensity from the sliding window (weighted average → nearest level).
    fn compute_smoothed_intensity(&self) -> MoodIntensity {
        if self.window.is_empty() {
            return MoodIntensity::Medium;
        }

        let base_weights: &[f32] = &[0.10, 0.15, 0.20, 0.25, 0.30];
        let n = self.window.len();
        let weights = if n <= base_weights.len() {
            &base_weights[base_weights.len() - n..]
        } else {
            base_weights
        };
        let weight_sum: f32 = weights.iter().sum();

        let avg: f32 = self
            .window
            .iter()
            .enumerate()
            .map(|(i, page)| {
                let w = if i < weights.len() {
                    weights[i] / weight_sum
                } else {
                    0.05 / weight_sum
                };
                page.intensity.as_u8() as f32 * w
            })
            .sum();

        MoodIntensity::from_f32(avg)
    }

    /// Generate a history summary string for the VLM prompt context.
    pub fn mood_history_summary(&self) -> String {
        if self.window.is_empty() {
            return String::new();
        }

        let n = self.window.len();
        self.window
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let offset = i as i32 - n as i32;
                format!(
                    "Page {}: {} ({:.2})",
                    offset,
                    a.dominant_mood.as_str(),
                    a.scores.get(a.dominant_mood)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Get the current committed mood.
    pub fn committed_mood(&self) -> Option<BaseMood> {
        self.committed_mood
    }

    /// Get the current committed intensity.
    pub fn committed_intensity(&self) -> Option<MoodIntensity> {
        self.committed_intensity
    }

    /// Get the current dwell count.
    pub fn dwell_count(&self) -> u32 {
        self.dwell_counter
    }

    /// Get the dominant moods from the window (oldest → newest).
    pub fn window_moods(&self) -> Vec<&str> {
        self.window
            .iter()
            .map(|a| a.dominant_mood.as_str())
            .collect()
    }

    /// Reset all state (new chapter, profile switch, etc.).
    pub fn reset(&mut self) {
        self.window.clear();
        self.committed_mood = None;
        self.committed_intensity = None;
        self.dwell_counter = 0;
        self.current_chapter = None;
    }

    /// Update configuration at runtime.
    pub fn update_config(&mut self, config: DirectorConfig) {
        self.config = config;
        // Trim window if new size is smaller
        while self.window.len() > self.config.window_size {
            self.window.remove(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    mod realtest_suite {
        include!("director_realtest_suite.rs");
    }

    fn make_config() -> DirectorConfig {
        DirectorConfig {
            entry_threshold: 0.55,
            exit_threshold: 0.25,
            min_dwell_pages: 2,
            window_size: 5,
        }
    }

    fn make_analysis(mood: BaseMood, role: NarrativeRole) -> PageAnalysis {
        PageAnalysis {
            scores: MoodScores::from_single(mood),
            intensity: MoodIntensity::Medium,
            narrative_role: role,
            dominant_mood: mood,
        }
    }

    #[test]
    fn first_page_commits_immediately() {
        let mut director = MoodDirector::new(make_config());
        let decision = director.process(
            make_analysis(BaseMood::Tension, NarrativeRole::Continuation),
            None,
        );
        assert!(decision.mood_changed);
        assert_eq!(decision.committed_mood, BaseMood::Tension);
    }

    #[test]
    fn same_mood_does_not_change() {
        let mut director = MoodDirector::new(make_config());
        director.process(
            make_analysis(BaseMood::Tension, NarrativeRole::Continuation),
            None,
        );
        let d2 = director.process(
            make_analysis(BaseMood::Tension, NarrativeRole::Continuation),
            None,
        );
        assert!(!d2.mood_changed);
        assert_eq!(d2.committed_mood, BaseMood::Tension);
    }

    #[test]
    fn single_outlier_does_not_flip() {
        let mut director = MoodDirector::new(make_config());
        for _ in 0..3 {
            director.process(
                make_analysis(BaseMood::Tension, NarrativeRole::Continuation),
                None,
            );
        }
        let mut scores = MoodScores::new();
        scores.set(BaseMood::Comedy, 0.65);
        scores.set(BaseMood::Tension, 0.20);
        scores.set(BaseMood::Peaceful, 0.15);
        let analysis = PageAnalysis {
            scores,
            intensity: MoodIntensity::Medium,
            narrative_role: NarrativeRole::Transition,
            dominant_mood: BaseMood::Comedy,
        };
        let d = director.process(analysis, None);
        assert!(!d.mood_changed);
        assert_eq!(d.committed_mood, BaseMood::Tension);
    }

    #[test]
    fn sustained_change_triggers_transition() {
        let mut director = MoodDirector::new(make_config());
        director.process(
            make_analysis(BaseMood::Tension, NarrativeRole::Continuation),
            None,
        );
        for _ in 0..4 {
            director.process(
                make_analysis(BaseMood::Epic, NarrativeRole::Escalation),
                None,
            );
        }
        let d = director.process(
            make_analysis(BaseMood::Epic, NarrativeRole::Continuation),
            None,
        );
        assert!(d.mood_changed || d.committed_mood == BaseMood::Epic);
    }

    #[test]
    fn chapter_change_resets() {
        let mut director = MoodDirector::new(make_config());
        director.process(
            make_analysis(BaseMood::Tension, NarrativeRole::Continuation),
            Some("/chapter/1"),
        );
        let d = director.process(
            make_analysis(BaseMood::Comedy, NarrativeRole::Continuation),
            Some("/chapter/2"),
        );
        assert!(d.mood_changed);
        assert_eq!(d.committed_mood, BaseMood::Comedy);
    }

    #[test]
    fn strong_override_ignores_dwell() {
        let mut director = MoodDirector::new(make_config());
        director.process(
            make_analysis(BaseMood::Peaceful, NarrativeRole::Continuation),
            None,
        );
        let mut scores = MoodScores::new();
        scores.set(BaseMood::Epic, 0.90);
        scores.set(BaseMood::Peaceful, 0.05);
        let analysis = PageAnalysis {
            scores,
            intensity: MoodIntensity::High,
            narrative_role: NarrativeRole::Climax,
            dominant_mood: BaseMood::Epic,
        };
        let d = director.process(analysis, None);
        assert!(d.mood_changed);
        assert_eq!(d.committed_mood, BaseMood::Epic);
    }

    #[test]
    fn mood_history_summary_format() {
        let mut director = MoodDirector::new(make_config());
        director.process(
            make_analysis(BaseMood::Tension, NarrativeRole::Continuation),
            None,
        );
        director.process(
            make_analysis(BaseMood::Epic, NarrativeRole::Escalation),
            None,
        );
        let summary = director.mood_history_summary();
        assert!(summary.contains("tension"));
        assert!(summary.contains("epic"));
    }

    // ─── Integration test: real VLM + MoodDirector on Blue Lock sequence ────

    /// Ground truth: Blue Lock Tome 1 sequence (31 pages)
    /// Format: (page_num, filename, expected_mood, expected_intensity)
    /// Migrated from old 10-mood format:
    ///   emotional_climax → the actual mood at intensity 3
    ///   chase_action → tension 3
    ///   epic_battle → epic 3
    const GROUND_TRUTH: &[(u32, &str, &str, u8)] = &[
        (6, "BlueLockTome1-006.webp", "tension", 2),
        (7, "BlueLockTome1-007.webp", "tension", 2),
        (8, "BlueLockTome1-008.webp", "tension", 2),
        (9, "BlueLockTome1-009.webp", "tension", 3), // was emotional_climax → tension peak
        (10, "BlueLockTome1-010.webp", "tension", 3), // was emotional_climax
        (11, "BlueLockTome1-011.webp", "tension", 3), // was emotional_climax
        (12, "BlueLockTome1-012.webp", "tension", 2),
        (13, "BlueLockTome1-013.webp", "tension", 3), // was chase_action
        (14, "BlueLockTome1-014.webp", "tension", 2),
        (15, "BlueLockTome1-015.webp", "tension", 2),
        (16, "BlueLockTome1-016.webp", "tension", 2),
        (17, "BlueLockTome1-017.webp", "tension", 2),
        (18, "BlueLockTome1-018.webp", "epic", 3), // was emotional_climax → epic peak
        (19, "BlueLockTome1-019.webp", "epic", 3), // was emotional_climax
        (20, "BlueLockTome1-020.webp", "epic", 3), // was emotional_climax
        (21, "BlueLockTome1-021.webp", "epic", 3), // was emotional_climax
        (22, "BlueLockTome1-022.webp", "sadness", 3), // was emotional_climax → sadness peak
        (23, "BlueLockTome1-023.webp", "sadness", 3), // was emotional_climax
        (24, "BlueLockTome1-024.webp", "sadness", 2),
        (25, "BlueLockTome1-025.webp", "sadness", 2),
        (26, "BlueLockTome1-026.webp", "sadness", 2),
        (27, "BlueLockTome1-027.webp", "sadness", 2),
        (28, "BlueLockTome1-028.webp", "sadness", 2),
        (29, "BlueLockTome1-029.webp", "sadness", 2),
        (30, "BlueLockTome1-030.webp", "sadness", 2),
        (31, "BlueLockTome1-031.webp", "sadness", 2),
        (32, "BlueLockTome1-032.webp", "sadness", 2),
        (33, "BlueLockTome1-033.webp", "sadness", 2),
        (34, "BlueLockTome1-034.webp", "peaceful", 1),
        (35, "BlueLockTome1-035.webp", "mystery", 2),
        (36, "BlueLockTome1-036.webp", "mystery", 2),
    ];

    /// Acceptable alternatives for mood (relaxed matching)
    fn acceptable_alts(filename: &str) -> &'static [&'static str] {
        match filename {
            "BlueLockTome1-006.webp" => &["peaceful", "mystery"],
            "BlueLockTome1-009.webp" => &["epic"], // was tension alt
            "BlueLockTome1-010.webp" => &["epic"],
            "BlueLockTome1-011.webp" => &["mystery", "epic"],
            "BlueLockTome1-013.webp" => &["epic"],
            "BlueLockTome1-014.webp" => &["epic"],
            "BlueLockTome1-015.webp" => &["epic"],
            "BlueLockTome1-017.webp" => &["epic"], // was emotional_climax alt
            "BlueLockTome1-018.webp" => &["tension"],
            "BlueLockTome1-019.webp" => &["tension"],
            "BlueLockTome1-020.webp" => &["tension"],
            "BlueLockTome1-021.webp" => &["sadness", "tension"],
            "BlueLockTome1-022.webp" => &["tension", "epic"],
            "BlueLockTome1-023.webp" => &["tension"],
            "BlueLockTome1-024.webp" => &["tension"], // was emotional_climax alt
            "BlueLockTome1-025.webp" => &["tension"],
            "BlueLockTome1-026.webp" => &["peaceful"],
            "BlueLockTome1-030.webp" => &["mystery", "tension"],
            "BlueLockTome1-032.webp" => &["mystery", "peaceful"],
            "BlueLockTome1-033.webp" => &["tension"], // was emotional_climax alt
            "BlueLockTome1-034.webp" => &["comedy"],
            "BlueLockTome1-035.webp" => &["tension"],
            "BlueLockTome1-036.webp" => &["tension", "peaceful"],
            _ => &[],
        }
    }

    /// Multi-model benchmark: runs real VLM inference on manga images.
    /// Tests the new dimensional mood system (BaseMood + MoodIntensity).
    ///
    /// Run: cargo test --manifest-path src-tauri/Cargo.toml bluelock_sequence -- --ignored --nocapture
    #[tokio::test]
    #[ignore]
    async fn bluelock_sequence() {
        use crate::mood::inference::{self, extract_content, LlamaServer, MOOD_INTENSITY_PROMPT};
        use crate::types::BaseMood;
        use std::path::Path;

        // ANSI colors
        const GREEN: &str = "\x1b[32m";
        const RED: &str = "\x1b[31m";
        const YELLOW: &str = "\x1b[33m";
        const CYAN: &str = "\x1b[36m";
        const DIM: &str = "\x1b[2m";
        const BOLD: &str = "\x1b[1m";
        const RESET: &str = "\x1b[0m";

        inference::lower_current_process_priority();

        let image_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("manga-mood-ai/test-images/bluelock-sequence");
        assert!(image_dir.exists(), "Images not found: {:?}", image_dir);

        // ━━━ Shared prompt & HTTP client ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        let prompt_text = MOOD_INTENSITY_PROMPT;

        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .unwrap();

        // Pre-encode all images
        struct ImageEntry<'a> {
            filename: &'a str,
            expected: &'a str,
            expected_intensity: u8,
            b64: String,
            file_path: std::path::PathBuf,
        }
        let mut images: Vec<ImageEntry> = Vec::new();
        for &(_idx, filename, expected, expected_intensity) in GROUND_TRUTH.iter() {
            let full_path = image_dir.join(filename);
            let bytes = std::fs::read(&full_path)
                .unwrap_or_else(|e| panic!("Failed to read {}: {}", filename, e));
            let b64 = inference::prepare_image(&bytes)
                .unwrap_or_else(|e| panic!("Failed to prepare {}: {}", filename, e));
            images.push(ImageEntry {
                filename,
                expected,
                expected_intensity,
                b64,
                file_path: full_path,
            });
        }

        // ━━━ Per-model results storage ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        /// (filename, expected_mood, detected_mood, strict, relaxed, detected_intensity, expected_intensity)
        type PageResult = (String, String, String, bool, bool, u8, u8);
        struct ModelResult {
            name: String,
            results: Vec<PageResult>,
            correct_strict: u32,
            correct_relaxed: u32,
            intensity_correct: u32,
            errors: u32,
            total: u32,
        }
        let mut all_results: Vec<ModelResult> = Vec::new();

        // ━━━ Helper: run benchmark on a set of images via URL ━━━━━━━━━
        // Returns (results, correct_strict, correct_relaxed, intensity_correct, err_count)
        async fn run_benchmark(
            http: &reqwest::Client,
            server_url: &str,
            images: &[ImageEntry<'_>],
            prompt_text: &str,
        ) -> (Vec<PageResult>, u32, u32, u32, u32) {
            use crate::mood::inference::parse_mood_intensity_response;
            const GREEN: &str = "\x1b[32m";
            const RED: &str = "\x1b[31m";
            const YELLOW: &str = "\x1b[33m";
            const CYAN: &str = "\x1b[36m";
            const DIM: &str = "\x1b[2m";
            const RESET: &str = "\x1b[0m";

            let mut correct_strict = 0u32;
            let mut correct_relaxed = 0u32;
            let mut intensity_correct = 0u32;
            let mut err_count = 0u32;
            let mut results: Vec<PageResult> = Vec::new();

            for img in images {
                let body = serde_json::json!({
                    "model": "test",
                    "messages": [{ "role": "user", "content": [
                        { "type": "image_url", "image_url": { "url": format!("data:image/jpeg;base64,{}", img.b64) } },
                        { "type": "text", "text": prompt_text }
                    ]}],
                    "max_tokens": 8192,
                    "temperature": 0.0
                });

                // Call with retry
                let mut json_response = None;
                for attempt in 1..=3u32 {
                    match http.post(server_url).json(&body).send().await {
                        Ok(resp) if resp.status().is_success() => {
                            if let Ok(j) = resp.json::<serde_json::Value>().await {
                                json_response = Some(j);
                                break;
                            }
                        }
                        _ => {}
                    }
                    if attempt < 3 {
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    }
                }

                let json = match json_response {
                    Some(j) => j,
                    None => {
                        println!(
                            "  {:<30} {:<12} {YELLOW}ERROR{RESET}",
                            img.filename, img.expected
                        );
                        err_count += 1;
                        results.push((
                            img.filename.to_string(),
                            img.expected.to_string(),
                            "ERROR".into(),
                            false,
                            false,
                            0,
                            img.expected_intensity,
                        ));
                        continue;
                    }
                };

                let raw_content = extract_content(&json).unwrap_or_default();
                let (detected_mood, detected_intensity) = match parse_mood_intensity_response(&json)
                {
                    Ok(tag) => (tag.mood.as_str().to_string(), tag.intensity.as_u8()),
                    Err(_) => {
                        // Fallback: find the LAST mood keyword in text
                        let lower = raw_content.to_lowercase();
                        let cleaned = if let Some(pos) = lower.find("</think>") {
                            &lower[pos + 8..]
                        } else {
                            &lower
                        };
                        let mut best: Option<(&BaseMood, usize)> = None;
                        for m in BaseMood::ALL.iter() {
                            if let Some(pos) = cleaned.rfind(m.as_str()) {
                                if best.is_none() || pos > best.unwrap().1 {
                                    best = Some((m, pos));
                                }
                            }
                        }
                        best.map(|(m, _)| (m.as_str().to_string(), 2u8))
                            .unwrap_or_else(|| {
                                err_count += 1;
                                ("???".to_string(), 0)
                            })
                    }
                };

                let strict = detected_mood == *img.expected;
                let alts = acceptable_alts(img.filename);
                let relaxed = strict || alts.contains(&detected_mood.as_str());
                let int_ok = detected_intensity == img.expected_intensity;
                if strict {
                    correct_strict += 1;
                }
                if relaxed {
                    correct_relaxed += 1;
                }
                if int_ok {
                    intensity_correct += 1;
                }
                let (icon, color) = if strict {
                    ("pass", GREEN)
                } else if relaxed {
                    ("~ok~", CYAN)
                } else if detected_mood == "???" {
                    ("ERR", YELLOW)
                } else {
                    ("FAIL", RED)
                };
                let int_icon = if int_ok {
                    format!("{GREEN}{}{RESET}", detected_intensity)
                } else {
                    format!("{YELLOW}{}{RESET}", detected_intensity)
                };
                println!(
                    "  {:<30} {:<12} {color}{:<12} {icon}{RESET}  int: {int_icon} (exp {})",
                    img.filename, img.expected, detected_mood, img.expected_intensity
                );
                // Show raw output snippet for ERR and first 5 FAIL images
                if detected_mood == "???"
                    || (results.iter().filter(|(_, _, _, s, _, _, _)| !s).count() < 5 && !strict)
                {
                    let tail = if let Some(pos) = raw_content.find("</think>") {
                        &raw_content[pos + 8..]
                    } else {
                        &raw_content[raw_content.len().saturating_sub(300)..]
                    };
                    let snippet = tail.trim().replace('\n', " ");
                    let snippet = if snippet.len() > 200 {
                        &snippet[..200]
                    } else {
                        &snippet
                    };
                    println!("    {DIM}raw: {snippet}{RESET}");
                }
                results.push((
                    img.filename.to_string(),
                    img.expected.to_string(),
                    detected_mood,
                    strict,
                    relaxed,
                    detected_intensity,
                    img.expected_intensity,
                ));
            }

            (
                results,
                correct_strict,
                correct_relaxed,
                intensity_correct,
                err_count,
            )
        }

        // ━━━ llama-server backends ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        let models_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("manga-mood-ai/models");

        struct LlamaModelConfig {
            name: &'static str,
            model_path: std::path::PathBuf,
            mmproj_path: std::path::PathBuf,
        }

        let model_choice =
            std::env::var("BLUELOCK_SEQUENCE_MODEL").unwrap_or_else(|_| "qwen3.5-4b".to_string());

        let llama_models: Vec<LlamaModelConfig> = match model_choice.as_str() {
            "qwen3.5-4b" | "qwen35-4b" | "winner" => {
                let dir = models_dir.join("unsloth_Qwen3.5-4B-GGUF");
                vec![LlamaModelConfig {
                    name: "llama-server Qwen3.5 4B",
                    model_path: dir.join("Qwen3.5-4B-Q4_K_M.gguf"),
                    mmproj_path: dir.join("mmproj-F16.gguf"),
                }]
            }
            "qwen3-vl-4b-thinking" | "thinking4b" => {
                let dir = models_dir.join("Qwen3-VL-4B-Thinking");
                vec![LlamaModelConfig {
                    name: "llama-server Qwen3-VL-4B-Thinking",
                    model_path: dir.join("Qwen3VL-4B-Thinking-Q4_K_M.gguf"),
                    mmproj_path: dir.join("mmproj-Qwen3VL-4B-Thinking-F16.gguf"),
                }]
            }
            "qwen3-vl-2b" | "2b" => {
                let dir = models_dir.join("Qwen_Qwen3-VL-2B-Instruct-GGUF");
                vec![LlamaModelConfig {
                    name: "llama-server Qwen3-VL 2B",
                    model_path: dir.join("Qwen3VL-2B-Instruct-Q4_K_M.gguf"),
                    mmproj_path: dir.join("mmproj-Qwen3VL-2B-Instruct-F16.gguf"),
                }]
            }
            other => {
                panic!(
                    "Unsupported BLUELOCK_SEQUENCE_MODEL='{}'. Expected one of: qwen3.5-4b, qwen3-vl-4b-thinking, qwen3-vl-2b",
                    other
                );
            }
        };

        for model_cfg in &llama_models {
            assert!(
                model_cfg.model_path.exists(),
                "Model not found for {}: {:?}",
                model_cfg.name,
                model_cfg.model_path
            );
            assert!(
                model_cfg.mmproj_path.exists(),
                "mmproj not found for {}: {:?}",
                model_cfg.name,
                model_cfg.mmproj_path
            );
        }

        println!(
            "\n  {BOLD}Manga Mood Benchmark — {} images, {} llama-server model(s) [BLUELOCK_SEQUENCE_MODEL={}]{RESET}\n",
            images.len(),
            llama_models.len(),
            model_choice
        );

        for model_cfg in &llama_models {
            println!("  {CYAN}{BOLD}━━━ {} ━━━{RESET}", model_cfg.name);
            println!("  {DIM}Starting llama-server...{RESET}");

            let mut server = match LlamaServer::start(
                model_cfg.model_path.to_str().unwrap(),
                model_cfg.mmproj_path.to_str().unwrap(),
            )
            .await
            {
                Ok(s) => s,
                Err(e) => {
                    println!("  {RED}Failed to start: {e}{RESET}\n");
                    continue;
                }
            };
            println!("  {DIM}Ready on port {}{RESET}\n", server.port);

            let server_url = format!("http://127.0.0.1:{}/v1/chat/completions", server.port);

            // ━━━ Pass 1: VLM dimensional (cached on disk) ━━━━━━━━━━━━━━━
            let cache_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("manga-mood-ai/results");
            let _ = std::fs::create_dir_all(&cache_dir);
            let cache_name = model_cfg.name.replace(' ', "_").to_lowercase();
            // New cache format: { "filename": "mood:intensity", ... }
            let pass1_cache_path =
                cache_dir.join(format!("pass1_dim_bluelock_{}.json", cache_name));

            let cached_detections: Option<std::collections::HashMap<String, String>> =
                std::fs::read_to_string(&pass1_cache_path)
                    .ok()
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .filter(|map: &std::collections::HashMap<String, String>| {
                        map.len() >= images.len()
                    });

            let (results, cs, cr, int_ok, errs) = if let Some(ref detected_map) = cached_detections
            {
                // Reconstruct from cache — no VLM inference needed
                println!(
                    "  {DIM}(Pass 1 loaded from cache: {}){RESET}\n",
                    pass1_cache_path.display()
                );
                let mut cs = 0u32;
                let mut cr = 0u32;
                let mut int_ok = 0u32;
                let mut errs = 0u32;
                let mut results: Vec<PageResult> = Vec::new();
                for img in images.iter() {
                    let cached_val = detected_map
                        .get(img.filename)
                        .cloned()
                        .unwrap_or_else(|| "???:0".to_string());
                    // Parse "mood:intensity" format
                    let parts: Vec<&str> = cached_val.split(':').collect();
                    let detected = parts[0].to_string();
                    let det_int: u8 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(2);

                    if detected == "ERROR" || detected == "???" {
                        errs += 1;
                    }
                    let strict = detected == img.expected;
                    let alts = acceptable_alts(img.filename);
                    let relaxed = strict || alts.contains(&detected.as_str());
                    let i_ok = det_int == img.expected_intensity;
                    if strict {
                        cs += 1;
                    }
                    if relaxed {
                        cr += 1;
                    }
                    if i_ok {
                        int_ok += 1;
                    }
                    let (icon, color) = if strict {
                        ("pass", GREEN)
                    } else if relaxed {
                        ("~ok~", CYAN)
                    } else if detected == "???" {
                        ("ERR", YELLOW)
                    } else {
                        ("FAIL", RED)
                    };
                    let int_icon = if i_ok {
                        format!("{GREEN}{}{RESET}", det_int)
                    } else {
                        format!("{YELLOW}{}{RESET}", det_int)
                    };
                    println!(
                        "  {:<30} {:<12} {color}{:<12} {icon}{RESET}  int: {int_icon} (exp {})",
                        img.filename, img.expected, detected, img.expected_intensity
                    );
                    results.push((
                        img.filename.to_string(),
                        img.expected.to_string(),
                        detected,
                        strict,
                        relaxed,
                        det_int,
                        img.expected_intensity,
                    ));
                }
                (results, cs, cr, int_ok, errs)
            } else {
                // Run VLM benchmark and save cache
                let r = run_benchmark(&http, &server_url, &images, prompt_text).await;
                // Save as "mood:intensity" format
                let det_map: std::collections::HashMap<String, String> = r
                    .0
                    .iter()
                    .map(|(f, _, d, _, _, det_int, _)| (f.clone(), format!("{}:{}", d, det_int)))
                    .collect();
                let _ = std::fs::write(
                    &pass1_cache_path,
                    serde_json::to_string_pretty(&det_map).unwrap(),
                );
                println!(
                    "  {DIM}(Pass 1 saved to cache: {}){RESET}",
                    pass1_cache_path.display()
                );
                r
            };

            let processed = images.len() as u32 - errs;
            let pct_s = if processed > 0 {
                cs as f64 / processed as f64 * 100.0
            } else {
                0.0
            };
            let pct_r = if processed > 0 {
                cr as f64 / processed as f64 * 100.0
            } else {
                0.0
            };
            let pct_i = if processed > 0 {
                int_ok as f64 / processed as f64 * 100.0
            } else {
                0.0
            };
            let sc_s = if pct_s >= 60.0 {
                GREEN
            } else if pct_s >= 40.0 {
                YELLOW
            } else {
                RED
            };
            let sc_r = if pct_r >= 60.0 {
                GREEN
            } else if pct_r >= 40.0 {
                YELLOW
            } else {
                RED
            };
            let sc_i = if pct_i >= 50.0 {
                GREEN
            } else if pct_i >= 30.0 {
                YELLOW
            } else {
                RED
            };
            println!("\n  {BOLD}=> Mood strict: {sc_s}{cs}/{processed} ({pct_s:.0}%){RESET}  {BOLD}Relaxed: {sc_r}{cr}/{processed} ({pct_r:.0}%){RESET}  {BOLD}Intensity: {sc_i}{int_ok}/{processed} ({pct_i:.0}%){RESET}\n");

            all_results.push(ModelResult {
                name: model_cfg.name.to_string(),
                results,
                correct_strict: cs,
                correct_relaxed: cr,
                intensity_correct: int_ok,
                errors: errs,
                total: images.len() as u32,
            });

            // ━━━ Pass V12: Sliding window 3-image + majority vote ━━━━━━━━

            // Health check: restart server if it crashed after Pass 1
            if !server.is_running() {
                println!("  {YELLOW}llama-server crashed after Pass 1, restarting...{RESET}");
                server.stop();
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                match LlamaServer::start(
                    model_cfg.model_path.to_str().unwrap(),
                    model_cfg.mmproj_path.to_str().unwrap(),
                )
                .await
                {
                    Ok(s) => {
                        println!("  {DIM}Restarted on port {}{RESET}\n", s.port);
                        server = s;
                    }
                    Err(e) => {
                        println!("  {RED}Failed to restart llama-server: {e}{RESET}\n");
                        drop(server);
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        continue;
                    }
                }
            }

            {
                use crate::mood::inference::{extract_content, parse_mood_intensity_response};

                let v12_name = format!("{} (pass V12: sliding window 3-img)", model_cfg.name);
                println!("  {CYAN}{BOLD}━━━ {} ━━━{RESET}", v12_name);
                println!("  {DIM}Running V12: sliding window of 3 images + majority vote (always live)...{RESET}\n");

                let server_url = format!("http://127.0.0.1:{}/v1/chat/completions", server.port);

                let v12_prompt = "\
These are 3 consecutive manga pages from the same chapter.\n\
They are shown in reading order: page LEFT, page CENTER, page RIGHT.\n\
\n\
Considering the flow across all 3 pages, what is the overall mood of this sequence for soundtrack purposes?\n\
\n\
Classify as ONE mood that best represents the group:\n\
- epic: climactic moments, declarations of resolve, characters unleashing power, pivotal turning points, peak dramatic intensity\n\
- tension: buildup, uncertainty, standoffs, threats without resolution, ominous atmosphere, characters evaluating the situation\n\
- sadness: loss, grief, crying, emotional pain, defeat\n\
- comedy: comic relief, gag reactions, slapstick, funny situations (NOT just smiling or friendly characters)\n\
- romance: love, intimacy, tender moments between characters\n\
- horror: fear, gore, monsters, nightmarish imagery\n\
- peaceful: calm scenes, daily life, quiet contemplation, friendly conversations\n\
- mystery: secrets revealed, scheming, hidden motives, ominous foreshadowing\n\
\n\
Rate the intensity from 1 (low) to 3 (high).\n\
\n\
Reply format: mood intensity\n\
Example: tension 2";

                // Phase 1: Run all windows (one per center page)
                // window_results[center_idx] = (left_idx, center_idx, right_idx, mood, intensity)
                let mut window_results: Vec<(usize, usize, usize, String, u8)> = Vec::new();
                let mut window_durations: Vec<f64> = Vec::new();
                let mut window_errors = 0u32;

                println!("  {DIM}Phase 1: {}{RESET} windows\n", images.len());

                for center in 0..images.len() {
                    let left = if center > 0 { center - 1 } else { center };
                    let right = if center < images.len() - 1 {
                        center + 1
                    } else {
                        center
                    };

                    let body = serde_json::json!({
                        "model": "test",
                        "messages": [{ "role": "user", "content": [
                            { "type": "image_url", "image_url": { "url": format!("data:image/jpeg;base64,{}", images[left].b64) } },
                            { "type": "image_url", "image_url": { "url": format!("data:image/jpeg;base64,{}", images[center].b64) } },
                            { "type": "image_url", "image_url": { "url": format!("data:image/jpeg;base64,{}", images[right].b64) } },
                            { "type": "text", "text": v12_prompt }
                        ]}],
                        "max_tokens": 8192,
                        "temperature": 0.0
                    });

                    let win_start = std::time::Instant::now();
                    let mut json_response = None;
                    for attempt in 1..=3u32 {
                        match http.post(&server_url).json(&body).send().await {
                            Ok(resp) if resp.status().is_success() => {
                                if let Ok(j) = resp.json::<serde_json::Value>().await {
                                    json_response = Some(j);
                                    break;
                                }
                            }
                            _ => {}
                        }
                        if attempt < 3 {
                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        }
                    }
                    let win_elapsed = win_start.elapsed().as_secs_f64();
                    window_durations.push(win_elapsed);

                    let (mood, intensity) = match json_response {
                        None => {
                            println!(
                                "  Window [{},{},{}] → {YELLOW}ERROR{RESET}  | {:.1}s",
                                GROUND_TRUTH[left].0,
                                GROUND_TRUTH[center].0,
                                GROUND_TRUTH[right].0,
                                win_elapsed
                            );
                            window_errors += 1;
                            window_results.push((left, center, right, "???".to_string(), 0));
                            continue;
                        }
                        Some(json) => {
                            let raw_content = extract_content(&json).unwrap_or_default();
                            match parse_mood_intensity_response(&json) {
                                Ok(tag) => (tag.mood.as_str().to_string(), tag.intensity.as_u8()),
                                Err(_) => {
                                    let lower = raw_content.to_lowercase();
                                    let cleaned = if let Some(pos) = lower.find("</think>") {
                                        &lower[pos + 8..]
                                    } else {
                                        &lower
                                    };
                                    let mut best: Option<(&BaseMood, usize)> = None;
                                    for m in BaseMood::ALL.iter() {
                                        if let Some(pos) = cleaned.rfind(m.as_str()) {
                                            if best.is_none() || pos > best.unwrap().1 {
                                                best = Some((m, pos));
                                            }
                                        }
                                    }
                                    best.map(|(m, _)| (m.as_str().to_string(), 2u8))
                                        .unwrap_or_else(|| {
                                            window_errors += 1;
                                            ("???".to_string(), 0)
                                        })
                                }
                            }
                        }
                    };

                    println!(
                        "  Window [{},{},{}] → {:<12} {}  | {:.1}s",
                        GROUND_TRUTH[left].0,
                        GROUND_TRUTH[center].0,
                        GROUND_TRUTH[right].0,
                        mood,
                        intensity,
                        win_elapsed
                    );
                    window_results.push((left, center, right, mood, intensity));
                }

                // Window latency summary
                {
                    let total: f64 = window_durations.iter().sum();
                    let avg = total / window_durations.len() as f64;
                    let min = window_durations
                        .iter()
                        .cloned()
                        .fold(f64::INFINITY, f64::min);
                    let max = window_durations
                        .iter()
                        .cloned()
                        .fold(f64::NEG_INFINITY, f64::max);
                    println!("\n  Windows: {} inferences, avg {:.1}s (min {:.1}s, max {:.1}s, total {:.0}s)\n",
                        window_durations.len(), avg, min, max, total);
                }

                // Phase 2: Majority vote per page
                println!("  {DIM}Phase 2: Majority vote per page{RESET}\n");

                let mut v12_results: Vec<PageResult> = Vec::new();
                let mut v12_cs = 0u32;
                let mut v12_cr = 0u32;
                let mut v12_int = 0u32;
                let mut v12_errs = 0u32;
                let mut vote_unanimous = 0u32;
                let mut vote_majority = 0u32;
                let mut vote_split = 0u32;

                for (i, img) in images.iter().enumerate() {
                    // Collect all votes for this page (it appears as left, center, or right)
                    let votes: Vec<(&str, u8)> = window_results
                        .iter()
                        .filter(|(l, c, r, _, _)| *l == i || *c == i || *r == i)
                        .filter(|(_, _, _, m, _)| m != "???")
                        .map(|(_, _, _, mood, intensity)| (mood.as_str(), *intensity))
                        .collect();

                    if votes.is_empty() {
                        v12_errs += 1;
                        println!(
                            "  {:<30} {:<12} {YELLOW}???          ERR{RESET}  votes: []",
                            img.filename, img.expected
                        );
                        v12_results.push((
                            img.filename.to_string(),
                            img.expected.to_string(),
                            "???".into(),
                            false,
                            false,
                            0,
                            img.expected_intensity,
                        ));
                        continue;
                    }

                    // Count mood occurrences
                    let mut mood_counts: std::collections::HashMap<&str, u32> =
                        std::collections::HashMap::new();
                    let mut intensity_sum: std::collections::HashMap<&str, (u32, u32)> =
                        std::collections::HashMap::new(); // (sum, count)
                    for &(mood, int) in &votes {
                        *mood_counts.entry(mood).or_insert(0) += 1;
                        let entry = intensity_sum.entry(mood).or_insert((0, 0));
                        entry.0 += int as u32;
                        entry.1 += 1;
                    }

                    // Find majority mood
                    let (final_mood, vote_type) = {
                        let mut sorted: Vec<(&&str, &u32)> = mood_counts.iter().collect();
                        sorted.sort_by(|a, b| b.1.cmp(a.1));

                        if sorted.len() == 1 || *sorted[0].1 > *sorted[1].1 {
                            // Clear winner (unanimous or majority)
                            let vt = if mood_counts.len() == 1 && votes.len() >= 3 {
                                vote_unanimous += 1;
                                "unanimous"
                            } else if votes.len() >= 3 {
                                vote_majority += 1;
                                "2-1 majority"
                            } else {
                                "single"
                            };
                            (sorted[0].0.to_string(), vt)
                        } else {
                            // Tie or 3-way split: use center window vote
                            vote_split += 1;
                            let center_mood = window_results
                                .iter()
                                .find(|(_, c, _, _, _)| *c == i)
                                .map(|(_, _, _, m, _)| m.as_str())
                                .unwrap_or("???");
                            (center_mood.to_string(), "3-way split")
                        }
                    };

                    // Average intensity for the winning mood
                    let final_int = intensity_sum
                        .get(final_mood.as_str())
                        .map(|(sum, count)| {
                            let avg = (*sum as f64 / *count as f64).round() as u8;
                            avg.max(1).min(3)
                        })
                        .unwrap_or(2);

                    let strict = final_mood == img.expected;
                    let alts = acceptable_alts(img.filename);
                    let relaxed = strict || alts.contains(&final_mood.as_str());
                    let i_ok = final_int == img.expected_intensity;
                    if strict {
                        v12_cs += 1;
                    }
                    if relaxed {
                        v12_cr += 1;
                    }
                    if i_ok {
                        v12_int += 1;
                    }
                    let (icon, color) = if strict {
                        ("pass", GREEN)
                    } else if relaxed {
                        ("~ok~", CYAN)
                    } else if final_mood == "???" {
                        ("ERR", YELLOW)
                    } else {
                        ("FAIL", RED)
                    };
                    let int_icon = if i_ok {
                        format!("{GREEN}{}{RESET}", final_int)
                    } else {
                        format!("{YELLOW}{}{RESET}", final_int)
                    };
                    let vote_list: Vec<&str> = votes.iter().map(|(m, _)| *m).collect();
                    println!("  {:<30} {:<12} {color}{:<12} {icon}{RESET}  int: {int_icon} (exp {})  votes: {:?} ({vote_type})",
                        img.filename, img.expected, final_mood, img.expected_intensity, vote_list);
                    v12_results.push((
                        img.filename.to_string(),
                        img.expected.to_string(),
                        final_mood,
                        strict,
                        relaxed,
                        final_int,
                        img.expected_intensity,
                    ));
                }

                // Vote stats
                println!("\n  Vote stats: {vote_unanimous} unanimous, {vote_majority} majority (2-1), {vote_split} three-way splits");

                let v12_total = images.len() as u32;
                let v12_processed = v12_total - v12_errs;
                let v12_pct_s = if v12_processed > 0 {
                    v12_cs as f64 / v12_processed as f64 * 100.0
                } else {
                    0.0
                };
                let v12_pct_r = if v12_processed > 0 {
                    v12_cr as f64 / v12_processed as f64 * 100.0
                } else {
                    0.0
                };
                let v12_pct_i = if v12_processed > 0 {
                    v12_int as f64 / v12_processed as f64 * 100.0
                } else {
                    0.0
                };
                let v12_sc_s = if v12_pct_s >= 60.0 {
                    GREEN
                } else if v12_pct_s >= 40.0 {
                    YELLOW
                } else {
                    RED
                };
                let v12_sc_r = if v12_pct_r >= 60.0 {
                    GREEN
                } else if v12_pct_r >= 40.0 {
                    YELLOW
                } else {
                    RED
                };
                let v12_sc_i = if v12_pct_i >= 50.0 {
                    GREEN
                } else if v12_pct_i >= 30.0 {
                    YELLOW
                } else {
                    RED
                };

                // Delta from Pass 1
                let p1_cs = all_results[0].correct_strict;
                let p1_cr = all_results[0].correct_relaxed;
                let delta_s = v12_cs as i32 - p1_cs as i32;
                let delta_r = v12_cr as i32 - p1_cr as i32;
                let delta_s_str = if delta_s > 0 {
                    format!(" {GREEN}+{delta_s}{RESET}")
                } else if delta_s < 0 {
                    format!(" {RED}{delta_s}{RESET}")
                } else {
                    String::new()
                };
                let delta_r_str = if delta_r > 0 {
                    format!(" {GREEN}+{delta_r}{RESET}")
                } else if delta_r < 0 {
                    format!(" {RED}{delta_r}{RESET}")
                } else {
                    String::new()
                };

                println!(
                    "\n  {BOLD}=> Mood strict: {v12_sc_s}{v12_cs}/{v12_processed} ({v12_pct_s:.0}%){RESET}{delta_s_str}  {BOLD}Relaxed: {v12_sc_r}{v12_cr}/{v12_processed} ({v12_pct_r:.0}%){RESET}{delta_r_str}  {BOLD}Intensity: {v12_sc_i}{v12_int}/{v12_processed} ({v12_pct_i:.0}%){RESET}  {DIM}(delta from Pass 1){RESET}\n"
                );

                all_results.push(ModelResult {
                    name: v12_name,
                    results: v12_results,
                    correct_strict: v12_cs,
                    correct_relaxed: v12_cr,
                    intensity_correct: v12_int,
                    errors: v12_errs,
                    total: v12_total,
                });
            }

            // ━━━ Pass V12-think: Sliding window 3-image + native thinking ━━━

            // Health check: restart server if it crashed after V12
            if !server.is_running() {
                println!("  {YELLOW}llama-server crashed after V12, restarting...{RESET}");
                server.stop();
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                match LlamaServer::start(
                    model_cfg.model_path.to_str().unwrap(),
                    model_cfg.mmproj_path.to_str().unwrap(),
                )
                .await
                {
                    Ok(s) => {
                        println!("  {DIM}Restarted on port {}{RESET}\n", s.port);
                        server = s;
                    }
                    Err(e) => {
                        println!("  {RED}Failed to restart llama-server: {e}{RESET}\n");
                        drop(server);
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        continue;
                    }
                }
            }

            {
                use crate::mood::inference::{extract_content, parse_mood_intensity_response};

                let v12t_name = format!(
                    "{} (pass V12-think: 3-img + native thinking)",
                    model_cfg.name
                );
                println!("  {CYAN}{BOLD}━━━ {} ━━━{RESET}", v12t_name);
                println!("  {DIM}Running V12-think: sliding window 3-img + native thinking (temp 0.6)...{RESET}\n");

                let server_url = format!("http://127.0.0.1:{}/v1/chat/completions", server.port);

                let v12t_prompt = "\
These are 3 consecutive manga pages from the same chapter.\n\
They are shown in reading order: page LEFT, page CENTER, page RIGHT.\n\
\n\
Considering the flow across all 3 pages, what is the overall mood of this sequence for soundtrack purposes?\n\
\n\
Classify as ONE mood that best represents the group:\n\
Moods: epic, tension, sadness, comedy, romance, horror, peaceful, mystery\n\
\n\
Rate the intensity from 1 (low) to 3 (high).\n\
\n\
Reply format: mood intensity\n\
Example: tension 2";

                // Phase 1: Run all windows with thinking enabled
                let mut window_results: Vec<(usize, usize, usize, String, u8)> = Vec::new();
                let mut window_durations: Vec<f64> = Vec::new();
                let mut window_errors = 0u32;

                println!(
                    "  {DIM}Phase 1: {}{RESET} windows (native thinking mode)\n",
                    images.len()
                );

                for center in 0..images.len() {
                    let left = if center > 0 { center - 1 } else { center };
                    let right = if center < images.len() - 1 {
                        center + 1
                    } else {
                        center
                    };

                    let body = serde_json::json!({
                        "model": "test",
                        "messages": [
                            { "role": "system", "content": "/think" },
                            { "role": "user", "content": [
                                { "type": "image_url", "image_url": { "url": format!("data:image/jpeg;base64,{}", images[left].b64) } },
                                { "type": "image_url", "image_url": { "url": format!("data:image/jpeg;base64,{}", images[center].b64) } },
                                { "type": "image_url", "image_url": { "url": format!("data:image/jpeg;base64,{}", images[right].b64) } },
                                { "type": "text", "text": v12t_prompt }
                            ]}
                        ],
                        "max_tokens": 8192,
                        "temperature": 0.6
                    });

                    let win_start = std::time::Instant::now();
                    let mut json_response = None;
                    for attempt in 1..=3u32 {
                        match http.post(&server_url).json(&body).send().await {
                            Ok(resp) if resp.status().is_success() => {
                                if let Ok(j) = resp.json::<serde_json::Value>().await {
                                    json_response = Some(j);
                                    break;
                                }
                            }
                            _ => {}
                        }
                        if attempt < 3 {
                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        }
                    }
                    let win_elapsed = win_start.elapsed().as_secs_f64();
                    window_durations.push(win_elapsed);

                    let (mood, intensity) = match json_response {
                        None => {
                            println!(
                                "  Window [{},{},{}] → {YELLOW}ERROR{RESET}  | {:.1}s",
                                GROUND_TRUTH[left].0,
                                GROUND_TRUTH[center].0,
                                GROUND_TRUTH[right].0,
                                win_elapsed
                            );
                            window_errors += 1;
                            window_results.push((left, center, right, "???".to_string(), 0));
                            continue;
                        }
                        Some(json) => {
                            let raw_content = extract_content(&json).unwrap_or_default();
                            match parse_mood_intensity_response(&json) {
                                Ok(tag) => (tag.mood.as_str().to_string(), tag.intensity.as_u8()),
                                Err(_) => {
                                    let lower = raw_content.to_lowercase();
                                    let cleaned = if let Some(pos) = lower.find("</think>") {
                                        &lower[pos + 8..]
                                    } else {
                                        &lower
                                    };
                                    let mut best: Option<(&BaseMood, usize)> = None;
                                    for m in BaseMood::ALL.iter() {
                                        if let Some(pos) = cleaned.rfind(m.as_str()) {
                                            if best.is_none() || pos > best.unwrap().1 {
                                                best = Some((m, pos));
                                            }
                                        }
                                    }
                                    best.map(|(m, _)| (m.as_str().to_string(), 2u8))
                                        .unwrap_or_else(|| {
                                            window_errors += 1;
                                            ("???".to_string(), 0)
                                        })
                                }
                            }
                        }
                    };

                    println!(
                        "  Window [{},{},{}] → {:<12} {}  | {:.1}s",
                        GROUND_TRUTH[left].0,
                        GROUND_TRUTH[center].0,
                        GROUND_TRUTH[right].0,
                        mood,
                        intensity,
                        win_elapsed
                    );
                    window_results.push((left, center, right, mood, intensity));
                }

                // Window latency summary
                {
                    let total: f64 = window_durations.iter().sum();
                    let avg = total / window_durations.len() as f64;
                    let min = window_durations
                        .iter()
                        .cloned()
                        .fold(f64::INFINITY, f64::min);
                    let max = window_durations
                        .iter()
                        .cloned()
                        .fold(f64::NEG_INFINITY, f64::max);
                    println!("\n  Windows: {} inferences, avg {:.1}s (min {:.1}s, max {:.1}s, total {:.0}s)\n",
                        window_durations.len(), avg, min, max, total);
                }

                // Phase 2: Majority vote per page
                println!("  {DIM}Phase 2: Majority vote per page{RESET}\n");

                let mut v12t_results: Vec<PageResult> = Vec::new();
                let mut v12t_cs = 0u32;
                let mut v12t_cr = 0u32;
                let mut v12t_int = 0u32;
                let mut v12t_errs = 0u32;
                let mut vote_unanimous = 0u32;
                let mut vote_majority = 0u32;
                let mut vote_split = 0u32;

                for (i, img) in images.iter().enumerate() {
                    let votes: Vec<(&str, u8)> = window_results
                        .iter()
                        .filter(|(l, c, r, _, _)| *l == i || *c == i || *r == i)
                        .filter(|(_, _, _, m, _)| m != "???")
                        .map(|(_, _, _, mood, intensity)| (mood.as_str(), *intensity))
                        .collect();

                    if votes.is_empty() {
                        v12t_errs += 1;
                        println!(
                            "  {:<30} {:<12} {YELLOW}???          ERR{RESET}  votes: []",
                            img.filename, img.expected
                        );
                        v12t_results.push((
                            img.filename.to_string(),
                            img.expected.to_string(),
                            "???".into(),
                            false,
                            false,
                            0,
                            img.expected_intensity,
                        ));
                        continue;
                    }

                    let mut mood_counts: std::collections::HashMap<&str, u32> =
                        std::collections::HashMap::new();
                    let mut intensity_sum: std::collections::HashMap<&str, (u32, u32)> =
                        std::collections::HashMap::new();
                    for &(mood, int) in &votes {
                        *mood_counts.entry(mood).or_insert(0) += 1;
                        let entry = intensity_sum.entry(mood).or_insert((0, 0));
                        entry.0 += int as u32;
                        entry.1 += 1;
                    }

                    let (final_mood, vote_type) = {
                        let mut sorted: Vec<(&&str, &u32)> = mood_counts.iter().collect();
                        sorted.sort_by(|a, b| b.1.cmp(a.1));

                        if sorted.len() == 1 || *sorted[0].1 > *sorted[1].1 {
                            let vt = if mood_counts.len() == 1 && votes.len() >= 3 {
                                vote_unanimous += 1;
                                "unanimous"
                            } else if votes.len() >= 3 {
                                vote_majority += 1;
                                "2-1 majority"
                            } else {
                                "single"
                            };
                            (sorted[0].0.to_string(), vt)
                        } else {
                            vote_split += 1;
                            let center_mood = window_results
                                .iter()
                                .find(|(_, c, _, _, _)| *c == i)
                                .map(|(_, _, _, m, _)| m.as_str())
                                .unwrap_or("???");
                            (center_mood.to_string(), "3-way split")
                        }
                    };

                    let final_int = intensity_sum
                        .get(final_mood.as_str())
                        .map(|(sum, count)| {
                            let avg = (*sum as f64 / *count as f64).round() as u8;
                            avg.max(1).min(3)
                        })
                        .unwrap_or(2);

                    let strict = final_mood == img.expected;
                    let alts = acceptable_alts(img.filename);
                    let relaxed = strict || alts.contains(&final_mood.as_str());
                    let i_ok = final_int == img.expected_intensity;
                    if strict {
                        v12t_cs += 1;
                    }
                    if relaxed {
                        v12t_cr += 1;
                    }
                    if i_ok {
                        v12t_int += 1;
                    }
                    let (icon, color) = if strict {
                        ("pass", GREEN)
                    } else if relaxed {
                        ("~ok~", CYAN)
                    } else if final_mood == "???" {
                        ("ERR", YELLOW)
                    } else {
                        ("FAIL", RED)
                    };
                    let int_icon = if i_ok {
                        format!("{GREEN}{}{RESET}", final_int)
                    } else {
                        format!("{YELLOW}{}{RESET}", final_int)
                    };
                    let vote_list: Vec<&str> = votes.iter().map(|(m, _)| *m).collect();
                    println!("  {:<30} {:<12} {color}{:<12} {icon}{RESET}  int: {int_icon} (exp {})  votes: {:?} ({vote_type})",
                        img.filename, img.expected, final_mood, img.expected_intensity, vote_list);
                    v12t_results.push((
                        img.filename.to_string(),
                        img.expected.to_string(),
                        final_mood,
                        strict,
                        relaxed,
                        final_int,
                        img.expected_intensity,
                    ));
                }

                // Vote stats
                println!("\n  Vote stats: {vote_unanimous} unanimous, {vote_majority} majority (2-1), {vote_split} three-way splits");

                let v12t_total = images.len() as u32;
                let v12t_processed = v12t_total - v12t_errs;
                let v12t_pct_s = if v12t_processed > 0 {
                    v12t_cs as f64 / v12t_processed as f64 * 100.0
                } else {
                    0.0
                };
                let v12t_pct_r = if v12t_processed > 0 {
                    v12t_cr as f64 / v12t_processed as f64 * 100.0
                } else {
                    0.0
                };
                let v12t_pct_i = if v12t_processed > 0 {
                    v12t_int as f64 / v12t_processed as f64 * 100.0
                } else {
                    0.0
                };
                let v12t_sc_s = if v12t_pct_s >= 60.0 {
                    GREEN
                } else if v12t_pct_s >= 40.0 {
                    YELLOW
                } else {
                    RED
                };
                let v12t_sc_r = if v12t_pct_r >= 60.0 {
                    GREEN
                } else if v12t_pct_r >= 40.0 {
                    YELLOW
                } else {
                    RED
                };
                let v12t_sc_i = if v12t_pct_i >= 50.0 {
                    GREEN
                } else if v12t_pct_i >= 30.0 {
                    YELLOW
                } else {
                    RED
                };

                // Delta from Pass 1
                let p1_cs = all_results[0].correct_strict;
                let p1_cr = all_results[0].correct_relaxed;
                let delta_s = v12t_cs as i32 - p1_cs as i32;
                let delta_r = v12t_cr as i32 - p1_cr as i32;
                let delta_s_str = if delta_s > 0 {
                    format!(" {GREEN}+{delta_s}{RESET}")
                } else if delta_s < 0 {
                    format!(" {RED}{delta_s}{RESET}")
                } else {
                    String::new()
                };
                let delta_r_str = if delta_r > 0 {
                    format!(" {GREEN}+{delta_r}{RESET}")
                } else if delta_r < 0 {
                    format!(" {RED}{delta_r}{RESET}")
                } else {
                    String::new()
                };

                println!(
                    "\n  {BOLD}=> Mood strict: {v12t_sc_s}{v12t_cs}/{v12t_processed} ({v12t_pct_s:.0}%){RESET}{delta_s_str}  {BOLD}Relaxed: {v12t_sc_r}{v12t_cr}/{v12t_processed} ({v12t_pct_r:.0}%){RESET}{delta_r_str}  {BOLD}Intensity: {v12t_sc_i}{v12t_int}/{v12t_processed} ({v12t_pct_i:.0}%){RESET}  {DIM}(delta from Pass 1){RESET}\n"
                );

                all_results.push(ModelResult {
                    name: v12t_name,
                    results: v12t_results,
                    correct_strict: v12t_cs,
                    correct_relaxed: v12t_cr,
                    intensity_correct: v12t_int,
                    errors: v12t_errs,
                    total: v12t_total,
                });
            }
            drop(server);
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        // ━━━ Comparison table ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        println!("\n  {BOLD}{CYAN}━━━ COMPARISON ━━━{RESET}\n");

        print!("  {DIM}{:<30} {:<8}", "Image", "Expected");
        for r in &all_results {
            print!(" {:<26}", r.name);
        }
        println!("{RESET}");
        print!("  {DIM}{}", "-".repeat(40));
        for _ in &all_results {
            print!("{}", "-".repeat(26));
        }
        println!("{RESET}");

        for (i, &(_idx, filename, expected, exp_int)) in GROUND_TRUTH.iter().enumerate() {
            print!("  {:<30} {:<2} i{}", filename, expected, exp_int);
            for model_r in &all_results {
                if let Some((_f, _e, detected, strict, relaxed, det_int, _)) =
                    model_r.results.get(i)
                {
                    let (color, icon) = if *strict {
                        (GREEN, "pass")
                    } else if *relaxed {
                        (CYAN, "~ok~")
                    } else {
                        (RED, "FAIL")
                    };
                    print!(" {color}{:<12} i{} {icon}{RESET}     ", detected, det_int);
                } else {
                    print!(" {:<26}", "-");
                }
            }
            println!();
        }

        print!("\n  {BOLD}{:<30} {:<8}", "", "STRICT");
        for r in &all_results {
            let processed = r.total - r.errors;
            let pct = if processed > 0 {
                r.correct_strict as f64 / processed as f64 * 100.0
            } else {
                0.0
            };
            let sc = if pct >= 60.0 {
                GREEN
            } else if pct >= 40.0 {
                YELLOW
            } else {
                RED
            };
            print!(
                " {sc}{:>2}/{:<2} ({:.0}%)               {RESET}",
                r.correct_strict, processed, pct
            );
        }
        println!("{RESET}");

        print!("  {BOLD}{:<30} {:<8}", "", "RELAXED");
        for r in &all_results {
            let processed = r.total - r.errors;
            let pct = if processed > 0 {
                r.correct_relaxed as f64 / processed as f64 * 100.0
            } else {
                0.0
            };
            let sc = if pct >= 60.0 {
                GREEN
            } else if pct >= 40.0 {
                YELLOW
            } else {
                RED
            };
            print!(
                " {sc}{:>2}/{:<2} ({:.0}%)               {RESET}",
                r.correct_relaxed, processed, pct
            );
        }
        println!("{RESET}");

        print!("  {BOLD}{:<30} {:<8}", "", "INTENS.");
        for r in &all_results {
            let processed = r.total - r.errors;
            let pct = if processed > 0 {
                r.intensity_correct as f64 / processed as f64 * 100.0
            } else {
                0.0
            };
            let sc = if pct >= 50.0 {
                GREEN
            } else if pct >= 30.0 {
                YELLOW
            } else {
                RED
            };
            print!(
                " {sc}{:>2}/{:<2} ({:.0}%)               {RESET}",
                r.intensity_correct, processed, pct
            );
        }
        println!("{RESET}\n");
    }

    /// Real test benchmark: V12 pipeline on 1047 pages across 6 manga series.
    /// Uses ground_truth.json for expected moods.
    /// Filter with REALTEST_FILTER env var (e.g. "BL", "BL/1", "BL,TPN").
    ///
    /// Default protocol: comparative RealTest suite with cached historical baseline + live variants.
    /// Run: cargo test --manifest-path src-tauri/Cargo.toml realtest_benchmark -- --ignored --nocapture
    /// Run filtered: REALTEST_FILTER=BL/1 cargo test --manifest-path src-tauri/Cargo.toml realtest_benchmark -- --ignored --nocapture
    /// Narrow experiments: REALTEST_EXPERIMENTS=t4b_viterbi,q35_9b_viterbi cargo test --manifest-path src-tauri/Cargo.toml realtest_benchmark -- --ignored --nocapture
    /// Smoke run: REALTEST_FILTER=BL/1 REALTEST_PAGE_LIMIT=3 cargo test --manifest-path src-tauri/Cargo.toml realtest_benchmark -- --ignored --nocapture
    #[tokio::test]
    #[ignore]
    async fn realtest_benchmark() {
        realtest_suite::run().await;
    }
}
