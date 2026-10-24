//! Guessing where a new sound goes: what it is (music, ambience, effect),
//! its track, mood, key, loop mode and start point — following how the
//! profile is organised, track by track.
//!
//! Two ways of organising a soundboard are common:
//! - **grouped**: a key holds a family of sounds ("every epic piece on Q",
//!   played at random), keys are moods;
//! - **one per key**: each key plays one sound; similar sounds sit side by
//!   side (three epic pieces on Q, W and E).
//!
//! A new sound joins the group of its mood in the first case, and takes a
//! free key next to its kin in the second.

use std::collections::{HashMap, HashSet};

use crate::types::{BaseMood, KeyBinding, LoopMode, Profile, SoundSource};

use super::keymap::AUTO_KEY_ORDER;
use super::rules::{seed_mood, youtube_video_id};

/// What a sound is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundRole {
    Music,
    /// A looping background (rain, wind, crowd).
    Ambience,
    /// A short one-shot (door, hit, scream).
    Effect,
}

/// How the keys of a track are organised.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    /// Nothing there yet.
    Empty,
    OnePerKey,
    /// Keys hold families of sounds.
    Grouped,
}

/// What is known about a sound to place.
#[derive(Debug, Clone, Default)]
pub struct SoundHint {
    pub name: String,
    /// Folder of a local file, often telling ("Combat", "Ambiances").
    pub folder: Option<String>,
    pub duration: f64,
    /// A mood the user chose.
    pub mood: Option<BaseMood>,
    /// A track the user chose.
    pub track_id: Option<String>,
    /// Videos of the profile this sound was found from (Discover).
    pub seed_video_ids: Vec<String>,
}

/// Where a sound goes, and why.
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    pub role: SoundRole,
    pub key: Option<String>,
    /// None: a new track is needed.
    pub track_id: Option<String>,
    pub mood: Option<BaseMood>,
    /// For a new key; a key joined keeps its own.
    pub loop_mode: LoopMode,
    /// Sounds already on the key it joins (0: a new key).
    pub joins: usize,
    /// Short French explanation ("avec tes 3 sons Épique").
    pub reason: String,
}

// ── What a sound is ──

const AMBIENCE_WORDS: &[&str] = &[
    "ambience", "ambiance", "ambient", "atmosphere", "atmosphère", "rain", "pluie", "wind", "vent",
    "storm", "orage", "thunder", "tonnerre", "forest", "forêt", "foret", "ocean", "océan", "sea",
    "mer", "waves", "vagues", "river", "rivière", "city", "ville", "crowd", "foule", "night",
    "nuit", "fire", "feu", "fireplace", "cafe", "café", "tavern", "taverne", "birds", "oiseaux",
    "cave", "grotte", "room", "noise", "bruit", "background", "soundscape",
];
const EFFECT_WORDS: &[&str] = &[
    "sfx", "effect", "effects", "effet", "door", "porte", "knock", "punch", "coup", "hit",
    "explosion", "gunshot", "gun", "sword", "épée", "epee", "scream", "cri", "laugh", "rire",
    "footsteps", "pas", "heartbeat", "cœur", "coeur", "glass", "verre", "slash", "whoosh",
    "swoosh", "bell", "cloche", "click", "creak", "creaking", "grincement", "growl", "roar",
    "magic", "magie", "spell", "impact", "crash", "jump", "saut", "gasp", "notification",
];
const MUSIC_WORDS: &[&str] = &[
    "ost", "soundtrack", "theme", "thème", "bgm", "music", "musique", "song", "piano", "orchestra",
    "orchestral", "opening", "ending", "remix", "cover", "symphony", "violin", "guitar",
    "instrumental", "lofi", "score", "suite",
];

fn fold(c: char) -> char {
    match c {
        'à' | 'â' | 'ä' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'î' | 'ï' => 'i',
        'ô' | 'ö' => 'o',
        'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        c => c,
    }
}

/// Lowercase words without accents.
pub fn words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| w.chars().map(fold).collect())
        .collect()
}

