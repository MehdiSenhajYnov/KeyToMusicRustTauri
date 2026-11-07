//! Default moods for the keys nobody tagged.
//!
//! The model reads what each key holds (its name, the titles of its sounds,
//! their artist and folder) and answers a mood and the intensity the piece
//! fits. It knows most film, game and anime scores, which the keyword guess
//! of [`placement`](crate::session::placement) cannot ("Unravel",
//! "Kamado Tanjiro no Uta"). Without a model, or for what it leaves out, the
//! keyword guess still applies. The user reviews the suggestions before they
//! are applied.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::Serialize;
use serde_json::{json, Value};

use super::provider::{self, Part, ProviderError, ProviderSettings};
use crate::audio::analysis::read_audio_metadata_tags;
use crate::session::placement::{self, SoundHint, SoundRole};
use crate::types::{BaseMood, KeyBinding, MoodIntensity, Profile, SoundSource};

/// Keys per request: one answer stays short and fast.
pub const BATCH_KEYS: usize = 60;
/// Sounds of a key shown to the model (a family is told by a few).
const TITLES_PER_KEY: usize = 4;
/// Keys the user already tagged, shown as examples of their taste.
const EXAMPLES: usize = 24;

/// A key without a mood, on a music track.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyCandidate {
    pub key_code: String,
    pub track_id: String,
    /// What the UI shows: the key's name, else its first sound's.
    pub label: String,
    pub name: Option<String>,
    pub titles: Vec<String>,
    pub artists: Vec<String>,
    pub folders: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SuggestionSource {
    Model,
    Keywords,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Certainty {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoodSuggestion {
    pub key_code: String,
    pub track_id: String,
    pub label: String,
    pub mood: BaseMood,
    /// Minimum scene intensity that plays the key (Low: any).
    pub intensity: MoodIntensity,
    pub certainty: Certainty,
    pub source: SuggestionSource,
}

impl MoodSuggestion {
    /// Worth applying without a second look.
    pub fn recommended(&self) -> bool {
        self.certainty >= Certainty::Medium
    }
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoTagResult {
    pub suggestions: Vec<MoodSuggestion>,
    /// Keys left without a mood (not music, or nothing to go on).
    pub unknown: Vec<String>,
    /// The model was asked (and answered at least in part).
    pub used_model: bool,
    /// Why the model could not help, when it was meant to.
    pub model_error: Option<String>,
}

// ── What the keys hold ──

fn sound_path(source: &SoundSource) -> &str {
    match source {
        SoundSource::Local { path } => path,
        SoundSource::YouTube { cached_path, .. } => cached_path,
    }
}

/// A folder that says something ("Combat", "Ambiances"), not the app's own
/// storage ("sounds", an id).
fn telling_folder(source: &SoundSource) -> Option<String> {
    let SoundSource::Local { path } = source else {
        return None;
    };
    let name = Path::new(path)
        .parent()?
        .file_name()?
        .to_str()?
        .trim()
        .to_string();
    let lower = name.to_lowercase();
    let generic = [
        "sounds",
        "imported_sounds",
        "cache",
        "youtube",
        "music",
        "musique",
        "downloads",
        "téléchargements",
    ];
    let id_like = name.len() >= 16 && name.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
    (!name.is_empty() && !id_like && !generic.contains(&lower.as_str())).then_some(name)
}

fn key_label(p: &Profile, b: &KeyBinding) -> String {
    b.name
        .clone()
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| {
            b.sound_ids
                .first()
                .and_then(|id| p.sounds.iter().find(|s| &s.id == id))
                .map(|s| s.name.clone())
                .unwrap_or_else(|| b.key_code.clone())
        })
}

/// Untagged keys on music tracks that hold music (effects and ambiences
/// would fire with Mood AI).
pub fn candidates(p: &Profile) -> Vec<KeyCandidate> {
    let mut out = Vec::new();
    for b in &p.key_bindings {
        if b.mood.is_some() || b.sound_ids.is_empty() {
            continue;
        }
        if placement::track_role(p, &b.track_id) != SoundRole::Music {
            continue;
        }
        let sounds: Vec<_> = b
            .sound_ids
            .iter()
            .filter_map(|id| p.sounds.iter().find(|s| &s.id == id))
            .collect();
        let music = sounds
            .iter()
            .filter(|s| {
                placement::guess_role(&SoundHint {
                    name: s.name.clone(),
                    folder: telling_folder(&s.source),
                    duration: s.duration,
                    ..Default::default()
                }) == SoundRole::Music
            })
            .count();
        // A key of effects stays untagged.
        if sounds.is_empty() || music * 2 < sounds.len() {
            continue;
        }
        let mut folders: Vec<String> = Vec::new();
        for f in sounds.iter().filter_map(|s| telling_folder(&s.source)) {
            if !folders.contains(&f) {
                folders.push(f);
            }
        }
        out.push(KeyCandidate {
            key_code: b.key_code.clone(),
            track_id: b.track_id.clone(),
            label: key_label(p, b),
            name: b.name.clone().filter(|n| !n.trim().is_empty()),
            titles: sounds
                .iter()
                .take(TITLES_PER_KEY)
                .map(|s| s.name.clone())
                .collect(),
            artists: Vec::new(),
            folders,
        });
    }
    out
}

/// Artist tags of local files: "Track 3" by "Hiroyuki Sawano" says a lot.
pub fn read_artists(p: &Profile, candidates: &mut [KeyCandidate]) {
    for c in candidates.iter_mut() {
        let Some(b) = p
            .key_bindings
            .iter()
            .find(|b| b.key_code == c.key_code && b.track_id == c.track_id)
        else {
            continue;
        };
        for s in b
            .sound_ids
            .iter()
            .take(TITLES_PER_KEY)
            .filter_map(|id| p.sounds.iter().find(|s| &s.id == id))
        {
            if !matches!(s.source, SoundSource::Local { .. }) {
                continue;
            }
            let Some(artist) =
                read_audio_metadata_tags(sound_path(&s.source)).and_then(|t| t.artist)
            else {
                continue;
            };
            let artist = artist.trim().to_string();
            let known = c.artists.contains(&artist)
                || s.name.to_lowercase().contains(&artist.to_lowercase());
            if !artist.is_empty() && !known {
                c.artists.push(artist);
            }
        }
    }
}

// ── Keyword guess ──

/// The mood the titles, key name or folders name, without a model.
pub fn keyword_suggestion(c: &KeyCandidate) -> Option<MoodSuggestion> {
    fn vote(votes: &mut Vec<(BaseMood, usize)>, mood: Option<BaseMood>) {
        if let Some(mood) = mood {
            match votes.iter_mut().find(|(m, _)| *m == mood) {
                Some((_, n)) => *n += 1,
                None => votes.push((mood, 1)),
            }
        }
    }
    let mut votes: Vec<(BaseMood, usize)> = Vec::new();
    for title in &c.titles {
        vote(&mut votes, placement::guess_title_mood(title));
    }
    vote(
        &mut votes,
        c.name.as_deref().and_then(placement::guess_mood),
    );
    if votes.is_empty() {
        for folder in &c.folders {
            vote(&mut votes, placement::guess_mood(folder));
        }
    }
    votes.sort_by_key(|v| std::cmp::Reverse(v.1));
    let mood = match votes.as_slice() {
        [] => return None,
        [(mood, _)] => *mood,
        [(mood, best), (_, next), ..] if best > next => *mood,
        _ => return None,
    };
    Some(MoodSuggestion {
        key_code: c.key_code.clone(),
        track_id: c.track_id.clone(),
        label: c.label.clone(),
        mood,
        intensity: MoodIntensity::Low,
        certainty: Certainty::Low,
        source: SuggestionSource::Keywords,
    })
}

// ── The model ──

pub const SYSTEM_PROMPT: &str = "You tag background music for a soundboard used while reading manga, manhwa or webtoons. \
When the reader reaches a scene of a given mood, the keys tagged with that mood play.

For each key you get what it holds: the key's name if the user gave one, the titles of its sounds (often \"Work — Piece\" or a YouTube title), and sometimes artists and folders. \
Use what you know about the piece (film, game, anime and trailer music) and the words in its title.

Moods (scenes the music should accompany):
- epic: climax, power unleashed, decisive blow, triumph, heroic resolve
- tension: build-up, danger, standoff, chase, undecided fight, suspense
- sadness: loss, grief, regret, painful memories, melancholy
- comedy: gags, comic relief, silly or playful moments
- romance: affection, tenderness, love, warm emotional bonds
- horror: fear, dread, monsters, disturbing shock
- peaceful: calm daily life, quiet conversation, rest, travel, contemplation
- mystery: secrets, scheming, investigation, ominous reveal, deduction

intensity: the least intense scene the piece suits. 1 = fits any scene of that mood (subtle background), \
2 = clear moments, 3 = only the strongest moments (a climax, a breakdown, a full-orchestra peak).

certainty: high = you know the piece or the title is explicit; medium = a reasonable guess from the title, \
artist or genre; low = little to go on. Use mood \"none\" for sound effects, ambience recordings, \
or when there is nothing to go on at all.

Answer every key, with its id, as JSON only.";

pub fn schema() -> Value {
    let mut moods: Vec<&str> = BaseMood::ALL.iter().map(|m| m.as_str()).collect();
    moods.push("none");
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["keys"],
        "properties": {
            "keys": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["id", "mood", "intensity", "certainty"],
                    "properties": {
                        "id": { "type": "string" },
                        "mood": { "type": "string", "enum": moods },
                        "intensity": { "type": "integer", "enum": [1, 2, 3] },
                        "certainty": { "type": "string", "enum": ["low", "medium", "high"] }
                    }
                }
            }
        }
    })
}

