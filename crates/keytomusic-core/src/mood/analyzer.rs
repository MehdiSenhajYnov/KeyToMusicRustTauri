//! One request = a run of consecutive pages, split into scenes that each get
//! one soundtrack mood. Scenes, not single pages: the music should follow the
//! story, not flicker with every panel.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::{json, Value};

use super::provider::{self, Part, ProviderError, ProviderSettings};
use crate::types::{BaseMood, MoodIntensity};

/// Pages sent per request: enough story for the model to see scene changes,
/// small enough to answer in a few seconds.
pub const BATCH_PAGES: usize = 10;
/// Already-read pages shown before a batch so scenes continue across batches.
pub const CONTEXT_PAGES: usize = 2;

/// The mood decided for one page.
#[derive(Clone, Debug, PartialEq)]
pub struct PageLabel {
    pub mood: BaseMood,
    pub intensity: MoodIntensity,
    /// What happens in the scene, a few words (shown in the UIs, fed back as context).
    pub summary: String,
}

/// What a request knows about the pages around the batch.
#[derive(Clone, Debug, Default)]
pub struct BatchContext {
    /// Pages just before the batch: (page, image, label if already decided).
    pub before: Vec<(u32, Arc<String>, Option<PageLabel>)>,
    /// Label of the page right after the batch, if already decided.
    pub after: Option<(u32, PageLabel)>,
    pub total_pages: Option<u32>,
    pub title: Option<String>,
}

pub const SYSTEM_PROMPT: &str = "You pick the background music for someone reading a manga, manhwa or webtoon chapter. \
You see consecutive pages in reading order. Split them into scenes and give each scene ONE soundtrack mood.

Moods:
- epic: climax, power unleashed, decisive blow, triumph, heroic resolve paying off
- tension: build-up, danger, standoff, fight still undecided, pressure, suspenseful action
- sadness: loss, grief, tears, regret, painful memories, aftermath
- comedy: gags, comic relief, silly reactions, chibi faces, banter played for laughs
- romance: affection, blushing, intimacy, tender moments between characters
- horror: fear, gore, monsters, nightmare imagery, disturbing shock
- peaceful: calm daily life, quiet conversation, rest, travel, contemplation
- mystery: secrets, scheming, ominous reveal, investigation, foreshadowing

Intensity: 1 = subtle, 2 = clear, 3 = overwhelming.

Rules:
- Music must not flicker. A scene normally lasts at least 3 pages; start a new scene only when the mood of the story really changes (new location, new event, shift in tone). A single funny panel inside a fight stays in the fight's scene.
- Read the dialogue and sound effects as well as the art. Characters talking calmly in a dangerous situation is still tension.
- Fights are tension while undecided and become epic only at the climax or payoff.
- Covers, credits, translator notes and ads take the mood of the scene next to them.
- Every requested page belongs to exactly one scene; scenes are in reading order, contiguous, and use the page numbers given.
- summary: at most 12 words describing what happens.
Answer with JSON only.";

/// JSON schema of the answer (strict structured output).
pub fn schema() -> Value {
    let moods: Vec<&str> = BaseMood::ALL.iter().map(|m| m.as_str()).collect();
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["scenes"],
        "properties": {
            "scenes": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["first_page", "last_page", "mood", "intensity", "summary"],
                    "properties": {
                        "first_page": { "type": "integer" },
                        "last_page": { "type": "integer" },
                        "mood": { "type": "string", "enum": moods },
                        "intensity": { "type": "integer", "enum": [1, 2, 3] },
                        "summary": { "type": "string" }
                    }
                }
            }
        }
    })
}

fn describe(label: &PageLabel) -> String {
    format!(
        "{} {} ({})",
        label.mood.as_str(),
        label.intensity.as_u8(),
        label.summary
    )
}