fn count_in(words: &[String], list: &[&str]) -> usize {
    let list: HashSet<String> = list.iter().map(|w| w.chars().map(fold).collect()).collect();
    // Plurals count too ("Ambiances", "effects").
    words
        .iter()
        .filter(|w| list.contains(*w) || w.strip_suffix('s').is_some_and(|w| list.contains(w)))
        .count()
}

/// Music, ambience or effect, from the title, folder and length.
pub fn guess_role(hint: &SoundHint) -> SoundRole {
    let mut text = hint.name.clone();
    if let Some(folder) = &hint.folder {
        text.push(' ');
        text.push_str(folder);
    }
    let w = words(&text);
    let (ambience, effect, music) = (
        count_in(&w, AMBIENCE_WORDS),
        count_in(&w, EFFECT_WORDS),
        count_in(&w, MUSIC_WORDS),
    );
    let short = hint.duration > 0.0 && hint.duration < 20.0;
    if music > 0 && music >= ambience && music >= effect {
        // "Rain OST" is music; a 5 s "theme" sting is still an effect.
        return if short && effect > 0 { SoundRole::Effect } else { SoundRole::Music };
    }
    if effect > ambience || (short && effect > 0) {
        return SoundRole::Effect;
    }
    if ambience > 0 {
        return if short { SoundRole::Effect } else { SoundRole::Ambience };
    }
    if short {
        SoundRole::Effect
    } else {
        SoundRole::Music
    }
}

// ── Mood from a title ──

const MOOD_WORDS: [(BaseMood, &[&str]); 8] = [
    (
        BaseMood::Epic,
        &[
            "epic", "epique", "battle", "bataille", "fight", "combat", "war", "guerre", "boss",
            "heroic", "heroique", "hero", "heros", "victory", "victoire", "climax", "triumph",
            "orchestral", "power",
        ],
    ),
    (
        BaseMood::Tension,
        &[
            "tension", "tense", "suspense", "chase", "poursuite", "danger", "thriller", "urgent",
            "urgence", "stress", "infiltration", "stealth",
        ],
    ),
    (
        BaseMood::Sadness,
        &[
            "sad", "sadness", "triste", "tristesse", "sorrow", "melancholy", "melancolie",
            "melancolique", "tears", "larmes", "grief", "lament", "farewell", "adieu", "crying",
            "emotional", "emotion", "loss",
        ],
    ),
    (
        BaseMood::Comedy,
        &[
            "funny", "comedy", "comedie", "comique", "drole", "silly", "goofy", "gag", "humor",
            "humour", "wacky", "cartoon", "comical", "joyful", "happy",
        ],
    ),
    (
        BaseMood::Romance,
        &[
            "romance", "romantic", "romantique", "love", "amour", "amoureux", "kiss", "wedding",
            "mariage", "confession", "tender", "tendre",
        ],
    ),
    (
        BaseMood::Horror,
        &[
            "horror", "horreur", "scary", "creepy", "terror", "terreur", "ghost", "fantome",
            "zombie", "nightmare", "cauchemar", "haunted", "hante", "demon", "evil", "sinister",
            "sinistre", "effrayant", "peur",
        ],
    ),
    (
        BaseMood::Peaceful,
        &[
            "peaceful", "peace", "calm", "calme", "relax", "relaxing", "relaxant", "serene",
            "serein", "paisible", "tranquil", "tranquille", "chill", "lofi", "cozy", "gentle",
            "doux", "douce", "village", "morning", "matin", "sunny", "breeze",
        ],
    ),
    (
        BaseMood::Mystery,
        &[
            "mystery", "mysterious", "mystere", "mysterieux", "mysterieuse", "enigma", "enigme",
            "detective", "investigation", "enquete", "secret", "unknown", "inconnu", "riddle",
        ],
    ),
];

/// The mood `text` names, if it names one more than the others.
pub fn guess_mood(text: &str) -> Option<BaseMood> {
    let w = words(text);
    let mut scores: Vec<(BaseMood, usize)> = MOOD_WORDS
        .iter()
        .map(|(mood, list)| (*mood, w.iter().filter(|x| list.contains(&x.as_str())).count()))
        .filter(|(_, score)| *score > 0)
        .collect();
    scores.sort_by(|a, b| b.1.cmp(&a.1));
    match scores.as_slice() {
        [] => None,
        [(mood, _)] => Some(*mood),
        [(mood, best), (_, next), ..] => (best > next).then_some(*mood),
    }
}