fn quoted(items: &[String]) -> String {
    items
        .iter()
        .map(|t| format!("\"{}\"", t.replace('"', "'")))
        .collect::<Vec<_>>()
        .join("; ")
}

fn describe(c: &KeyCandidate) -> String {
    let mut line = String::new();
    if let Some(name) = &c.name {
        line.push_str(&format!(" | key name: \"{}\"", name.replace('"', "'")));
    }
    line.push_str(&format!(" | sounds: {}", quoted(&c.titles)));
    if !c.artists.is_empty() {
        line.push_str(&format!(" | artists: {}", quoted(&c.artists)));
    }
    if !c.folders.is_empty() {
        line.push_str(&format!(" | folders: {}", quoted(&c.folders)));
    }
    line
}

/// Keys the user tagged: how they hear their own music.
fn examples(p: &Profile) -> Vec<String> {
    let mut seen = HashSet::new();
    p.key_bindings
        .iter()
        .filter(|b| b.mood.is_some() && !b.sound_ids.is_empty())
        .filter(|b| seen.insert(b.key_code.clone()))
        .take(EXAMPLES)
        .map(|b| {
            let titles: Vec<String> = b
                .sound_ids
                .iter()
                .take(2)
                .filter_map(|id| p.sounds.iter().find(|s| &s.id == id))
                .map(|s| s.name.clone())
                .collect();
            format!(
                "{} → {} {}",
                quoted(&titles),
                b.mood.map(|m| m.as_str()).unwrap_or_default(),
                b.mood_intensity.map_or(1, |i| i.as_u8())
            )
        })
        .collect()
}

