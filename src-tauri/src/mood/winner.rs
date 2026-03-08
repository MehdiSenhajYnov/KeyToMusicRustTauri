use std::collections::BTreeMap;

use base64::Engine;
use image::GenericImageView;

use crate::mood::director::{MoodScores, NarrativeRole};
use crate::mood::inference::{
    self, extract_content, parse_mood_intensity_response, LlamaRuntimeIntent,
};
use crate::types::{BaseMood, MoodIntensity};

pub const WINNER_MODEL_KEY: &str = inference::ACTIVE_MOOD_MODEL_NAME;
pub const WINNER_BACKBONE_MAX_TOKENS: u32 = 8_192;
pub const WINNER_SELECTIVE_MAX_TOKENS: u32 = 512;

#[derive(Clone, Debug)]
pub struct EncodedImage {
    pub mime: &'static str,
    pub b64: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageRole {
    Context,
    Center,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrepMode {
    Legacy672Jpeg,
    CenterPng1024,
    AllPng1024,
}

#[derive(Clone, Debug)]
pub struct WindowPrediction {
    pub members: Vec<u32>,
    pub center: u32,
    pub mood: BaseMood,
    pub intensity: MoodIntensity,
}

#[derive(Clone, Debug)]
pub struct VoteStats {
    pub(crate) counts: [u32; 8],
    pub(crate) weighted: [f64; 8],
    pub(crate) intensity_sum: [u32; 8],
    pub(crate) intensity_count: [u32; 8],
    pub(crate) center_vote: Option<BaseMood>,
    pub(crate) center_intensity: Option<MoodIntensity>,
    pub(crate) winner_score: f64,
    pub(crate) runner_up_score: f64,
}

#[derive(Clone, Debug)]
pub struct AggregatedPrediction {
    pub mood: BaseMood,
    pub intensity: MoodIntensity,
    pub winner_score: f64,
    pub runner_up_score: f64,
    pub scores: MoodScores,
    pub narrative_role: NarrativeRole,
    pub note: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum SelectivePrompt {
    Focus,
    Narrative,
}

impl SelectivePrompt {
    pub fn note(self) -> &'static str {
        match self {
            Self::Focus => "selective_focus",
            Self::Narrative => "selective_narrative",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepairAcceptSingle {
    Tension,
    Mystery,
    MysteryOrTension,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepairAcceptPair {
    BothMystery,
    BothEpic,
    BothTension,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RepairPlan {
    Single {
        key: String,
        page: u32,
        prompt_kind: SelectivePrompt,
        reason: &'static str,
        accept: RepairAcceptSingle,
    },
    Pair {
        key: String,
        first_page: u32,
        second_page: u32,
        prompt_kind: SelectivePrompt,
        reason: &'static str,
        accept: RepairAcceptPair,
    },
}

#[derive(Clone, Copy, Debug)]
struct MoodRun {
    start: usize,
    end: usize,
}

impl MoodRun {
    fn len(self) -> usize {
        self.end - self.start + 1
    }
}

pub fn winner_runtime_intent() -> LlamaRuntimeIntent {
    LlamaRuntimeIntent::BenchmarkPrimary
}

pub fn wide_context_indices(center: u32, total_pages: u32) -> Vec<u32> {
    let max_page = total_pages.saturating_sub(1) as i64;
    let center = center as i64;
    [-2, -1, 0, 1, 2]
        .into_iter()
        .map(|offset| (center + offset).clamp(0, max_page) as u32)
        .collect()
}

pub fn build_wide_sequence_prompt(context_indices: &[u32], total_pages: u32) -> String {
    let page_numbers = context_indices
        .iter()
        .map(|idx| (idx + 1).to_string())
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "These are 5 consecutive manga pages from the same chapter (pages {} out of {}).\n\
        They are shown in reading order from earliest to latest.\n\
        \n\
        Considering the short narrative flow across all 5 pages, what is the dominant mood of this sequence for soundtrack purposes?\n\
        \n\
        Classify as ONE mood that best represents the 5-page segment:\n\
        - epic: climactic moments, declarations of resolve, power unleashed, clear payoff or triumph\n\
        - tension: buildup, uncertainty, standoffs, unresolved threats, pressure without release\n\
        - sadness: grief, defeat, crying, emotional pain, aftermath\n\
        - comedy: comic relief, gags, slapstick, funny reactions\n\
        - romance: intimacy, affection, tender moments\n\
        - horror: fear, monsters, gore, nightmare imagery\n\
        - peaceful: calm pause, daily life, quiet contemplation, friendly conversation\n\
        - mystery: secrets, scheming, ominous reveal, hidden motives, foreshadowing\n\
        \n\
        Rate the intensity from 1 (low) to 3 (high).\n\
        \n\
        Reply format: mood intensity\n\
        Example: tension 2",
        page_numbers,
        total_pages
    )
}

pub fn build_focus_prompt(page_idx: u32, total_pages: u32) -> String {
    format!(
        "These are 3 consecutive manga pages from the same chapter.\n\
        Image 1 is the PREVIOUS page, Image 2 is the CURRENT page, Image 3 is the NEXT page.\n\
        \n\
        Classify the CURRENT page only (Image 2) for soundtrack purposes.\n\
        Use the neighboring pages only to disambiguate the current page.\n\
        \n\
        Current page number: {} out of {}.\n\
        \n\
        Reply format: mood intensity\n\
        Allowed moods: epic, tension, sadness, comedy, romance, horror, peaceful, mystery",
        page_idx + 1,
        total_pages
    )
}

pub fn build_narrative_focus_prompt(page_idx: u32, total_pages: u32) -> String {
    format!(
        "These are 3 consecutive manga pages from the same chapter.\n\
        Image 1 is the PREVIOUS page, Image 2 is the CURRENT page, Image 3 is the NEXT page.\n\
        \n\
        Classify the CURRENT page only (Image 2) for soundtrack purposes.\n\
        Use neighboring pages only to understand the narrative function of the CURRENT page.\n\
        \n\
        First decide whether the CURRENT page is mainly:\n\
        - payoff / climax / release -> epic\n\
        - buildup / standoff / unresolved threat / evaluation -> tension\n\
        - grief / loss / crying / aftermath -> sadness\n\
        - comic relief / gag / silly reaction -> comedy\n\
        - tenderness / intimacy / affection -> romance\n\
        - fear / monster / nightmare / shock -> horror\n\
        - quiet pause / daily life / calm reflection -> peaceful\n\
        - reveal / secret / foreshadowing / scheming -> mystery\n\
        \n\
        Critical rules:\n\
        - action without release or payoff is tension, not epic\n\
        - hidden motives, reveals, and ominous setup are mystery, not epic\n\
        - a single quiet page can still be peaceful even between intense pages\n\
        - crying, regret, or aftermath are sadness, not epic\n\
        \n\
        Current page number: {} out of {}.\n\
        \n\
        Reply with ONLY: mood intensity\n\
        Example: tension 2",
        page_idx + 1,
        total_pages
    )
}

pub fn build_request_body(
    prompt: &str,
    images: &[EncodedImage],
    max_tokens: u32,
    temperature: f32,
    top_p: Option<f32>,
    top_k: Option<u32>,
    seed: Option<u32>,
    grammar: Option<&str>,
) -> serde_json::Value {
    let mut content = images
        .iter()
        .map(|image| {
            serde_json::json!({
                "type": "image_url",
                "image_url": { "url": format!("data:{};base64,{}", image.mime, image.b64) }
            })
        })
        .collect::<Vec<_>>();
    content.push(serde_json::json!({ "type": "text", "text": prompt }));

    let mut body = serde_json::json!({
        "model": WINNER_MODEL_KEY,
        "messages": [{
            "role": "user",
            "content": content
        }],
        "max_tokens": max_tokens,
        "temperature": temperature
    });

    if let Some(top_p) = top_p {
        body["top_p"] = serde_json::json!(top_p);
    }
    if let Some(top_k) = top_k {
        body["top_k"] = serde_json::json!(top_k);
    }
    if let Some(seed) = seed {
        body["seed"] = serde_json::json!(seed);
    }
    if let Some(grammar) = grammar {
        body["grammar"] = serde_json::json!(grammar);
    }

    body
}

pub fn encode_image(
    raw_bytes: &[u8],
    prep: PrepMode,
    role: ImageRole,
) -> Result<EncodedImage, String> {
    match prep {
        PrepMode::Legacy672Jpeg => Ok(EncodedImage {
            mime: "image/jpeg",
            b64: inference::prepare_image(raw_bytes)?,
        }),
        PrepMode::CenterPng1024 => {
            if matches!(role, ImageRole::Center) {
                encode_custom_image(raw_bytes, 1024, image::ImageFormat::Png, "image/png")
            } else {
                Ok(EncodedImage {
                    mime: "image/jpeg",
                    b64: inference::prepare_image(raw_bytes)?,
                })
            }
        }
        PrepMode::AllPng1024 => {
            encode_custom_image(raw_bytes, 1024, image::ImageFormat::Png, "image/png")
        }
    }
}

pub fn parse_prediction(json: &serde_json::Value) -> Result<(BaseMood, MoodIntensity), String> {
    match parse_mood_intensity_response(json) {
        Ok(tag) => Ok((tag.mood, tag.intensity)),
        Err(_) => {
            let raw_content =
                extract_content(json).ok_or_else(|| "No content in response".to_string())?;
            let raw_content = raw_content.to_lowercase();
            let cleaned = if let Some(pos) = raw_content.find("</think>") {
                &raw_content[pos + 8..]
            } else {
                raw_content.as_str()
            };
            let mut best: Option<(BaseMood, usize)> = None;
            for &mood in &BaseMood::ALL {
                if let Some(pos) = cleaned.rfind(mood.as_str()) {
                    if best.is_none() || pos > best.unwrap().1 {
                        best = Some((mood, pos));
                    }
                }
            }
            best.map(|(mood, _)| (mood, MoodIntensity::Medium))
                .ok_or_else(|| {
                    format!(
                        "No mood found in response: '{}'",
                        &cleaned[..cleaned.len().min(120)]
                    )
                })
        }
    }
}

pub fn build_vote_stats(window_predictions: &[WindowPrediction], page: u32) -> VoteStats {
    let mut counts = [0u32; 8];
    let mut weighted = [0.0f64; 8];
    let mut intensity_sum = [0u32; 8];
    let mut intensity_count = [0u32; 8];
    let mut center_vote = None;
    let mut center_intensity = None;

    for vote in window_predictions
        .iter()
        .filter(|vote| vote.members.contains(&page))
    {
        let idx = vote.mood.index();
        counts[idx] += 1;
        let role_weight = if vote.center == page { 1.35 } else { 1.0 };
        weighted[idx] += role_weight;
        intensity_sum[idx] += vote.intensity.as_u8() as u32;
        intensity_count[idx] += 1;
        if vote.center == page {
            center_vote = Some(vote.mood);
            center_intensity = Some(vote.intensity);
        }
    }

    let mut sorted_scores = weighted
        .iter()
        .enumerate()
        .map(|(idx, score)| (idx, *score))
        .collect::<Vec<_>>();
    sorted_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    VoteStats {
        counts,
        weighted,
        intensity_sum,
        intensity_count,
        center_vote,
        center_intensity,
        winner_score: sorted_scores
            .first()
            .map(|(_, score)| *score)
            .unwrap_or(0.0),
        runner_up_score: sorted_scores.get(1).map(|(_, score)| *score).unwrap_or(0.0),
    }
}

pub fn aggregate_majority(
    page_order: &[u32],
    window_predictions: &[WindowPrediction],
) -> BTreeMap<u32, AggregatedPrediction> {
    let mut predictions = BTreeMap::new();
    for &page in page_order {
        let stats = build_vote_stats(window_predictions, page);
        predictions.insert(page, aggregate_prediction(&stats));
    }
    predictions
}

pub fn aggregate_prediction(stats: &VoteStats) -> AggregatedPrediction {
    let mut sorted = stats
        .counts
        .iter()
        .enumerate()
        .map(|(mood_idx, count)| (mood_idx, *count))
        .collect::<Vec<_>>();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    let winner_idx = if sorted.first().map(|(_, c)| *c).unwrap_or(0) == 0 {
        BaseMood::Tension.index()
    } else if sorted.len() > 1 && sorted[0].1 == sorted[1].1 {
        stats
            .center_vote
            .map(|mood| mood.index())
            .unwrap_or(sorted[0].0)
    } else {
        sorted[0].0
    };

    let mood = BaseMood::ALL[winner_idx];
    let intensity = average_intensity(stats, winner_idx).unwrap_or(MoodIntensity::Medium);

    AggregatedPrediction {
        mood,
        intensity,
        winner_score: stats.winner_score,
        runner_up_score: stats.runner_up_score,
        scores: normalized_scores(stats),
        narrative_role: infer_narrative_role(mood, intensity),
        note: None,
    }
}

pub fn plan_selective_repairs(
    page_order: &[u32],
    base_predictions: &BTreeMap<u32, AggregatedPrediction>,
) -> Vec<RepairPlan> {
    let ordered_moods = page_order
        .iter()
        .map(|page| base_predictions.get(page).map(|prediction| prediction.mood))
        .collect::<Option<Vec<_>>>();
    let Some(ordered_moods) = ordered_moods else {
        return Vec::new();
    };

    let runs = build_mood_runs(&ordered_moods);
    let mut plans = Vec::new();

    for (run_idx, run) in runs.iter().copied().enumerate() {
        let mood = ordered_moods[run.start];
        let next_run = runs.get(run_idx + 1).copied();

        if mood == BaseMood::Comedy && run.len() >= 4 {
            for local_idx in [run.start, run.end] {
                let page = page_order[local_idx];
                plans.push(RepairPlan::Single {
                    key: format!("comedy_edge:{page}"),
                    page,
                    prompt_kind: SelectivePrompt::Focus,
                    reason: "comedy_edge",
                    accept: RepairAcceptSingle::MysteryOrTension,
                });
            }
        }

        if mood == BaseMood::Tension && run.len() >= 10 {
            let first_page = page_order[run.start];
            let second_page = page_order[run.start + 1];
            plans.push(RepairPlan::Pair {
                key: format!("tension_open:{first_page}-{second_page}"),
                first_page,
                second_page,
                prompt_kind: SelectivePrompt::Narrative,
                reason: "tension_run_open_mystery",
                accept: RepairAcceptPair::BothMystery,
            });

            let last_first = page_order[run.end.saturating_sub(1)];
            let last_second = page_order[run.end];
            plans.push(RepairPlan::Pair {
                key: format!("tension_close:{last_first}-{last_second}"),
                first_page: last_first,
                second_page: last_second,
                prompt_kind: SelectivePrompt::Narrative,
                reason: "tension_run_close_epic",
                accept: RepairAcceptPair::BothEpic,
            });
        }

        if mood == BaseMood::Epic && run.len() <= 4 && run.start > 0 {
            let page = page_order[run.start];
            plans.push(RepairPlan::Single {
                key: format!("short_epic_open:{page}"),
                page,
                prompt_kind: SelectivePrompt::Narrative,
                reason: "short_epic_open_tension",
                accept: RepairAcceptSingle::Tension,
            });
        }

        if mood == BaseMood::Epic
            && run.len() >= 4
            && next_run
                .map(|next| ordered_moods[next.start] == BaseMood::Tension && next.len() >= 10)
                .unwrap_or(false)
        {
            let penultimate = page_order[run.end.saturating_sub(1)];
            plans.push(RepairPlan::Single {
                key: format!("epic_bridge_penultimate:{penultimate}"),
                page: penultimate,
                prompt_kind: SelectivePrompt::Focus,
                reason: "epic_bridge_penultimate_mystery",
                accept: RepairAcceptSingle::Mystery,
            });

            let end_page = page_order[run.end];
            plans.push(RepairPlan::Single {
                key: format!("epic_bridge_end:{end_page}"),
                page: end_page,
                prompt_kind: SelectivePrompt::Narrative,
                reason: "epic_bridge_final_mystery",
                accept: RepairAcceptSingle::Mystery,
            });
        }

        if mood == BaseMood::Epic && run.len() >= 8 {
            let left_page = page_order[run.end.saturating_sub(1)];
            let right_page = page_order[run.end];
            plans.push(RepairPlan::Pair {
                key: format!("long_epic_tail:{left_page}-{right_page}"),
                first_page: left_page,
                second_page: right_page,
                prompt_kind: SelectivePrompt::Focus,
                reason: "long_epic_tail_tension",
                accept: RepairAcceptPair::BothTension,
            });
        }
    }

    plans
}

pub fn apply_action_bridge_holds(
    page_order: &[u32],
    base_predictions: &BTreeMap<u32, AggregatedPrediction>,
    effective_predictions: &mut BTreeMap<u32, AggregatedPrediction>,
) {
    let ordered_moods = page_order
        .iter()
        .map(|page| base_predictions.get(page).map(|prediction| prediction.mood))
        .collect::<Option<Vec<_>>>();
    let Some(ordered_moods) = ordered_moods else {
        return;
    };

    let runs = build_mood_runs(&ordered_moods);

    for run_idx in 1..runs.len().saturating_sub(1) {
        let previous = runs[run_idx - 1];
        let current = runs[run_idx];
        let next = runs[run_idx + 1];

        if ordered_moods[current.start] != BaseMood::Tension {
            continue;
        }
        if ordered_moods[previous.start] != BaseMood::Epic
            || ordered_moods[next.start] != BaseMood::Epic
        {
            continue;
        }
        if !(4..=6).contains(&current.len()) || previous.len() > 3 || next.len() < 4 {
            continue;
        }

        for local_idx in current.start..=(current.start + 1).min(current.end) {
            let page = page_order[local_idx];
            let Some(original) = effective_predictions.get(&page).cloned() else {
                continue;
            };
            if original.mood != BaseMood::Tension {
                continue;
            }
            let mut replacement = original;
            replacement.mood = BaseMood::Epic;
            replacement.scores = MoodScores::from_single(BaseMood::Epic);
            replacement.narrative_role =
                infer_narrative_role(BaseMood::Epic, replacement.intensity);
            replacement.note = Some("action_bridge_hold_epic".to_string());
            effective_predictions.insert(page, replacement);
        }
    }
}

pub fn accepts_single(prediction: &AggregatedPrediction, rule: RepairAcceptSingle) -> bool {
    match rule {
        RepairAcceptSingle::Tension => prediction.mood == BaseMood::Tension,
        RepairAcceptSingle::Mystery => prediction.mood == BaseMood::Mystery,
        RepairAcceptSingle::MysteryOrTension => {
            matches!(prediction.mood, BaseMood::Mystery | BaseMood::Tension)
        }
    }
}

pub fn accepts_pair(
    first: &AggregatedPrediction,
    second: &AggregatedPrediction,
    rule: RepairAcceptPair,
) -> bool {
    match rule {
        RepairAcceptPair::BothMystery => {
            first.mood == BaseMood::Mystery && second.mood == BaseMood::Mystery
        }
        RepairAcceptPair::BothEpic => first.mood == BaseMood::Epic && second.mood == BaseMood::Epic,
        RepairAcceptPair::BothTension => {
            first.mood == BaseMood::Tension && second.mood == BaseMood::Tension
        }
    }
}

pub fn average_intensity(stats: &VoteStats, mood_idx: usize) -> Option<MoodIntensity> {
    let count = stats.intensity_count[mood_idx];
    if count == 0 {
        return None;
    }
    Some(MoodIntensity::from_u8(
        ((stats.intensity_sum[mood_idx] as f64 / count as f64).round() as u8).clamp(1, 3),
    ))
}

pub fn infer_narrative_role(mood: BaseMood, intensity: MoodIntensity) -> NarrativeRole {
    match (mood, intensity) {
        (BaseMood::Epic, MoodIntensity::Medium | MoodIntensity::High) => NarrativeRole::Climax,
        (
            BaseMood::Tension | BaseMood::Mystery | BaseMood::Horror,
            MoodIntensity::Medium | MoodIntensity::High,
        ) => NarrativeRole::Escalation,
        (BaseMood::Peaceful, MoodIntensity::Low)
        | (BaseMood::Sadness, MoodIntensity::Low | MoodIntensity::Medium) => {
            NarrativeRole::DeEscalation
        }
        _ => NarrativeRole::Continuation,
    }
}

fn normalized_scores(stats: &VoteStats) -> MoodScores {
    let mut scores = MoodScores::new();
    let sum = stats.weighted.iter().sum::<f64>().max(0.001);
    for mood in BaseMood::ALL {
        scores.set(mood, (stats.weighted[mood.index()] / sum) as f32);
    }
    scores
}

fn build_mood_runs(moods: &[BaseMood]) -> Vec<MoodRun> {
    if moods.is_empty() {
        return Vec::new();
    }

    let mut runs = Vec::new();
    let mut start = 0usize;
    while start < moods.len() {
        let mut end = start;
        while end + 1 < moods.len() && moods[end + 1] == moods[start] {
            end += 1;
        }
        runs.push(MoodRun { start, end });
        start = end + 1;
    }
    runs
}

fn encode_custom_image(
    raw_bytes: &[u8],
    max_dim: u32,
    format: image::ImageFormat,
    mime: &'static str,
) -> Result<EncodedImage, String> {
    let img = image::load_from_memory(raw_bytes)
        .map_err(|e| format!("Failed to decode source image: {}", e))?;
    let (w, h) = img.dimensions();
    let resized = if w > max_dim || h > max_dim {
        let scale = max_dim as f64 / w.max(h) as f64;
        let new_w = (w as f64 * scale).round() as u32;
        let new_h = (h as f64 * scale).round() as u32;
        img.resize(new_w, new_h, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };

    let mut cursor = std::io::Cursor::new(Vec::new());
    resized
        .write_to(&mut cursor, format)
        .map_err(|e| format!("Failed to encode image: {}", e))?;
    Ok(EncodedImage {
        mime,
        b64: base64::engine::general_purpose::STANDARD.encode(cursor.into_inner()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aggregated_prediction(mood: BaseMood) -> AggregatedPrediction {
        AggregatedPrediction {
            mood,
            intensity: MoodIntensity::High,
            winner_score: 1.0,
            runner_up_score: 0.0,
            scores: MoodScores::from_single(mood),
            narrative_role: infer_narrative_role(mood, MoodIntensity::High),
            note: None,
        }
    }

    #[test]
    fn plans_tension_open_and_close_repairs() {
        let page_order: Vec<u32> = (0..12).collect();
        let base = page_order
            .iter()
            .map(|&page| {
                let mood = if page < 10 {
                    BaseMood::Tension
                } else {
                    BaseMood::Epic
                };
                (page, aggregated_prediction(mood))
            })
            .collect::<BTreeMap<_, _>>();

        let plans = plan_selective_repairs(&page_order, &base);
        assert!(plans.iter().any(|plan| matches!(
            plan,
            RepairPlan::Pair {
                reason: "tension_run_open_mystery",
                ..
            }
        )));
        assert!(plans.iter().any(|plan| matches!(
            plan,
            RepairPlan::Pair {
                reason: "tension_run_close_epic",
                ..
            }
        )));
    }

    #[test]
    fn action_bridge_hold_promotes_first_two_tension_pages() {
        let page_order: Vec<u32> = (0..11).collect();
        let base_moods = [
            BaseMood::Epic,
            BaseMood::Epic,
            BaseMood::Tension,
            BaseMood::Tension,
            BaseMood::Tension,
            BaseMood::Tension,
            BaseMood::Tension,
            BaseMood::Epic,
            BaseMood::Epic,
            BaseMood::Epic,
            BaseMood::Epic,
        ];
        let base = page_order
            .iter()
            .zip(base_moods)
            .map(|(&page, mood)| (page, aggregated_prediction(mood)))
            .collect::<BTreeMap<_, _>>();
        let mut effective = base.clone();

        apply_action_bridge_holds(&page_order, &base, &mut effective);

        assert_eq!(
            effective.get(&2).map(|prediction| prediction.mood),
            Some(BaseMood::Epic)
        );
        assert_eq!(
            effective.get(&3).map(|prediction| prediction.mood),
            Some(BaseMood::Epic)
        );
    }
}