/// French name of a mood, for explanations.
pub fn mood_label(mood: BaseMood) -> &'static str {
    match mood {
        BaseMood::Epic => "Épique",
        BaseMood::Tension => "Tension",
        BaseMood::Sadness => "Tristesse",
        BaseMood::Comedy => "Comédie",
        BaseMood::Romance => "Romance",
        BaseMood::Horror => "Horreur",
        BaseMood::Peaceful => "Paisible",
        BaseMood::Mystery => "Mystère",
    }
}

// ── Tracks ──

fn track_name_has(p: &Profile, id: &str, parts: &[&str]) -> bool {
    p.tracks.iter().any(|t| {
        t.id == id && {
            let name: String = t.name.to_lowercase().chars().map(fold).collect();
            parts.iter().any(|part| name.contains(part))
        }
    })
}

/// The ambience track (named so).
pub fn ambience_track(p: &Profile) -> Option<String> {
    p.tracks
        .iter()
        .find(|t| track_name_has(p, &t.id, &["ambian", "ambien", "fond", "atmos"]))
        .map(|t| t.id.clone())
}

/// The effects track (lowers the others, or named so).
pub fn effects_track(p: &Profile) -> Option<String> {
    p.tracks
        .iter()
        .find(|t| t.ducks_others)
        .or_else(|| {
            p.tracks
                .iter()
                .find(|t| track_name_has(p, &t.id, &["effet", "sfx", "bruit", "fx"]))
        })
        .map(|t| t.id.clone())
}

/// What a track is for, from its name and what it holds.
pub fn track_role(p: &Profile, track: &str) -> SoundRole {
    if effects_track(p).as_deref() == Some(track) {
        SoundRole::Effect
    } else if ambience_track(p).as_deref() == Some(track) {
        SoundRole::Ambience
    } else {
        SoundRole::Music
    }
}

fn track_for_role(p: &Profile, role: SoundRole) -> Option<String> {
    match role {
        SoundRole::Ambience => ambience_track(p),
        SoundRole::Effect => effects_track(p),
        SoundRole::Music => {
            let background: HashSet<String> =
                ambience_track(p).into_iter().chain(effects_track(p)).collect();
            // The track holding the most music.
            let mut counts: HashMap<&str, usize> = HashMap::new();
            for b in &p.key_bindings {
                if !background.contains(&b.track_id) {
                    *counts.entry(b.track_id.as_str()).or_default() += b.sound_ids.len();
                }
            }
            counts
                .into_iter()
                .max_by_key(|(_, n)| *n)
                .map(|(t, _)| t.to_string())
                .or_else(|| {
                    p.tracks
                        .iter()
                        .find(|t| !background.contains(&t.id))
                        .map(|t| t.id.clone())
                })
        }
    }
}

// ── Organisation ──

/// How the keys of `track` (or the whole profile, when it has too few) are
/// organised.
pub fn style(p: &Profile, track: Option<&str>) -> Style {
    let on_track: Vec<&KeyBinding> = p
        .key_bindings
        .iter()
        .filter(|b| track.is_none_or(|t| b.track_id == t))
        .collect();
    let bindings: Vec<&KeyBinding> = if on_track.len() >= 2 {
        on_track
    } else {
        p.key_bindings.iter().collect()
    };
    if bindings.is_empty() {
        return Style::Empty;
    }
    let groups = bindings.iter().filter(|b| b.sound_ids.len() >= 2).count();
    let sounds: usize = bindings.iter().map(|b| b.sound_ids.len()).sum();
    // Most keys hold several sounds, or a few hold many.
    if groups * 2 >= bindings.len() && sounds as f64 / bindings.len() as f64 >= 1.8 {
        Style::Grouped
    } else {
        Style::OnePerKey
    }
}

// ── Keyboard geometry ──

