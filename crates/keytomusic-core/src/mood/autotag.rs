//! Default moods for the sounds nobody tagged.
//!
//! The model reads what is known of each sound (its title, the name of the
//! key it is on, its artist and folder) and answers a mood and the intensity
//! the piece fits. It knows most film, game and anime scores, which the
//! keyword guess of [`placement`](crate::session::placement) cannot
//! ("Unravel", "Kamado Tanjiro no Uta"). Without a model, or for what it
//! leaves out, the keyword guess still applies. The user reviews the
//! suggestions before they are applied.

use std::collections::HashMap;
use std::path::Path;

use serde::Serialize;
use serde_json::{json, Value};

use super::provider::{self, Part, ProviderError, ProviderSettings};
use crate::audio::analysis::read_audio_metadata_tags;
use crate::session::moods;
use crate::session::placement::{self, SoundHint, SoundRole};
use crate::types::{BaseMood, MoodIntensity, Profile, Sound, SoundSource};

/// Sounds per request: one answer stays short and fast.
pub const BATCH: usize = 60;
/// Sounds the user already tagged, shown as examples of their taste.
const EXAMPLES: usize = 24;

/// A sound without a mood, on a key of a music track.
#[derive(Debug, Clone, PartialEq)]
pub struct SoundCandidate {
    pub sound_id: String,
    /// The first key holding it, and its track.
    pub key_code: String,
    pub track_id: String,
    pub title: String,
    pub key_name: Option<String>,
    pub artist: Option<String>,
    pub folder: Option<String>,
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
    pub sound_id: String,
    pub key_code: String,
    pub track_id: String,
    /// The sound's title.
    pub label: String,
    pub mood: BaseMood,
    /// Least intense scene that plays the sound (Low: any).
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
    /// Sounds left without a mood (not music, or nothing to go on).
    pub unknown: Vec<String>,
    /// The model was asked (and answered at least in part).
    pub used_model: bool,
    /// Why the model could not help, when it was meant to.
    pub model_error: Option<String>,
}

// ── What is known of the sounds ──

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
        "tmp",
    ];
    let id_like = name.len() >= 16 && name.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
    (!name.is_empty() && !id_like && !generic.contains(&lower.as_str())).then_some(name)
}

fn is_music(sound: &Sound) -> bool {
    placement::guess_role(&SoundHint {
        name: sound.name.clone(),
        folder: telling_folder(&sound.source),
        duration: sound.duration,
        ..Default::default()
    }) == SoundRole::Music
}

/// Untagged sounds on keys of music tracks that sound like music (effects
/// and ambiences would fire with Mood AI).
pub fn candidates(p: &Profile) -> Vec<SoundCandidate> {
    let mut out: Vec<SoundCandidate> = Vec::new();
    for b in &p.key_bindings {
        if placement::track_role(p, &b.track_id) != SoundRole::Music {
            continue;
        }
        for sound in b
            .sound_ids
            .iter()
            .filter_map(|id| p.sounds.iter().find(|s| &s.id == id))
        {
            if sound.mood.is_some() || out.iter().any(|c| c.sound_id == sound.id) {
                continue;
            }
            if !is_music(sound) {
                continue;
            }
            out.push(SoundCandidate {
                sound_id: sound.id.clone(),
                key_code: b.key_code.clone(),
                track_id: b.track_id.clone(),
                title: sound.name.clone(),
                key_name: b
                    .name
                    .clone()
                    .filter(|n| !n.trim().is_empty() && *n != sound.name),
                artist: None,
                folder: telling_folder(&sound.source),
            });
        }
    }
    out
}

/// Artist tags of local files: "Track 3" by "Hiroyuki Sawano" says a lot.
pub fn read_artists(p: &Profile, candidates: &mut [SoundCandidate]) {
    for c in candidates.iter_mut() {
        let Some(sound) = p.sounds.iter().find(|s| s.id == c.sound_id) else {
            continue;
        };
        if !matches!(sound.source, SoundSource::Local { .. }) {
            continue;
        }
        c.artist = read_audio_metadata_tags(sound_path(&sound.source))
            .and_then(|t| t.artist)
            .map(|a| a.trim().to_string())
            .filter(|a| !a.is_empty() && !c.title.to_lowercase().contains(&a.to_lowercase()));
    }
}

// ── Keyword guess ──

/// The mood the title, key name or folder names, without a model.
pub fn keyword_suggestion(c: &SoundCandidate) -> Option<MoodSuggestion> {
    let mood = placement::guess_title_mood(&c.title)
        .or_else(|| c.key_name.as_deref().and_then(placement::guess_mood))
        .or_else(|| c.folder.as_deref().and_then(placement::guess_mood))?;
    Some(MoodSuggestion {
        sound_id: c.sound_id.clone(),
        key_code: c.key_code.clone(),
        track_id: c.track_id.clone(),
        label: c.title.clone(),
        mood,
        intensity: MoodIntensity::Low,
        certainty: Certainty::Low,
        source: SuggestionSource::Keywords,
    })
}

// ── The model ──

