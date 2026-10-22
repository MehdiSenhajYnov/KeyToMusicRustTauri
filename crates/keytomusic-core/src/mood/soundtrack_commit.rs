use std::collections::VecDeque;

use super::shared::{MoodScores, NarrativeRole};
use crate::types::{BaseMood, MoodIntensity};

const MAX_RECENT_OBSERVATIONS: usize = 6;
const MAX_CONTIGUOUS_PAGE_GAP: u32 = 3;
const CONTEXT_RADIUS: u32 = 2;
const MIN_FUTURE_PAGES_FOR_FORCED_FAMILY_EXIT: usize = 2;
const MIN_FAMILY_STREAK_TO_SWITCH: usize = 2;
const MIN_MOOD_STREAK_TO_SWITCH_WITHIN_FAMILY: usize = 2;
const FAMILY_SUPPORT_THRESHOLD: f32 = 0.55;
const CONTEXT_FAMILY_SUPPORT_THRESHOLD: f32 = 0.50;
const CONTEXT_FAMILY_GAP_THRESHOLD: f32 = 0.02;
const CONTEXT_MOOD_SUPPORT_THRESHOLD: f32 = 0.50;
const CONTEXT_MOOD_GAP_THRESHOLD: f32 = 0.02;
const SAME_FAMILY_MOOD_GAP_THRESHOLD: f32 = 0.08;
const SAME_FAMILY_MOOD_SCORE_THRESHOLD: f32 = 0.50;
const STRONG_CURRENT_FAMILY_SUPPORT_THRESHOLD: f32 = 0.72;
const STRONG_CURRENT_FAMILY_GAP_THRESHOLD: f32 = 0.22;
const STRONG_CURRENT_MOOD_SCORE_THRESHOLD: f32 = 0.58;
const STRONG_CURRENT_MOOD_GAP_THRESHOLD: f32 = 0.12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SoundtrackFamily {
    Action,
    Emotion,
    Light,
    Suspense,
}

impl SoundtrackFamily {
    const ALL: [SoundtrackFamily; 4] = [
        SoundtrackFamily::Action,
        SoundtrackFamily::Emotion,
        SoundtrackFamily::Light,
        SoundtrackFamily::Suspense,
    ];
}

#[derive(Clone, Debug)]
pub struct SoundtrackObservation {
    pub page: Option<u32>,
    pub mood: BaseMood,
    pub intensity: MoodIntensity,
    pub scores: MoodScores,
    pub narrative_role: NarrativeRole,
    pub finalized: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SoundtrackCommitDecision {
    pub raw_mood: BaseMood,
    pub raw_intensity: MoodIntensity,
    pub committed_mood: BaseMood,
    pub committed_intensity: MoodIntensity,
    pub previous_mood: Option<BaseMood>,
    pub previous_intensity: Option<MoodIntensity>,
    pub mood_changed: bool,
    pub intensity_changed: bool,
    pub dwell_count: u32,
}

#[derive(Clone, Debug)]
struct RecentObservation {
    page: Option<u32>,
    mood: BaseMood,
    intensity: MoodIntensity,
    scores: MoodScores,
    dominant_family: SoundtrackFamily,
}

impl RecentObservation {
    fn from_observation(observation: &SoundtrackObservation) -> Self {
        Self {
            page: observation.page,
            mood: observation.mood,
            intensity: observation.intensity,
            scores: observation.scores.clone(),
            dominant_family: dominant_family(&observation.scores, observation.mood),
        }
    }
}

pub struct SoundtrackCommitter {
    current_chapter: Option<String>,
    committed_mood: Option<BaseMood>,
    committed_intensity: Option<MoodIntensity>,
    dwell_count: u32,
    recent: VecDeque<RecentObservation>,
    last_page: Option<u32>,
}

impl SoundtrackCommitter {
    pub fn new() -> Self {
        Self {
            current_chapter: None,
            committed_mood: None,
            committed_intensity: None,
            dwell_count: 0,
            recent: VecDeque::new(),
            last_page: None,
        }
    }

