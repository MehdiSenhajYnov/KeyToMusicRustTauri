use serde::{Deserialize, Serialize};

use crate::types::BaseMood;

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