pub fn build_prompt(batch: &[(String, &KeyCandidate)], examples: &[String]) -> String {
    let mut text = String::new();
    if !examples.is_empty() {
        text.push_str("Keys the user already tagged (mood, intensity), to match their taste:\n");
        for e in examples {
            text.push_str(&format!("- {e}\n"));
        }
        text.push('\n');
    }
    text.push_str("Keys to tag:\n");
    for (id, c) in batch {
        text.push_str(&format!("{id}{}\n", describe(c)));
    }
    text
}

fn certainty(text: &str) -> Certainty {
    match text {
        "high" => Certainty::High,
        "medium" => Certainty::Medium,
        _ => Certainty::Low,
    }
}

/// The model's answers, by id. `None` for keys it judged not to be music.
pub fn parse_answer(
    answer: &Value,
) -> HashMap<String, Option<(BaseMood, MoodIntensity, Certainty)>> {
    let mut out = HashMap::new();
    for item in answer
        .get("keys")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(id) = item.get("id").and_then(Value::as_str) else {
            continue;
        };
        let mood = item.get("mood").and_then(Value::as_str).unwrap_or("none");
        let parsed = BaseMood::from_str_opt(mood).map(|mood| {
            let intensity = item.get("intensity").and_then(Value::as_u64).unwrap_or(1);
            (
                mood,
                MoodIntensity::from_u8(intensity as u8),
                certainty(
                    item.get("certainty")
                        .and_then(Value::as_str)
                        .unwrap_or("low"),
                ),
            )
        });
        out.insert(id.to_string(), parsed);
    }
    out
}