    pub fn reset(&mut self) {
        self.current_chapter = None;
        self.committed_mood = None;
        self.committed_intensity = None;
        self.dwell_count = 0;
        self.recent.clear();
        self.last_page = None;
    }

    pub fn update(
        &mut self,
        chapter: Option<&str>,
        observation: SoundtrackObservation,
    ) -> SoundtrackCommitDecision {
        self.update_with_context(chapter, observation, &[])
    }

    pub fn update_with_lookahead(
        &mut self,
        chapter: Option<&str>,
        observation: SoundtrackObservation,
        lookahead: Option<SoundtrackObservation>,
    ) -> SoundtrackCommitDecision {
        let future = lookahead.into_iter().collect::<Vec<_>>();
        self.update_with_context(chapter, observation, &future)
    }

    pub fn update_with_context(
        &mut self,
        chapter: Option<&str>,
        observation: SoundtrackObservation,
        future_context: &[SoundtrackObservation],
    ) -> SoundtrackCommitDecision {
        self.sync_chapter(chapter);

        let previous_mood = self.committed_mood;
        let previous_intensity = self.committed_intensity;

        if observation.page.is_some() && observation.page == self.last_page {
            return self.decision_from_current(
                observation.mood,
                observation.intensity,
                previous_mood,
                previous_intensity,
                false,
                false,
            );
        }

        self.clear_recent_if_discontinuous(observation.page);
        self.push_recent(RecentObservation::from_observation(&observation));
        self.last_page = observation.page;
        let future_recent = collect_future_context(observation.page, future_context);
        let has_future_context = !future_recent.is_empty();
        let context_scores = self.compute_context_scores(observation.page, &future_recent);

        if self.committed_mood.is_none() || self.committed_intensity.is_none() {
            self.committed_mood = Some(observation.mood);
            self.committed_intensity = Some(observation.intensity);
            self.dwell_count = 1;
            return SoundtrackCommitDecision {
                raw_mood: observation.mood,
                raw_intensity: observation.intensity,
                committed_mood: observation.mood,
                committed_intensity: observation.intensity,
                previous_mood,
                previous_intensity,
                mood_changed: previous_mood != Some(observation.mood),
                intensity_changed: previous_intensity != Some(observation.intensity),
                dwell_count: self.dwell_count,
            };
        }

        let current_mood = self.committed_mood.expect("committed mood should exist");
        let current_intensity = self
            .committed_intensity
            .expect("committed intensity should exist");
        let committed_family = family_of(current_mood);
        let candidate_family = dominant_family(&observation.scores, observation.mood);

        let mut next_mood = current_mood;
        let mut next_intensity = current_intensity;

        if candidate_family != committed_family {
            if let Some(target_family) = self.resolve_cross_family_switch_family(
                committed_family,
                candidate_family,
                &observation,
                &context_scores,
                &future_recent,
                has_future_context,
            ) {
                next_mood = self.select_cross_family_target_mood(
                    target_family,
                    &observation,
                    &context_scores,
                );
                next_intensity = self.select_cross_family_target_intensity(
                    next_mood,
                    target_family,
                    &observation,
                    &future_recent,
                );
            }
        } else if observation.mood != current_mood
            && self.should_switch_within_family(
                current_mood,
                &observation,
                &context_scores,
                has_future_context,
            )
        {
            next_mood = observation.mood;
            next_intensity = observation.intensity;
        }

        if next_mood == current_mood {
            next_intensity = self.resolve_intensity(
                current_mood,
                current_intensity,
                &observation,
                &future_recent,
            );
        }

        let mood_changed = next_mood != current_mood;
        let intensity_changed = next_intensity != current_intensity;

        self.committed_mood = Some(next_mood);
        self.committed_intensity = Some(next_intensity);
        if mood_changed {
            self.dwell_count = 1;
        } else {
            self.dwell_count += 1;
        }

        SoundtrackCommitDecision {
            raw_mood: observation.mood,
            raw_intensity: observation.intensity,
            committed_mood: next_mood,
            committed_intensity: next_intensity,
            previous_mood,
            previous_intensity,
            mood_changed,
            intensity_changed,
            dwell_count: self.dwell_count,
        }
    }