/// Position of a key on a standard keyboard (row, column), for "next to".
fn key_position(code: &str) -> Option<(f64, f64)> {
    const ROWS: [(&[&str], f64, f64); 5] = [
        (
            &["F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12"],
            -1.2,
            0.5,
        ),
        (
            &[
                "Digit1", "Digit2", "Digit3", "Digit4", "Digit5", "Digit6", "Digit7", "Digit8",
                "Digit9", "Digit0",
            ],
            0.0,
            0.0,
        ),
        (
            &["KeyQ", "KeyW", "KeyE", "KeyR", "KeyT", "KeyY", "KeyU", "KeyI", "KeyO", "KeyP"],
            1.0,
            0.5,
        ),
        (
            &["KeyA", "KeyS", "KeyD", "KeyF", "KeyG", "KeyH", "KeyJ", "KeyK", "KeyL"],
            2.0,
            0.75,
        ),
        (
            &["KeyZ", "KeyX", "KeyC", "KeyV", "KeyB", "KeyN", "KeyM"],
            3.0,
            1.25,
        ),
    ];
    // A combo ("ShiftLeft+KeyQ") sits where its main key does.
    let base = code.rsplit('+').next().unwrap_or(code);
    ROWS.iter().find_map(|(keys, row, offset)| {
        keys.iter()
            .position(|k| *k == base)
            .map(|col| (*row, col as f64 + offset))
    })
}

fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    // Moving along a row is cheaper than changing rows.
    ((a.0 - b.0) * 1.6).hypot(a.1 - b.1)
}

/// The row a role starts on when nothing guides it.
fn role_row(role: SoundRole) -> &'static [&'static str] {
    match role {
        SoundRole::Music => &[
            "KeyQ", "KeyW", "KeyE", "KeyR", "KeyT", "KeyY", "KeyU", "KeyI", "KeyO", "KeyP",
        ],
        SoundRole::Ambience => &[
            "KeyA", "KeyS", "KeyD", "KeyF", "KeyG", "KeyH", "KeyJ", "KeyK", "KeyL",
        ],
        SoundRole::Effect => &["KeyZ", "KeyX", "KeyC", "KeyV", "KeyB", "KeyN", "KeyM"],
    }
}

/// The free key nearest to `anchors`, else the first free one of the role's
/// row, else of the usual order.
fn free_key_near(anchors: &[String], role: SoundRole, taken: &HashSet<String>) -> Option<String> {
    let free: Vec<&str> = AUTO_KEY_ORDER
        .iter()
        .copied()
        .filter(|k| !taken.contains(*k))
        .collect();
    let points: Vec<(f64, f64)> = anchors.iter().filter_map(|a| key_position(a)).collect();
    if !points.is_empty() {
        let mut best: Option<(&str, f64)> = None;
        for key in &free {
            let Some(pos) = key_position(key) else { continue };
            let d = points
                .iter()
                .map(|p| distance(*p, pos))
                .fold(f64::INFINITY, f64::min);
            // The earliest key of the usual order wins a tie.
            if best.is_none_or(|(_, bd)| d < bd) {
                best = Some((key, d));
            }
        }
        if let Some((key, _)) = best {
            return Some(key.to_string());
        }
    }
    role_row(role)
        .iter()
        .copied()
        .find(|k| !taken.contains(*k))
        .or_else(|| free.first().copied())
        .map(str::to_string)
}

// ── Similarity to the sounds already there ──

const STOP_WORDS: &[&str] = &[
    "the", "and", "for", "with", "from", "official", "video", "audio", "full", "version", "music",
    "sound", "theme", "anime", "les", "des", "une", "pour", "avec", "dans", "music", "ost",
];

fn significant(text: &str) -> HashSet<String> {
    words(text)
        .into_iter()
        .filter(|w| w.chars().count() >= 4 && !STOP_WORDS.contains(&w.as_str()))
        .filter(|w| !w.chars().all(|c| c.is_ascii_digit()))
        .collect()
}

/// How close a binding is to a sound: shared seed videos weigh most, then
/// shared title words (same anime, same composer).
fn kinship(p: &Profile, b: &KeyBinding, seeds: &HashSet<&str>, title: &HashSet<String>) -> usize {
    let mut score = 0;
    for id in &b.sound_ids {
        let Some(sound) = p.sounds.iter().find(|s| &s.id == id) else {
            continue;
        };
        let video = match &sound.source {
            SoundSource::YouTube { url, .. } => youtube_video_id(url),
            SoundSource::Local { .. } => sound.resolved_video_id.as_deref(),
        };
        if video.is_some_and(|v| seeds.contains(v)) {
            score += 10;
        }
        score += significant(&sound.name).intersection(title).count();
    }
    if let Some(name) = &b.name {
        score += 2 * significant(name).intersection(title).count();
    }
    score
}