pub const SYSTEM_PROMPT: &str = "You tag background music for a soundboard used while reading manga, manhwa or webtoons. \
When the reader reaches a scene of a given mood, the sounds tagged with that mood play.

For each sound you get its title (often \"Work — Piece\" or a YouTube title), and sometimes the name the user gave \
the key it is on, its artist and its folder. Use what you know about the piece (film, game, anime and trailer music) \
and the words in its title.

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

Answer every sound, with its id, as JSON only.";

pub fn schema() -> Value {
    let mut moods: Vec<&str> = BaseMood::ALL.iter().map(|m| m.as_str()).collect();
    moods.push("none");
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["sounds"],
        "properties": {
            "sounds": {
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

fn quoted(text: &str) -> String {
    format!("\"{}\"", text.replace('"', "'"))
}

fn describe(c: &SoundCandidate) -> String {
    let mut line = format!(" | sound: {}", quoted(&c.title));
    if let Some(name) = &c.key_name {
        line.push_str(&format!(" | key name: {}", quoted(name)));
    }
    if let Some(artist) = &c.artist {
        line.push_str(&format!(" | artist: {}", quoted(artist)));
    }
    if let Some(folder) = &c.folder {
        line.push_str(&format!(" | folder: {}", quoted(folder)));
    }
    line
}

/// Sounds the user tagged: how they hear their own music.
fn examples(p: &Profile) -> Vec<String> {
    p.sounds
        .iter()
        .filter_map(|s| moods::of_sound(s).map(|(m, l)| (s, m, l)))
        .take(EXAMPLES)
        .map(|(s, mood, level)| {
            format!("{} → {} {}", quoted(&s.name), mood.as_str(), level.as_u8())
        })
        .collect()
}

pub fn build_prompt(batch: &[(String, &SoundCandidate)], examples: &[String]) -> String {
    let mut text = String::new();
    if !examples.is_empty() {
        text.push_str("Sounds the user already tagged (mood, intensity), to match their taste:\n");
        for e in examples {
            text.push_str(&format!("- {e}\n"));
        }
        text.push('\n');
    }
    text.push_str("Sounds to tag:\n");
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

type Answer = Option<(BaseMood, MoodIntensity, Certainty)>;

/// The model's answers, by id. `None` for sounds it judged not to be music.
pub fn parse_answer(answer: &Value) -> HashMap<String, Answer> {
    let mut out = HashMap::new();
    for item in answer
        .get("sounds")
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

/// Ask the model about every candidate, `BATCH` at a time. Answers gathered
/// before an error are kept.
pub async fn classify(
    settings: &ProviderSettings,
    p: &Profile,
    candidates: &[SoundCandidate],
) -> (Vec<Option<Answer>>, Option<ProviderError>) {
    let examples = examples(p);
    let mut answers = vec![None; candidates.len()];
    for (chunk_index, chunk) in candidates.chunks(BATCH).enumerate() {
        let start = chunk_index * BATCH;
        let batch: Vec<(String, &SoundCandidate)> = chunk
            .iter()
            .enumerate()
            .map(|(i, c)| (format!("s{}", start + i + 1), c))
            .collect();
        let prompt = build_prompt(&batch, &examples);
        match provider::chat_json(
            settings,
            SYSTEM_PROMPT,
            &[Part::Text(prompt)],
            "sound_moods",
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
/// least demanding new sound of a mood plays from the first level unless a
/// sound already tagged there does.
pub fn ensure_floor(p: &Profile, suggestions: &mut [MoodSuggestion]) {
    let mut groups: HashMap<(BaseMood, String), Vec<usize>> = HashMap::new();
    for (i, s) in suggestions.iter().enumerate() {
        groups
            .entry((s.mood, s.track_id.clone()))
            .or_default()
            .push(i);
    }
    for ((mood, track), members) in groups {
        let covered = p
            .key_bindings
            .iter()
            .filter(|b| b.track_id == track)
            .flat_map(|b| b.sound_ids.iter())
            .filter_map(|id| p.sounds.iter().find(|s| &s.id == id))
            .any(|s| moods::of_sound(s) == Some((mood, MoodIntensity::Low)))
            || members
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

/// Suggestions for every untagged sound: the model's when `settings` is
/// given and answers, the keyword guess otherwise.
pub async fn suggest(settings: Option<&ProviderSettings>, p: &Profile) -> AutoTagResult {
    let mut sounds = candidates(p);
    read_artists(p, &mut sounds);
    let mut result = AutoTagResult::default();
    let answers = match settings {
        Some(settings) if !sounds.is_empty() => {
            let (answers, error) = classify(settings, p, &sounds).await;
            result.used_model = answers.iter().any(Option::is_some);
            result.model_error = error.map(|e| e.message);
            answers
        }
        _ => vec![None; sounds.len()],
    };
    for (c, answer) in sounds.iter().zip(answers) {
        let suggestion = match answer {
            Some(Some((mood, intensity, certainty))) => {
                let keyword = keyword_suggestion(c);
                Some(MoodSuggestion {
                    sound_id: c.sound_id.clone(),
                    key_code: c.key_code.clone(),
                    track_id: c.track_id.clone(),
                    label: c.title.clone(),
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
            None => result.unknown.push(c.title.clone()),
        }
    }
    ensure_floor(p, &mut result.suggestions);
    result
}

/// Tag the sounds (those still untagged: the profile may have changed since).
pub fn apply(p: &mut Profile, suggestions: &[MoodSuggestion]) -> usize {
    let mut applied = 0;
    for s in suggestions {
        if let Some(sound) = p
            .sounds
            .iter_mut()
            .find(|sound| sound.id == s.sound_id && sound.mood.is_none())
        {
            sound.mood = Some(s.mood);
            sound.mood_intensity = (s.intensity != MoodIntensity::Low).then_some(s.intensity);
            applied += 1;
        }
    }
    moods::sync_bindings(p);
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
        a.mood = Some(BaseMood::Epic);
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
        p.key_bindings = vec![
            binding("KeyQ", "t", &["a"]),
            binding("KeyW", "t", &["b", "d"]),
            binding("KeyE", "t", &["c"]),
        ];
        p
    }

    #[test]
    fn candidates_are_untagged_music_sounds() {
        let p = music_profile();
        let sounds: Vec<String> = candidates(&p).into_iter().map(|c| c.sound_id).collect();
        assert_eq!(sounds, ["b", "d"]);
    }

    #[test]
    fn keywords_guess_without_a_model() {
        let p = music_profile();
        let result = futures::executor::block_on(suggest(None, &p));
        assert!(!result.used_model);
        assert_eq!(result.suggestions.len(), 1);
        let s = &result.suggestions[0];
        assert_eq!(
            (s.sound_id.as_str(), s.key_code.as_str(), s.mood),
            ("b", "KeyW", BaseMood::Sadness)
        );
        assert_eq!(s.source, SuggestionSource::Keywords);
        assert!(!s.recommended());
        assert_eq!(result.unknown, ["Track 07"]);
    }

    #[test]
    fn prompt_lists_examples_and_sounds() {
        let p = music_profile();
        let sounds = candidates(&p);
        let batch: Vec<(String, &SoundCandidate)> = sounds
            .iter()
            .enumerate()
            .map(|(i, c)| (format!("s{}", i + 1), c))
            .collect();
        let prompt = build_prompt(&batch, &examples(&p));
        assert!(prompt.contains("\"Two Steps From Hell - Heart of Courage\" → epic 1"));
        assert!(prompt.contains("s1 | sound: \"Naruto - Sadness and Sorrow\""));
        assert!(prompt.contains("s2 | sound: \"Track 07\""));
    }

    #[test]
    fn answers_parse_and_none_means_not_music() {
        let parsed = parse_answer(&json!({ "sounds": [
            { "id": "s1", "mood": "sadness", "intensity": 3, "certainty": "high" },
            { "id": "s2", "mood": "none", "intensity": 1, "certainty": "low" }
        ]}));
        assert_eq!(
            parsed["s1"],
            Some((BaseMood::Sadness, MoodIntensity::High, Certainty::High))
        );
        assert_eq!(parsed["s2"], None);
    }

    #[test]
    fn each_mood_keeps_a_sound_for_any_intensity() {
        let p = music_profile();
        let s = |id: &str, mood, intensity| MoodSuggestion {
            sound_id: id.into(),
            key_code: "KeyW".into(),
            track_id: "t".into(),
            label: id.into(),
            mood,
            intensity,
            certainty: Certainty::High,
            source: SuggestionSource::Model,
        };
        let mut list = vec![
            s("b", BaseMood::Sadness, MoodIntensity::High),
            s("d", BaseMood::Sadness, MoodIntensity::Medium),
            // "a" already plays epic at any intensity.
            s("x", BaseMood::Epic, MoodIntensity::High),
        ];
        ensure_floor(&p, &mut list);
        assert_eq!(list[0].intensity, MoodIntensity::High);
        assert_eq!(list[1].intensity, MoodIntensity::Low);
        assert_eq!(list[2].intensity, MoodIntensity::High);
    }

    #[test]
    fn apply_skips_sounds_tagged_meanwhile_and_updates_keys() {
        let mut p = music_profile();
        let suggestion = |id: &str| MoodSuggestion {
            sound_id: id.into(),
            key_code: "KeyW".into(),
            track_id: "t".into(),
            label: id.into(),
            mood: BaseMood::Mystery,
            intensity: MoodIntensity::Medium,
            certainty: Certainty::High,
            source: SuggestionSource::Model,
        };
        let applied = apply(&mut p, &[suggestion("a"), suggestion("d")]);
        assert_eq!(applied, 1);
        assert_eq!(p.sounds[0].mood, Some(BaseMood::Epic));
        assert_eq!(p.sounds[3].mood, Some(BaseMood::Mystery));
        assert_eq!(p.sounds[3].mood_intensity, Some(MoodIntensity::Medium));
        assert_eq!(p.key_bindings[1].mood, Some(BaseMood::Mystery));
    }
}