/// The user message: context pages, then the pages to classify. Page numbers
/// shown to the model are 1-based.
pub fn build_parts(pages: &[(u32, Arc<String>)], context: &BatchContext) -> Vec<Part> {
    let first = pages.first().map(|(p, _)| p + 1).unwrap_or(1);
    let last = pages.last().map(|(p, _)| p + 1).unwrap_or(first);
    let mut parts = Vec::new();

    let mut intro = String::new();
    if let Some(title) = context.title.as_deref().filter(|t| !t.trim().is_empty()) {
        intro.push_str(&format!("Chapter: {}\n", title.trim()));
    }
    if let Some(total) = context.total_pages {
        intro.push_str(&format!("The chapter has {total} pages.\n"));
    }
    if !intro.is_empty() {
        parts.push(Part::Text(intro));
    }

    if !context.before.is_empty() {
        parts.push(Part::Text(
            "Earlier pages, for context only (do not include them in your scenes):".into(),
        ));
        for (page, image, label) in &context.before {
            let text = match label {
                Some(label) => format!("Page {} — already scored: {}", page + 1, describe(label)),
                None => format!("Page {}", page + 1),
            };
            parts.push(Part::Text(text));
            parts.push(Part::Image(image.clone()));
        }
    }

    parts.push(Part::Text(format!(
        "Pages to split into scenes: {first} to {last}."
    )));
    for (page, image) in pages {
        parts.push(Part::Text(format!("Page {}", page + 1)));
        parts.push(Part::Image(image.clone()));
    }
    if let Some((page, label)) = &context.after {
        parts.push(Part::Text(format!(
            "For continuity, page {} (right after these) was scored: {}.",
            page + 1,
            describe(label)
        )));
    }
    parts.push(Part::Text(format!(
        "Return the scenes covering every page from {first} to {last}."
    )));
    parts
}

/// Turn the answer into one label per requested page (0-based). Scenes outside
/// the batch are clipped; pages no scene covers take the nearest scene's mood.
pub fn parse_scenes(answer: &Value, pages: &[u32]) -> Result<BTreeMap<u32, PageLabel>, String> {
    let scenes = answer
        .get("scenes")
        .and_then(|s| s.as_array())
        .ok_or_else(|| "Réponse sans scènes".to_string())?;

    let mut covered: BTreeMap<u32, PageLabel> = BTreeMap::new();
    for scene in scenes {
        let Some(mood) = scene
            .get("mood")
            .and_then(|m| m.as_str())
            .and_then(|m| BaseMood::from_str_opt(m.trim().to_ascii_lowercase().as_str()))
        else {
            continue;
        };
        let number = |key: &str| {
            scene
                .get(key)
                .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
        };
        let (Some(first), Some(last)) = (number("first_page"), number("last_page")) else {
            continue;
        };
        let (first, last) = (first.min(last), first.max(last));
        let intensity = MoodIntensity::from_u8(number("intensity").unwrap_or(2).clamp(1, 3) as u8);
        let summary: String = scene
            .get("summary")
            .and_then(|s| s.as_str())
            .unwrap_or_default()
            .trim()
            .chars()
            .take(120)
            .collect();
        for &page in pages {
            let shown = page as i64 + 1;
            if (first..=last).contains(&shown) {
                covered.entry(page).or_insert_with(|| PageLabel {
                    mood,
                    intensity,
                    summary: summary.clone(),
                });
            }
        }
    }
    if covered.is_empty() {
        return Err("Le modèle n'a classé aucune des pages demandées".into());
    }

    let mut labels = BTreeMap::new();
    for &page in pages {
        let label = covered.get(&page).cloned().unwrap_or_else(|| {
            // Nearest covered page, preferring the one before.
            let before = covered.range(..page).next_back();
            let after = covered.range(page..).next();
            match (before, after) {
                (Some((b, lb)), Some((a, la))) => {
                    if page - b <= a - page {
                        lb.clone()
                    } else {
                        la.clone()
                    }
                }
                (Some((_, l)), None) | (None, Some((_, l))) => l.clone(),
                (None, None) => unreachable!("covered is not empty"),
            }
        });
        labels.insert(page, label);
    }
    Ok(labels)
}