fn majority_loop(bindings: &[&KeyBinding]) -> Option<LoopMode> {
    let mut counts: Vec<(LoopMode, usize)> = Vec::new();
    for b in bindings {
        match counts.iter_mut().find(|(m, _)| *m == b.loop_mode) {
            Some((_, n)) => *n += 1,
            None => counts.push((b.loop_mode.clone(), 1)),
        }
    }
    counts.into_iter().max_by_key(|(_, n)| *n).map(|(m, _)| m)
}

fn plural(n: usize, word: &str) -> String {
    format!("{n} {word}{}", if n > 1 { "s" } else { "" })
}

/// Where each of `hints` goes, in order: keys taken by an earlier one are
/// not offered again, and one-per-key sounds gather next to each other.
pub fn place_all(p: &Profile, hints: &[SoundHint]) -> Vec<Placement> {
    let mut taken: HashSet<String> = p.key_bindings.iter().map(|b| b.key_code.clone()).collect();
    // Keys given to earlier sounds of this batch: (key, track, mood).
    let mut placed: Vec<(String, Option<String>, Option<BaseMood>)> = Vec::new();
    hints
        .iter()
        .map(|hint| {
            let placement = place_one(p, hint, &taken, &placed);
            if let Some(key) = &placement.key {
                if placement.joins == 0 {
                    taken.insert(key.clone());
                    placed.push((key.clone(), placement.track_id.clone(), placement.mood));
                }
            }
            placement
        })
        .collect()
}

pub fn place(p: &Profile, hint: &SoundHint) -> Placement {
    place_all(p, std::slice::from_ref(hint)).remove(0)
}