    fn sync_chapter(&mut self, chapter: Option<&str>) {
        let Some(chapter) = chapter else {
            return;
        };

        if self.current_chapter.as_deref() == Some(chapter) {
            return;
        }

        self.reset();
        self.current_chapter = Some(chapter.to_string());
    }

    fn decision_from_current(
        &self,
        raw_mood: BaseMood,
        raw_intensity: MoodIntensity,
        previous_mood: Option<BaseMood>,
        previous_intensity: Option<MoodIntensity>,
        mood_changed: bool,
        intensity_changed: bool,
    ) -> SoundtrackCommitDecision {
        SoundtrackCommitDecision {
            raw_mood,
            raw_intensity,
            committed_mood: self.committed_mood.unwrap_or(raw_mood),
            committed_intensity: self.committed_intensity.unwrap_or(raw_intensity),
            previous_mood,
            previous_intensity,
            mood_changed,
            intensity_changed,
            dwell_count: self.dwell_count.max(1),
        }
    }

    fn clear_recent_if_discontinuous(&mut self, page: Option<u32>) {
        let (Some(previous), Some(current)) = (self.last_page, page) else {
            return;
        };

        if current <= previous || current > previous + MAX_CONTIGUOUS_PAGE_GAP {
            self.recent.clear();
        }
    }

    fn push_recent(&mut self, observation: RecentObservation) {
        self.recent.push_back(observation);
        while self.recent.len() > MAX_RECENT_OBSERVATIONS {
            let _ = self.recent.pop_front();
        }
    }

    fn should_switch_family(
        &self,
        committed_family: SoundtrackFamily,
        candidate_family: SoundtrackFamily,
        observation: &SoundtrackObservation,
        context_scores: &MoodScores,
        has_future_context: bool,
    ) -> bool {
        let context_winner = dominant_family(context_scores, observation.mood);
        if context_winner != candidate_family {
            return !has_future_context
                && self.strong_current_family_override(
                    committed_family,
                    candidate_family,
                    observation,
                );
        }

        let context_candidate_support = family_score(context_scores, candidate_family);
        let context_committed_support = family_score(context_scores, committed_family);
        let context_gap = context_candidate_support - context_committed_support;
        let current_candidate_support = family_score(&observation.scores, candidate_family);
        let current_committed_support = family_score(&observation.scores, committed_family);
        let current_gap = current_candidate_support - current_committed_support;

        (context_candidate_support >= CONTEXT_FAMILY_SUPPORT_THRESHOLD
            && context_gap >= CONTEXT_FAMILY_GAP_THRESHOLD
            && (current_candidate_support >= FAMILY_SUPPORT_THRESHOLD || current_gap >= 0.0))
            || (!has_future_context
                && self.strong_current_family_override(
                    committed_family,
                    candidate_family,
                    observation,
                ))
    }

    fn resolve_cross_family_switch_family(
        &self,
        committed_family: SoundtrackFamily,
        candidate_family: SoundtrackFamily,
        observation: &SoundtrackObservation,
        context_scores: &MoodScores,
        future_context: &[RecentObservation],
        has_future_context: bool,
    ) -> Option<SoundtrackFamily> {
        if self.should_switch_family(
            committed_family,
            candidate_family,
            observation,
            context_scores,
            has_future_context,
        ) {
            return Some(candidate_family);
        }

        self.forced_exit_switch_family(
            committed_family,
            candidate_family,
            observation,
            context_scores,
            future_context,
        )
    }

    fn forced_exit_switch_family(
        &self,
        committed_family: SoundtrackFamily,
        candidate_family: SoundtrackFamily,
        observation: &SoundtrackObservation,
        context_scores: &MoodScores,
        future_context: &[RecentObservation],
    ) -> Option<SoundtrackFamily> {
        if committed_family != SoundtrackFamily::Light {
            return None;
        }

        if future_context.len() < MIN_FUTURE_PAGES_FOR_FORCED_FAMILY_EXIT {
            return None;
        }

        if future_context
            .iter()
            .take(MIN_FUTURE_PAGES_FOR_FORCED_FAMILY_EXIT)
            .any(|next| next.dominant_family == committed_family)
        {
            return None;
        }

        Some(best_alternative_family(
            context_scores,
            committed_family,
            candidate_family,
            observation.mood,
        ))
    }