/// Ask the model about `pages` (consecutive, 0-based, with their images).
pub async fn analyze(
    settings: &ProviderSettings,
    pages: &[(u32, Arc<String>)],
    context: &BatchContext,
) -> Result<BTreeMap<u32, PageLabel>, ProviderError> {
    let parts = build_parts(pages, context);
    let answer =
        provider::chat_json(settings, SYSTEM_PROMPT, &parts, "manga_scenes", &schema()).await?;
    let numbers: Vec<u32> = pages.iter().map(|(p, _)| *p).collect();
    parse_scenes(&answer, &numbers).map_err(ProviderError::retryable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenes_become_page_labels() {
        let answer = json!({ "scenes": [
            { "first_page": 11, "last_page": 14, "mood": "tension", "intensity": 2, "summary": "standoff" },
            { "first_page": 15, "last_page": 20, "mood": "epic", "intensity": 3, "summary": "final blow" }
        ]});
        let pages: Vec<u32> = (10..20).collect();
        let labels = parse_scenes(&answer, &pages).unwrap();
        assert_eq!(labels.len(), 10);
        assert_eq!(labels[&10].mood, BaseMood::Tension);
        assert_eq!(labels[&13].mood, BaseMood::Tension);
        assert_eq!(labels[&14].mood, BaseMood::Epic);
        assert_eq!(labels[&14].intensity, MoodIntensity::High);
        assert_eq!(labels[&19].summary, "final blow");
    }

    #[test]
    fn gaps_take_the_nearest_scene_and_outside_pages_are_ignored() {
        let answer = json!({ "scenes": [
            { "first_page": 1, "last_page": 2, "mood": "comedy", "intensity": 1, "summary": "" },
            { "first_page": 5, "last_page": 9, "mood": "Sadness", "intensity": 7, "summary": "" },
            { "first_page": 3, "last_page": 3, "mood": "unknown", "intensity": 2, "summary": "" }
        ]});
        // Pages 0-4 are shown as 1-5; shown 3 and 4 have no valid scene.
        let labels = parse_scenes(&answer, &[0, 1, 2, 3, 4]).unwrap();
        assert_eq!(labels[&1].mood, BaseMood::Comedy);
        assert_eq!(labels[&2].mood, BaseMood::Comedy);
        assert_eq!(labels[&3].mood, BaseMood::Sadness);
        assert_eq!(labels[&4].mood, BaseMood::Sadness);
        assert_eq!(labels[&4].intensity, MoodIntensity::High);
    }

    #[test]
    fn an_answer_covering_nothing_is_an_error() {
        let answer = json!({ "scenes": [
            { "first_page": 40, "last_page": 45, "mood": "epic", "intensity": 2, "summary": "" }
        ]});
        assert!(parse_scenes(&answer, &[0, 1]).is_err());
        assert!(parse_scenes(&json!({}), &[0]).is_err());
    }

    #[test]
    fn prompt_numbers_pages_from_one_and_marks_context() {
        let img = Arc::new("AAAA".to_string());
        let context = BatchContext {
            before: vec![(
                9,
                img.clone(),
                Some(PageLabel {
                    mood: BaseMood::Tension,
                    intensity: MoodIntensity::Medium,
                    summary: "ambush".into(),
                }),
            )],
            after: None,
            total_pages: Some(40),
            title: Some("Chapter 12".into()),
        };
        let parts = build_parts(&[(10, img.clone()), (11, img)], &context);
        let text: Vec<String> = parts
            .iter()
            .filter_map(|p| match p {
                Part::Text(t) => Some(t.clone()),
                Part::Image(_) => None,
            })
            .collect();
        assert!(text
            .iter()
            .any(|t| t.contains("Page 10 — already scored: tension 2 (ambush)")));
        assert!(text
            .iter()
            .any(|t| t.contains("Pages to split into scenes: 11 to 12.")));
        assert_eq!(
            parts.iter().filter(|p| matches!(p, Part::Image(_))).count(),
            3
        );
    }
}