fn place_one(
    p: &Profile,
    hint: &SoundHint,
    taken: &HashSet<String>,
    placed: &[(String, Option<String>, Option<BaseMood>)],
) -> Placement {
    // What it is: the chosen track says it, else the title and length.
    let role = match &hint.track_id {
        Some(track) if p.tracks.iter().any(|t| &t.id == track) => track_role(p, track),
        _ => guess_role(hint),
    };
    let track_id = hint
        .track_id
        .clone()
        .filter(|t| p.tracks.iter().any(|x| &x.id == t))
        .or_else(|| {
            // Close to sounds of the profile: their track (music found from
            // a music goes with the music).
            if hint.seed_video_ids.is_empty() || role != SoundRole::Music {
                return None;
            }
            let seeds: HashSet<&str> = hint.seed_video_ids.iter().map(String::as_str).collect();
            p.key_bindings
                .iter()
                .filter(|b| track_role(p, &b.track_id) == SoundRole::Music)
                .map(|b| (b, kinship(p, b, &seeds, &HashSet::new())))
                .filter(|(_, s)| *s > 0)
                .max_by_key(|(_, s)| *s)
                .map(|(b, _)| b.track_id.clone())
        })
        .or_else(|| track_for_role(p, role))
        .or_else(|| p.tracks.first().map(|t| t.id.clone()));

    // Its mood: chosen, else that of the sounds it was found from, else its
    // title's. Only music answers moods (an effect would fire with Mood AI).
    let mut text = hint.name.clone();
    if let Some(folder) = &hint.folder {
        text.push(' ');
        text.push_str(folder);
    }
    let mood = hint.mood.or_else(|| {
        if role != SoundRole::Music {
            return None;
        }
        seed_mood(p, &hint.seed_video_ids).or_else(|| guess_mood(&text))
    });

    let on_track: Vec<&KeyBinding> = p
        .key_bindings
        .iter()
        .filter(|b| track_id.as_deref() == Some(b.track_id.as_str()))
        .collect();
    let style = style(p, track_id.as_deref());
    let default_loop = match role {
        SoundRole::Ambience => LoopMode::Single,
        _ => LoopMode::Off,
    };

    if style == Style::Grouped {
        let seeds: HashSet<&str> = hint.seed_video_ids.iter().map(String::as_str).collect();
        let title = significant(&text);
        // The family of its mood first, else the closest one.
        let by_mood = mood.and_then(|m| {
            on_track
                .iter()
                .filter(|b| b.mood == Some(m))
                .max_by_key(|b| (kinship(p, b, &seeds, &title), b.sound_ids.len()))
        });
        let by_kin = || {
            on_track
                .iter()
                .map(|b| (*b, kinship(p, b, &seeds, &title)))
                .filter(|(_, s)| *s > 0)
                .max_by_key(|(b, s)| (*s, b.sound_ids.len()))
                .map(|(b, _)| b)
        };
        let join = by_mood.copied().or_else(|| {
            // A mood with no family yet starts its own (below).
            if mood.is_some() && on_track.iter().any(|b| b.mood.is_some()) {
                None
            } else {
                by_kin()
            }
        });
        if let Some(b) = join {
            let reason = match (by_mood.is_some(), mood) {
                (true, Some(m)) => format!(
                    "rejoint {} {}",
                    plural(b.sound_ids.len(), "son"),
                    mood_label(m)
                ),
                _ => format!("rejoint {} proches", plural(b.sound_ids.len(), "son")),
            };
            return Placement {
                role,
                key: Some(b.key_code.clone()),
                track_id,
                mood: mood.or(b.mood),
                loop_mode: b.loop_mode.clone(),
                joins: b.sound_ids.len(),
                reason,
            };
        }
        // A new family: next to the others of the track, playing like them.
        let anchors: Vec<String> = on_track.iter().map(|b| b.key_code.clone()).collect();
        let groups: Vec<&KeyBinding> = on_track
            .iter()
            .filter(|b| b.sound_ids.len() >= 2)
            .copied()
            .collect();
        let key = free_key_near(&anchors, role, taken);
        return Placement {
            role,
            key,
            track_id,
            mood,
            loop_mode: majority_loop(&groups).unwrap_or(LoopMode::Random),
            joins: 0,
            reason: match mood {
                Some(m) => format!("nouveau groupe {}", mood_label(m)),
                None => "nouveau groupe".to_string(),
            },
        };
    }

    // One sound per key: a free key next to its kin — same mood, else same
    // track — including the ones just placed in this batch.
    let same_track = |t: &Option<String>| t.as_deref() == track_id.as_deref();
    let mut kin: Vec<String> = Vec::new();
    if let Some(m) = mood {
        kin.extend(
            on_track
                .iter()
                .filter(|b| b.mood == Some(m))
                .map(|b| b.key_code.clone()),
        );
        kin.extend(
            placed
                .iter()
                .filter(|(_, t, pm)| same_track(t) && *pm == Some(m))
                .map(|(k, ..)| k.clone()),
        );
    }
    let by_mood = !kin.is_empty();
    if kin.is_empty() {
        kin.extend(on_track.iter().map(|b| b.key_code.clone()));
        kin.extend(
            placed
                .iter()
                .filter(|(_, t, _)| same_track(t))
                .map(|(k, ..)| k.clone()),
        );
    }
    let key = free_key_near(&kin, role, taken);
    let loop_mode = if on_track.is_empty() {
        default_loop
    } else {
        majority_loop(&on_track).unwrap_or(default_loop)
    };
    let reason = match (by_mood, mood, kin.is_empty()) {
        (true, Some(m), _) => format!("à côté de tes sons {}", mood_label(m)),
        (_, _, false) => match role {
            SoundRole::Music => "à côté de tes musiques".to_string(),
            SoundRole::Ambience => "à côté de tes ambiances".to_string(),
            SoundRole::Effect => "à côté de tes bruitages".to_string(),
        },
        _ => match role {
            SoundRole::Music => "rangée des musiques".to_string(),
            SoundRole::Ambience => "rangée des ambiances".to_string(),
            SoundRole::Effect => "rangée des bruitages".to_string(),
        },
    };
    Placement {
        role,
        key,
        track_id,
        mood,
        loop_mode,
        joins: 0,
        reason,
    }
}