    fn should_switch_within_family(
        &self,
        current_mood: BaseMood,
        observation: &SoundtrackObservation,
        context_scores: &MoodScores,
        has_future_context: bool,
    ) -> bool {
        let family = family_of(observation.mood);
        let context_winner = dominant_mood_in_family(context_scores, family, current_mood);
        if context_winner != observation.mood {
            return !has_future_context
                && self.strong_current_mood_override(current_mood, observation);
        }

        let context_candidate_score = context_scores.get(observation.mood);
        let context_current_score = context_scores.get(current_mood);
        let current_candidate_score = observation.scores.get(observation.mood);
        let current_score = observation.scores.get(current_mood);

        (context_candidate_score >= CONTEXT_MOOD_SUPPORT_THRESHOLD
            && context_candidate_score - context_current_score >= CONTEXT_MOOD_GAP_THRESHOLD)
            || (current_candidate_score >= SAME_FAMILY_MOOD_SCORE_THRESHOLD
                && current_candidate_score - current_score >= SAME_FAMILY_MOOD_GAP_THRESHOLD
                && self.trailing_mood_streak(observation.mood)
                    >= MIN_MOOD_STREAK_TO_SWITCH_WITHIN_FAMILY)
            || (!has_future_context && self.strong_current_mood_override(current_mood, observation))
    }

    fn select_cross_family_target_mood(
        &self,
        target_family: SoundtrackFamily,
        observation: &SoundtrackObservation,
        context_scores: &MoodScores,
    ) -> BaseMood {
        let _ = observation;
        best_target_mood_in_family(context_scores, target_family)
    }

    fn select_cross_family_target_intensity(
        &self,
        target_mood: BaseMood,
        target_family: SoundtrackFamily,
        observation: &SoundtrackObservation,
        future_context: &[RecentObservation],
    ) -> MoodIntensity {
        if observation.mood == target_mood || family_of(observation.mood) == target_family {
            return observation.intensity;
        }

        if let Some(intensity) = self
            .recent
            .iter()
            .rev()
            .chain(future_context.iter())
            .find(|candidate| candidate.mood == target_mood)
            .map(|candidate| candidate.intensity)
        {
            return intensity;
        }

        if let Some(intensity) = self
            .recent
            .iter()
            .rev()
            .chain(future_context.iter())
            .find(|candidate| family_of(candidate.mood) == target_family)
            .map(|candidate| candidate.intensity)
        {
            return intensity;
        }

        observation.intensity
    }

    fn resolve_intensity(
        &self,
        committed_mood: BaseMood,
        committed_intensity: MoodIntensity,
        observation: &SoundtrackObservation,
        future_context: &[RecentObservation],
    ) -> MoodIntensity {
        if observation.intensity == committed_intensity {
            return committed_intensity;
        }

        let same_mood_streak = self.trailing_mood_streak(committed_mood);
        let same_family_streak = self.trailing_family_streak(family_of(committed_mood));
        let context_confirms = future_context
            .iter()
            .any(|next| next.mood == committed_mood && next.intensity == observation.intensity);

        if observation.intensity.as_u8() > committed_intensity.as_u8() {
            if same_mood_streak >= MIN_MOOD_STREAK_TO_SWITCH_WITHIN_FAMILY
                || context_confirms
                || (same_family_streak >= MIN_FAMILY_STREAK_TO_SWITCH
                    && matches!(
                        observation.narrative_role,
                        NarrativeRole::Escalation | NarrativeRole::Climax
                    )
                    && observation.intensity == MoodIntensity::High)
            {
                return observation.intensity;
            }
            return committed_intensity;
        }

        if same_mood_streak >= MIN_MOOD_STREAK_TO_SWITCH_WITHIN_FAMILY
            || context_confirms
            || (same_family_streak >= MIN_FAMILY_STREAK_TO_SWITCH
                && matches!(
                    observation.narrative_role,
                    NarrativeRole::DeEscalation | NarrativeRole::Transition
                ))
        {
            return observation.intensity;
        }

        committed_intensity
    }