/// Ask the model about every candidate, `BATCH_KEYS` at a time. Answers
/// gathered before an error are kept.
pub async fn classify(
    settings: &ProviderSettings,
    p: &Profile,
    candidates: &[KeyCandidate],
) -> (
    Vec<Option<Option<(BaseMood, MoodIntensity, Certainty)>>>,
    Option<ProviderError>,
) {
    let examples = examples(p);
    let mut answers = vec![None; candidates.len()];
    for (chunk_index, chunk) in candidates.chunks(BATCH_KEYS).enumerate() {
        let start = chunk_index * BATCH_KEYS;
        let batch: Vec<(String, &KeyCandidate)> = chunk
            .iter()
            .enumerate()
            .map(|(i, c)| (format!("k{}", start + i + 1), c))
            .collect();
        let prompt = build_prompt(&batch, &examples);
        match provider::chat_json(
            settings,
            SYSTEM_PROMPT,
            &[Part::Text(prompt)],
            "key_moods",
            &schema(),
        )
        .await
        {
            Ok(answer) => {
                let parsed = parse_answer(&answer);
                for (i, (id, _)) in batch.iter().enumerate() {
                    if let Some(found) = parsed.get(id) {
                        answers[start + i] = Some(*found);
                    }
                }
            }
            Err(error) => return (answers, Some(error)),
        }
    }
    (answers, None)
}

// ── Putting it together ──

/// Each mood should play something at any intensity: on each track, the
/// least demanding key of a mood plays from the first level unless an
/// existing key already does.
pub fn ensure_floor(p: &Profile, suggestions: &mut [MoodSuggestion]) {
    let mut groups: HashMap<(BaseMood, String), Vec<usize>> = HashMap::new();
    for (i, s) in suggestions.iter().enumerate() {
        groups
            .entry((s.mood, s.track_id.clone()))
            .or_default()
            .push(i);
    }
    for ((mood, track), members) in groups {
        let covered = p.key_bindings.iter().any(|b| {
            b.track_id == track
                && b.mood == Some(mood)
                && b.mood_intensity.map_or(1, |i| i.as_u8()) == 1
                && !b.sound_ids.is_empty()
        }) || members
            .iter()
            .any(|&i| suggestions[i].intensity == MoodIntensity::Low);
        if covered {
            continue;
        }
        if let Some(&lowest) = members
            .iter()
            .min_by_key(|&&i| suggestions[i].intensity.as_u8())
        {
            suggestions[lowest].intensity = MoodIntensity::Low;
        }
    }
}

/// Suggestions for every untagged key: the model's when `settings` is given
/// and answers, the keyword guess otherwise.
pub async fn suggest(settings: Option<&ProviderSettings>, p: &Profile) -> AutoTagResult {
    let mut keys = candidates(p);
    read_artists(p, &mut keys);
    let mut result = AutoTagResult::default();
    let answers = match settings {
        Some(settings) if !keys.is_empty() => {
            let (answers, error) = classify(settings, p, &keys).await;
            result.used_model = answers.iter().any(Option::is_some);
            result.model_error = error.map(|e| e.message);
            answers
        }
        _ => vec![None; keys.len()],
    };
    for (c, answer) in keys.iter().zip(answers) {
        let suggestion = match answer {
            Some(Some((mood, intensity, certainty))) => {
                let keyword = keyword_suggestion(c);
                Some(MoodSuggestion {
                    key_code: c.key_code.clone(),
                    track_id: c.track_id.clone(),
                    label: c.label.clone(),
                    mood,
                    intensity,
                    // The title saying the same thing makes a guess safer.
                    certainty: match keyword {
                        Some(k) if k.mood == mood && certainty == Certainty::Low => {
                            Certainty::Medium
                        }
                        _ => certainty,
                    },
                    source: SuggestionSource::Model,
                })
            }
            // Not music, says the model.
            Some(None) => None,
            None => keyword_suggestion(c),
        };
        match suggestion {
            Some(s) => result.suggestions.push(s),
            None => result.unknown.push(c.label.clone()),
        }
    }
    ensure_floor(p, &mut result.suggestions);
    result
}