// ── Start point ──

/// Where a sound should start: music at its strong part (`suggested`), an
/// ambience or an effect where it becomes audible (no leading silence).
pub fn start_point(role: SoundRole, peaks: &[f32], duration: f64, suggested: Option<f64>) -> f64 {
    if role == SoundRole::Music {
        if let Some(s) = suggested {
            return s.max(0.0);
        }
    }
    audible_start(peaks, duration)
}

/// First moment louder than a whisper (a tenth of the loudest peak), a
/// little before it so the attack is kept. 0 when it starts right away.
pub fn audible_start(peaks: &[f32], duration: f64) -> f64 {
    let max = peaks.iter().copied().fold(0.0f32, f32::max);
    if peaks.is_empty() || max <= 0.0 || duration <= 0.0 {
        return 0.0;
    }
    let Some(first) = peaks.iter().position(|p| *p >= max * 0.1) else {
        return 0.0;
    };
    let at = first as f64 / peaks.len() as f64 * duration;
    // Under a quarter second of silence is not worth skipping.
    if at < 0.25 {
        0.0
    } else {
        (at - 0.05).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::editor::tests::{binding, profile, sound};
    use crate::types::{Profile, Track};

    fn track(id: &str, name: &str, ducks: bool) -> Track {
        Track {
            id: id.into(),
            name: name.into(),
            volume: 1.0,
            ducks_others: ducks,
        }
    }

    fn hint(name: &str, duration: f64) -> SoundHint {
        SoundHint {
            name: name.into(),
            duration,
            ..Default::default()
        }
    }

    fn scene() -> Profile {
        let mut p = profile();
        p.tracks = vec![
            track("m", "Musique", false),
            track("a", "Ambiance", false),
            track("e", "Effets", true),
        ];
        p.sounds = (1..=9).map(|i| sound(&format!("s{i}"))).collect();
        p.key_bindings.clear();
        p
    }

    #[test]
    fn titles_and_lengths_tell_what_a_sound_is() {
        assert_eq!(guess_role(&hint("Epic Battle Theme (Naruto OST)", 180.0)), SoundRole::Music);
        assert_eq!(guess_role(&hint("Rain sound for sleeping", 600.0)), SoundRole::Ambience);
        assert_eq!(guess_role(&hint("Door creaking", 4.0)), SoundRole::Effect);
        assert_eq!(guess_role(&hint("Untitled", 3.0)), SoundRole::Effect);
        assert_eq!(guess_role(&hint("Untitled", 200.0)), SoundRole::Music);
        let mut folder = hint("track 03", 240.0);
        folder.folder = Some("Ambiances".into());
        assert_eq!(guess_role(&folder), SoundRole::Ambience);
    }

    #[test]
    fn one_per_key_profiles_put_kin_side_by_side() {
        let mut p = scene();
        let mut epic = binding("KeyQ", "m", &["s1"]);
        epic.mood = Some(BaseMood::Epic);
        let mut sad = binding("KeyP", "m", &["s2"]);
        sad.mood = Some(BaseMood::Sadness);
        p.key_bindings = vec![epic, sad, binding("KeyA", "a", &["s3"])];
        assert_eq!(style(&p, Some("m")), Style::OnePerKey);

        let placed = place(&p, &hint("Epic battle music", 200.0));
        assert_eq!(placed.role, SoundRole::Music);
        assert_eq!(placed.track_id.as_deref(), Some("m"));
        assert_eq!(placed.mood, Some(BaseMood::Epic));
        assert_eq!(placed.key.as_deref(), Some("KeyW"), "{}", placed.reason);
        assert_eq!(placed.joins, 0);

        let sad = place(&p, &hint("Sad piano", 200.0));
        assert_eq!(sad.key.as_deref(), Some("KeyO"), "{}", sad.reason);

        let rain = place(&p, &hint("Rain ambience", 600.0));
        assert_eq!(rain.track_id.as_deref(), Some("a"));
        assert_eq!(rain.key.as_deref(), Some("KeyS"));
        assert_eq!(rain.mood, None);
        assert_eq!(rain.loop_mode, LoopMode::Off, "follows the track's keys");

        let door = place(&p, &hint("Door knock", 3.0));
        assert_eq!(door.track_id.as_deref(), Some("e"));
        assert_eq!(door.key.as_deref(), Some("KeyZ"));
    }

    #[test]
    fn a_batch_of_kin_fills_neighbouring_keys() {
        let p = scene();
        let placed = place_all(
            &p,
            &[
                hint("Epic battle", 200.0),
                hint("Epic victory", 200.0),
                hint("Rain ambience", 600.0),
            ],
        );
        let keys: Vec<_> = placed.iter().map(|x| x.key.clone().unwrap()).collect();
        assert_eq!(keys, ["KeyQ", "KeyW", "KeyA"]);
        assert_eq!(placed[2].loop_mode, LoopMode::Single);
    }

    #[test]
    fn grouped_profiles_join_the_family_of_their_mood() {
        let mut p = scene();
        let mut epic = binding("KeyQ", "m", &["s1", "s2", "s3"]);
        epic.mood = Some(BaseMood::Epic);
        epic.loop_mode = LoopMode::Random;
        let mut sad = binding("KeyW", "m", &["s4", "s5"]);
        sad.mood = Some(BaseMood::Sadness);
        sad.loop_mode = LoopMode::Random;
        p.key_bindings = vec![epic, sad];
        assert_eq!(style(&p, Some("m")), Style::Grouped);

        let joined = place(&p, &hint("Epic battle", 200.0));
        assert_eq!(joined.key.as_deref(), Some("KeyQ"));
        assert_eq!(joined.joins, 3);
        assert!(joined.reason.contains("Épique"), "{}", joined.reason);

        // A mood without a family starts one, next door, played like the others.
        let new = place(&p, &hint("Funny cartoon", 200.0));
        assert_eq!(new.joins, 0);
        assert_eq!(new.key.as_deref(), Some("KeyE"));
        assert_eq!(new.loop_mode, LoopMode::Random);
    }

    #[test]
    fn grouped_profiles_without_moods_follow_titles() {
        let mut p = scene();
        p.sounds[0].name = "Naruto OST - Sadness and Sorrow".into();
        p.sounds[1].name = "Naruto OST - Blue Bird".into();
        p.sounds[2].name = "Bleach OST - Number One".into();
        p.sounds[3].name = "Bleach OST - Treachery".into();
        p.key_bindings = vec![
            binding("KeyQ", "m", &["s1", "s2"]),
            binding("KeyW", "m", &["s3", "s4"]),
        ];
        let placed = place(&p, &hint("Bleach OST - Invasion", 200.0));
        assert_eq!(placed.key.as_deref(), Some("KeyW"), "{}", placed.reason);
    }

    #[test]
    fn suggestions_follow_their_seeds() {
        let mut p = scene();
        p.sounds[0].source = SoundSource::YouTube {
            url: "https://www.youtube.com/watch?v=seed1".into(),
            cached_path: String::new(),
        };
        let mut music = binding("KeyQ", "m", &["s1"]);
        music.mood = Some(BaseMood::Tension);
        p.key_bindings = vec![music, binding("KeyA", "a", &["s2"]), binding("KeyS", "a", &["s3"])];
        let mut h = hint("Some piece", 200.0);
        h.seed_video_ids = vec!["seed1".into()];
        let placed = place(&p, &h);
        assert_eq!(placed.track_id.as_deref(), Some("m"));
        assert_eq!(placed.mood, Some(BaseMood::Tension));
        assert_eq!(placed.key.as_deref(), Some("KeyW"));
    }

    #[test]
    fn effects_start_where_they_become_audible() {
        let mut peaks = vec![0.0; 10];
        peaks.extend(vec![0.8; 90]);
        // 10 s, the first second silent.
        let at = audible_start(&peaks, 10.0);
        assert!((at - 0.95).abs() < 1e-9, "{at}");
        assert_eq!(audible_start(&[0.5; 100], 10.0), 0.0);
        assert_eq!(start_point(SoundRole::Music, &peaks, 10.0, Some(4.0)), 4.0);
        assert!((start_point(SoundRole::Effect, &peaks, 10.0, Some(4.0)) - 0.95).abs() < 1e-9);
    }
}