    fn trailing_family_streak(&self, family: SoundtrackFamily) -> usize {
        self.recent
            .iter()
            .rev()
            .take_while(|observation| observation.dominant_family == family)
            .count()
    }

    fn trailing_mood_streak(&self, mood: BaseMood) -> usize {
        self.recent
            .iter()
            .rev()
            .take_while(|observation| observation.mood == mood)
            .count()
    }

    fn compute_context_scores(
        &self,
        current_page: Option<u32>,
        future_context: &[RecentObservation],
    ) -> MoodScores {
        let mut result = MoodScores::new();
        let Some(current_page) = current_page else {
            return self
                .recent
                .back()
                .map(|observation| observation.scores.clone())
                .unwrap_or(result);
        };

        let mut total_weight = 0.0f32;
        for observation in self.recent.iter().chain(future_context.iter()) {
            let Some(page) = observation.page else {
                continue;
            };
            let distance = page.abs_diff(current_page);
            if distance > CONTEXT_RADIUS {
                continue;
            }
            let weight = context_weight(distance);
            total_weight += weight;
            for mood in BaseMood::ALL {
                let blended = result.get(mood) + observation.scores.get(mood) * weight;
                result.set(mood, blended);
            }
        }

        if total_weight <= f32::EPSILON {
            return self
                .recent
                .back()
                .map(|observation| observation.scores.clone())
                .unwrap_or(result);
        }

        for mood in BaseMood::ALL {
            result.set(mood, result.get(mood) / total_weight);
        }

        result
    }

    fn strong_current_family_override(
        &self,
        committed_family: SoundtrackFamily,
        candidate_family: SoundtrackFamily,
        observation: &SoundtrackObservation,
    ) -> bool {
        let candidate_support = family_score(&observation.scores, candidate_family);
        let committed_support = family_score(&observation.scores, committed_family);
        candidate_support >= STRONG_CURRENT_FAMILY_SUPPORT_THRESHOLD
            && candidate_support - committed_support >= STRONG_CURRENT_FAMILY_GAP_THRESHOLD
            && observation.narrative_role != NarrativeRole::Continuation
    }