/// Tag the keys (those still untagged: the profile may have changed since).
pub fn apply(p: &mut Profile, suggestions: &[MoodSuggestion]) -> usize {
    let mut applied = 0;
    for s in suggestions {
        if let Some(b) = p
            .key_bindings
            .iter_mut()
            .find(|b| b.key_code == s.key_code && b.track_id == s.track_id && b.mood.is_none())
        {
            b.mood = Some(s.mood);
            b.mood_intensity = (s.intensity != MoodIntensity::Low).then_some(s.intensity);
            applied += 1;
        }
    }
    applied
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::editor::tests::{binding, profile, sound};
    use crate::types::Track;

    fn music_profile() -> Profile {
        let mut p = profile();
        p.tracks = vec![Track {
            id: "t".into(),
            name: "OST".into(),
            ..p.tracks.first().cloned().unwrap()
        }];
        let mut a = sound("a");
        a.name = "Two Steps From Hell - Heart of Courage".into();
        a.duration = 200.0;
        let mut b = sound("b");
        b.name = "Naruto - Sadness and Sorrow".into();
        b.duration = 180.0;
        let mut c = sound("c");
        c.name = "Door creak sound effect".into();
        c.duration = 2.0;
        let mut d = sound("d");
        d.name = "Track 07".into();
        d.duration = 150.0;
        p.sounds = vec![a, b, c, d];
        let mut tagged = binding("KeyQ", "t", &["a"]);
        tagged.mood = Some(BaseMood::Epic);
        p.key_bindings = vec![
            tagged,
            binding("KeyW", "t", &["b"]),
            binding("KeyE", "t", &["c"]),
            binding("KeyR", "t", &["d"]),
        ];
        p
    }

    #[test]
    fn candidates_are_untagged_music_keys() {
        let p = music_profile();
        let keys: Vec<String> = candidates(&p).into_iter().map(|c| c.key_code).collect();
        assert_eq!(keys, ["KeyW", "KeyR"]);
    }

    #[test]
    fn keywords_guess_without_a_model() {
        let p = music_profile();
        let result = futures::executor::block_on(suggest(None, &p));
        assert!(!result.used_model);
        assert_eq!(result.suggestions.len(), 1);
        let s = &result.suggestions[0];
        assert_eq!((s.key_code.as_str(), s.mood), ("KeyW", BaseMood::Sadness));
        assert_eq!(s.source, SuggestionSource::Keywords);
        assert!(!s.recommended());
        assert_eq!(result.unknown, ["Track 07"]);
    }

    #[test]
    fn prompt_lists_examples_and_keys() {
        let p = music_profile();
        let keys = candidates(&p);
        let batch: Vec<(String, &KeyCandidate)> = keys
            .iter()
            .enumerate()
            .map(|(i, c)| (format!("k{}", i + 1), c))
            .collect();
        let prompt = build_prompt(&batch, &examples(&p));
        assert!(prompt.contains("\"Two Steps From Hell - Heart of Courage\" → epic 1"));
        assert!(prompt.contains("k1 | sounds: \"Naruto - Sadness and Sorrow\""));
        assert!(prompt.contains("k2 | sounds: \"Track 07\""));
    }

    #[test]
    fn answers_parse_and_none_means_not_music() {
        let parsed = parse_answer(&json!({ "keys": [
            { "id": "k1", "mood": "sadness", "intensity": 3, "certainty": "high" },
            { "id": "k2", "mood": "none", "intensity": 1, "certainty": "low" }
        ]}));
        assert_eq!(
            parsed["k1"],
            Some((BaseMood::Sadness, MoodIntensity::High, Certainty::High))
        );
        assert_eq!(parsed["k2"], None);
    }

    #[test]
    fn each_mood_keeps_a_key_for_any_intensity() {
        let p = music_profile();
        let s = |key: &str, mood, intensity| MoodSuggestion {
            key_code: key.into(),
            track_id: "t".into(),
            label: key.into(),
            mood,
            intensity,
            certainty: Certainty::High,
            source: SuggestionSource::Model,
        };
        let mut list = vec![
            s("KeyW", BaseMood::Sadness, MoodIntensity::High),
            s("KeyR", BaseMood::Sadness, MoodIntensity::Medium),
            // KeyQ already plays epic at any intensity.
            s("KeyT", BaseMood::Epic, MoodIntensity::High),
        ];
        ensure_floor(&p, &mut list);
        assert_eq!(list[0].intensity, MoodIntensity::High);
        assert_eq!(list[1].intensity, MoodIntensity::Low);
        assert_eq!(list[2].intensity, MoodIntensity::High);
    }

    #[test]
    fn apply_skips_keys_tagged_meanwhile() {
        let mut p = music_profile();
        let suggestion = |key: &str| MoodSuggestion {
            key_code: key.into(),
            track_id: "t".into(),
            label: key.into(),
            mood: BaseMood::Mystery,
            intensity: MoodIntensity::Medium,
            certainty: Certainty::High,
            source: SuggestionSource::Model,
        };
        let applied = apply(&mut p, &[suggestion("KeyQ"), suggestion("KeyR")]);
        assert_eq!(applied, 1);
        assert_eq!(p.key_bindings[0].mood, Some(BaseMood::Epic));
        assert_eq!(p.key_bindings[3].mood, Some(BaseMood::Mystery));
        assert_eq!(
            p.key_bindings[3].mood_intensity,
            Some(MoodIntensity::Medium)
        );
    }
}