    fn strong_current_mood_override(
        &self,
        current_mood: BaseMood,
        observation: &SoundtrackObservation,
    ) -> bool {
        let candidate_score = observation.scores.get(observation.mood);
        let current_score = observation.scores.get(current_mood);
        candidate_score >= STRONG_CURRENT_MOOD_SCORE_THRESHOLD
            && candidate_score - current_score >= STRONG_CURRENT_MOOD_GAP_THRESHOLD
            && observation.narrative_role != NarrativeRole::Continuation
    }
}

fn family_of(mood: BaseMood) -> SoundtrackFamily {
    match mood {
        BaseMood::Epic | BaseMood::Tension => SoundtrackFamily::Action,
        BaseMood::Sadness | BaseMood::Romance => SoundtrackFamily::Emotion,
        BaseMood::Comedy | BaseMood::Peaceful => SoundtrackFamily::Light,
        BaseMood::Horror | BaseMood::Mystery => SoundtrackFamily::Suspense,
    }
}

fn family_score(scores: &MoodScores, family: SoundtrackFamily) -> f32 {
    match family {
        SoundtrackFamily::Action => scores.get(BaseMood::Epic) + scores.get(BaseMood::Tension),
        SoundtrackFamily::Emotion => scores.get(BaseMood::Sadness) + scores.get(BaseMood::Romance),
        SoundtrackFamily::Light => scores.get(BaseMood::Comedy) + scores.get(BaseMood::Peaceful),
        SoundtrackFamily::Suspense => scores.get(BaseMood::Horror) + scores.get(BaseMood::Mystery),
    }
}

fn dominant_family(scores: &MoodScores, fallback_mood: BaseMood) -> SoundtrackFamily {
    let fallback_family = family_of(fallback_mood);
    let mut best_family = fallback_family;
    let mut best_score = family_score(scores, fallback_family);

    for family in SoundtrackFamily::ALL {
        let score = family_score(scores, family);
        if score > best_score + f32::EPSILON {
            best_score = score;
            best_family = family;
        }
    }

    best_family
}

fn dominant_mood_in_family(
    scores: &MoodScores,
    family: SoundtrackFamily,
    fallback_mood: BaseMood,
) -> BaseMood {
    let moods = family_moods(family);
    let first_score = scores.get(moods[0]);
    let second_score = scores.get(moods[1]);
    if first_score > second_score + f32::EPSILON {
        moods[0]
    } else if second_score > first_score + f32::EPSILON {
        moods[1]
    } else if family_of(fallback_mood) == family {
        fallback_mood
    } else {
        moods[0]
    }
}

fn family_moods(family: SoundtrackFamily) -> [BaseMood; 2] {
    match family {
        SoundtrackFamily::Action => [BaseMood::Epic, BaseMood::Tension],
        SoundtrackFamily::Emotion => [BaseMood::Sadness, BaseMood::Romance],
        SoundtrackFamily::Light => [BaseMood::Comedy, BaseMood::Peaceful],
        SoundtrackFamily::Suspense => [BaseMood::Horror, BaseMood::Mystery],
    }
}

fn best_alternative_family(
    scores: &MoodScores,
    committed_family: SoundtrackFamily,
    fallback_family: SoundtrackFamily,
    fallback_mood: BaseMood,
) -> SoundtrackFamily {
    let mut best_family = fallback_family;
    let mut best_score = family_score(scores, fallback_family);

    for family in SoundtrackFamily::ALL {
        if family == committed_family {
            continue;
        }

        let score = family_score(scores, family);
        if score > best_score + f32::EPSILON
            || ((score - best_score).abs() <= f32::EPSILON
                && better_family_tiebreak(family, best_family, fallback_family, fallback_mood))
        {
            best_score = score;
            best_family = family;
        }
    }

    best_family
}

fn better_family_tiebreak(
    candidate: SoundtrackFamily,
    current_best: SoundtrackFamily,
    fallback_family: SoundtrackFamily,
    fallback_mood: BaseMood,
) -> bool {
    if candidate == current_best {
        return false;
    }

    if candidate == fallback_family && current_best != fallback_family {
        return true;
    }

    if current_best == fallback_family && candidate != fallback_family {
        return false;
    }

    if family_of(fallback_mood) == candidate && family_of(fallback_mood) != current_best {
        return true;
    }

    if family_of(fallback_mood) == current_best && family_of(fallback_mood) != candidate {
        return false;
    }

    family_priority(candidate) < family_priority(current_best)
}

fn family_priority(family: SoundtrackFamily) -> u8 {
    match family {
        SoundtrackFamily::Action => 0,
        SoundtrackFamily::Suspense => 1,
        SoundtrackFamily::Emotion => 2,
        SoundtrackFamily::Light => 3,
    }
}

fn best_target_mood_in_family(scores: &MoodScores, family: SoundtrackFamily) -> BaseMood {
    let moods = preferred_target_moods(family);
    let first_score = scores.get(moods[0]);
    let second_score = scores.get(moods[1]);

    if first_score > second_score + f32::EPSILON {
        moods[0]
    } else if second_score > first_score + f32::EPSILON {
        moods[1]
    } else {
        moods[0]
    }
}

fn preferred_target_moods(family: SoundtrackFamily) -> [BaseMood; 2] {
    match family {
        SoundtrackFamily::Action => [BaseMood::Tension, BaseMood::Epic],
        SoundtrackFamily::Emotion => [BaseMood::Sadness, BaseMood::Romance],
        SoundtrackFamily::Light => [BaseMood::Peaceful, BaseMood::Comedy],
        SoundtrackFamily::Suspense => [BaseMood::Mystery, BaseMood::Horror],
    }
}

fn is_immediate_next_page(current: Option<u32>, next: Option<u32>) -> bool {
    matches!((current, next), (Some(current), Some(next)) if next == current + 1)
}

fn collect_future_context(
    current_page: Option<u32>,
    future: &[SoundtrackObservation],
) -> Vec<RecentObservation> {
    let Some(current_page) = current_page else {
        return Vec::new();
    };

    let mut collected = Vec::new();
    let mut expected_page = current_page + 1;
    for observation in future {
        let Some(page) = observation.page else {
            break;
        };
        if !is_immediate_next_page(Some(expected_page - 1), Some(page)) {
            break;
        }
        if page.abs_diff(current_page) > CONTEXT_RADIUS {
            break;
        }
        collected.push(RecentObservation::from_observation(observation));
        expected_page = page + 1;
    }

    collected
}

fn context_weight(distance: u32) -> f32 {
    match distance {
        0 => 1.0,
        1 => 0.8,
        _ => 0.45,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one_hot(page: u32, mood: BaseMood) -> SoundtrackObservation {
        SoundtrackObservation {
            page: Some(page),
            mood,
            intensity: MoodIntensity::Medium,
            scores: MoodScores::from_single(mood),
            narrative_role: NarrativeRole::Continuation,
            finalized: true,
        }
    }

    #[test]
    fn single_cross_family_spike_does_not_switch_soundtrack() {
        let mut committer = SoundtrackCommitter::new();
        let chapter = Some("BL/2");

        let _ = committer.update(chapter, one_hot(20, BaseMood::Tension));
        let _ = committer.update(chapter, one_hot(21, BaseMood::Tension));
        let decision = committer.update(chapter, one_hot(22, BaseMood::Comedy));

        assert_eq!(decision.committed_mood, BaseMood::Tension);
        assert!(!decision.mood_changed);
    }

    #[test]
    fn sustained_cross_family_run_switches_after_second_page() {
        let mut committer = SoundtrackCommitter::new();
        let chapter = Some("OP/4");

        let _ = committer.update(chapter, one_hot(7, BaseMood::Peaceful));
        let _ = committer.update(chapter, one_hot(8, BaseMood::Peaceful));
        let _ = committer.update(chapter, one_hot(9, BaseMood::Comedy));
        let decision = committer.update(chapter, one_hot(10, BaseMood::Comedy));

        assert_eq!(decision.committed_mood, BaseMood::Comedy);
        assert!(decision.mood_changed);
    }

    #[test]
    fn single_same_family_spike_does_not_switch_soundtrack() {
        let mut committer = SoundtrackCommitter::new();
        let chapter = Some("BL/2");

        let _ = committer.update(chapter, one_hot(21, BaseMood::Tension));
        let _ = committer.update(chapter, one_hot(22, BaseMood::Tension));
        let decision = committer.update(chapter, one_hot(23, BaseMood::Epic));

        assert_eq!(decision.committed_mood, BaseMood::Tension);
        assert!(!decision.mood_changed);
    }

    #[test]
    fn repeated_same_family_signal_can_promote_within_family() {
        let mut committer = SoundtrackCommitter::new();
        let chapter = Some("BL/2");

        let _ = committer.update(chapter, one_hot(21, BaseMood::Tension));
        let _ = committer.update(chapter, one_hot(22, BaseMood::Tension));
        let _ = committer.update(chapter, one_hot(23, BaseMood::Epic));
        let decision = committer.update(chapter, one_hot(24, BaseMood::Epic));

        assert_eq!(decision.committed_mood, BaseMood::Epic);
        assert!(decision.mood_changed);
    }

    #[test]
    fn lookahead_allows_first_page_switch_without_one_page_delay() {
        let mut committer = SoundtrackCommitter::new();
        let chapter = Some("BL/2");

        let _ = committer.update(chapter, one_hot(21, BaseMood::Tension));
        let decision = committer.update_with_lookahead(
            chapter,
            one_hot(22, BaseMood::Epic),
            Some(one_hot(23, BaseMood::Epic)),
        );

        assert_eq!(decision.committed_mood, BaseMood::Epic);
        assert!(decision.mood_changed);
    }

    #[test]
    fn context_does_not_switch_on_single_page_spike() {
        let mut committer = SoundtrackCommitter::new();
        let chapter = Some("BL/2");

        let _ = committer.update(chapter, one_hot(21, BaseMood::Tension));
        let decision = committer.update_with_context(
            chapter,
            one_hot(22, BaseMood::Comedy),
            &[
                one_hot(23, BaseMood::Tension),
                one_hot(24, BaseMood::Tension),
            ],
        );

        assert_eq!(decision.committed_mood, BaseMood::Tension);
        assert!(!decision.mood_changed);
    }

    fn weighted_scores(primary: BaseMood, secondary: BaseMood) -> MoodScores {
        let mut scores = MoodScores::new();
        scores.set(primary, 0.78);
        scores.set(secondary, 0.14);
        scores.set(BaseMood::Peaceful, 0.03);
        scores.set(BaseMood::Sadness, 0.03);
        scores.set(BaseMood::Mystery, 0.02);
        scores
    }

    #[test]
    fn strong_current_scores_can_switch_without_future_context() {
        let mut committer = SoundtrackCommitter::new();
        let chapter = Some("OP/4");

        let _ = committer.update(chapter, one_hot(7, BaseMood::Peaceful));
        let _ = committer.update(chapter, one_hot(8, BaseMood::Peaceful));

        let decision = committer.update(
            chapter,
            SoundtrackObservation {
                page: Some(9),
                mood: BaseMood::Tension,
                intensity: MoodIntensity::High,
                scores: weighted_scores(BaseMood::Tension, BaseMood::Epic),
                narrative_role: NarrativeRole::Escalation,
                finalized: true,
            },
        );

        assert_eq!(decision.committed_mood, BaseMood::Tension);
        assert!(decision.mood_changed);
    }

    #[test]
    fn forced_exit_releases_stale_light_segment_when_next_two_pages_leave_family() {
        let mut committer = SoundtrackCommitter::new();
        let chapter = Some("BL/3");

        let _ = committer.update(chapter, one_hot(8, BaseMood::Comedy));
        let decision = committer.update_with_context(
            chapter,
            one_hot(9, BaseMood::Tension),
            &[
                one_hot(10, BaseMood::Horror),
                one_hot(11, BaseMood::Tension),
            ],
        );

        assert_eq!(decision.committed_mood, BaseMood::Tension);
        assert!(decision.mood_changed);
    }

    #[test]
    fn forced_exit_picks_context_anchor_instead_of_raw_cross_family_label() {
        let mut committer = SoundtrackCommitter::new();
        let chapter = Some("BL/3");

        let _ = committer.update(chapter, one_hot(8, BaseMood::Comedy));
        let decision = committer.update_with_context(
            chapter,
            one_hot(9, BaseMood::Horror),
            &[
                one_hot(10, BaseMood::Tension),
                one_hot(11, BaseMood::Tension),
            ],
        );

        assert_eq!(decision.committed_mood, BaseMood::Tension);
        assert_eq!(decision.committed_intensity, MoodIntensity::Medium);
        assert!(decision.mood_changed);
    }

    #[test]
    fn forced_exit_only_applies_to_stale_light_segments() {
        let mut committer = SoundtrackCommitter::new();
        let chapter = Some("OP/5");

        let _ = committer.update(chapter, one_hot(7, BaseMood::Tension));
        let _ = committer.update(chapter, one_hot(8, BaseMood::Tension));
        let decision = committer.update_with_context(
            chapter,
            one_hot(9, BaseMood::Comedy),
            &[
                one_hot(10, BaseMood::Sadness),
                one_hot(11, BaseMood::Sadness),
            ],
        );

        assert_eq!(decision.committed_mood, BaseMood::Tension);
        assert!(!decision.mood_changed);
    }
}
