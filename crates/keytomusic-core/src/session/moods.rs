//! Moods live on sounds; keys answer them through their sounds.
//!
//! A sound is tagged with the scenes it fits (a mood) and the least intense
//! scene it plays for. Whatever the organisation of the profile, it works
//! the same way:
//! - keys grouped by mood ("Q = epic"): tagging the key tags its sounds;
//! - one sound per key: the sound's mood is the key's;
//! - a key mixing moods answers each of them with the matching sounds only.
//!
//! When Mood AI decides a scene, each track plays the key holding sounds of
//! that mood for the most demanding level the scene reaches, and picks among
//! those sounds only.
//!
//! The key's own `mood`/`mood_intensity` fields are a derived copy (main mood
//! of its sounds) for older versions and the web UI; profiles from before
//! moods moved to sounds are migrated from them.

use std::collections::HashMap;

use crate::types::{BaseMood, KeyBinding, LoopMode, MoodIntensity, Profile, Sound};

/// A sound's mood, with its level (1 when none is set).
pub fn of_sound(sound: &Sound) -> Option<(BaseMood, MoodIntensity)> {
    sound
        .mood
        .map(|mood| (mood, sound.mood_intensity.unwrap_or(MoodIntensity::Low)))
}

fn sounds_of<'a>(p: &'a Profile, b: &KeyBinding) -> Vec<&'a Sound> {
    b.sound_ids
        .iter()
        .filter_map(|id| p.sounds.iter().find(|s| &s.id == id))
        .collect()
}

/// Moods a key's sounds carry: (mood, lowest level, number of sounds), most
/// common first (first seen wins a tie).
pub fn of_binding(p: &Profile, b: &KeyBinding) -> Vec<(BaseMood, MoodIntensity, usize)> {
    let mut moods: Vec<(BaseMood, MoodIntensity, usize)> = Vec::new();
    for (mood, level) in sounds_of(p, b).into_iter().filter_map(of_sound) {
        match moods.iter_mut().find(|(m, ..)| *m == mood) {
            Some((_, lowest, count)) => {
                *count += 1;
                if level.as_u8() < lowest.as_u8() {
                    *lowest = level;
                }
            }
            None => moods.push((mood, level, 1)),
        }
    }
    moods.sort_by_key(|(_, _, count)| std::cmp::Reverse(*count));
    moods
}

/// Moods a key answers (all of its tracks), most common first.
pub fn of_key(p: &Profile, key: &str) -> Vec<(BaseMood, MoodIntensity)> {
    let mut moods: Vec<(BaseMood, MoodIntensity, usize)> = Vec::new();
    for b in p.key_bindings.iter().filter(|b| b.key_code == key) {
        for (mood, level, count) in of_binding(p, b) {
            match moods.iter_mut().find(|(m, ..)| *m == mood) {
                Some((_, lowest, n)) => {
                    *n += count;
                    if level.as_u8() < lowest.as_u8() {
                        *lowest = level;
                    }
                }
                None => moods.push((mood, level, count)),
            }
        }
    }
    moods.sort_by_key(|(_, _, count)| std::cmp::Reverse(*count));
    moods.into_iter().map(|(m, l, _)| (m, l)).collect()
}

/// Does any sound of the key carry `mood`?
pub fn key_has(p: &Profile, key: &str, mood: BaseMood) -> bool {
    p.key_bindings
        .iter()
        .filter(|b| b.key_code == key)
        .flat_map(|b| b.sound_ids.iter())
        .filter_map(|id| p.sounds.iter().find(|s| &s.id == id))
        .any(|s| s.mood == Some(mood))
}

/// Keys answering `mood` with their lowest level, in key order.
pub fn keys_with(p: &Profile, mood: BaseMood) -> Vec<(String, MoodIntensity)> {
    let mut keys: Vec<(String, MoodIntensity)> = Vec::new();
    for b in &p.key_bindings {
        for (m, level, _) in of_binding(p, b) {
            if m != mood {
                continue;
            }
            match keys.iter_mut().find(|(k, _)| *k == b.key_code) {
                Some((_, lowest)) if level.as_u8() < lowest.as_u8() => *lowest = level,
                Some(_) => {}
                None => keys.push((b.key_code.clone(), level)),
            }
        }
    }
    keys.sort_by(|a, b| a.0.cmp(&b.0));
    keys
}

// ── Editing ──

fn level_field(intensity: Option<MoodIntensity>) -> Option<MoodIntensity> {
    intensity.filter(|i| *i != MoodIntensity::Low)
}

/// Tag one sound (`None` clears it).
pub fn set_sound(
    p: &mut Profile,
    sound_id: &str,
    mood: Option<BaseMood>,
    intensity: Option<MoodIntensity>,
) {
    if let Some(sound) = p.sounds.iter_mut().find(|s| s.id == sound_id) {
        sound.mood = mood;
        sound.mood_intensity = mood.and(level_field(intensity));
    }
    sync_bindings(p);
}

/// Tag every sound of a key, on all its tracks (`None` clears them).
pub fn set_key(
    p: &mut Profile,
    key: &str,
    mood: Option<BaseMood>,
    intensity: Option<MoodIntensity>,
) {
    let ids: Vec<String> = p
        .key_bindings
        .iter()
        .filter(|b| b.key_code == key)
        .flat_map(|b| b.sound_ids.iter().cloned())
        .collect();
    for sound in p.sounds.iter_mut().filter(|s| ids.contains(&s.id)) {
        sound.mood = mood;
        sound.mood_intensity = mood.and(level_field(intensity));
    }
    sync_bindings(p);
}

/// Change the level of the key's sounds of `mood` (others untouched).
pub fn set_level_in_key(
    p: &mut Profile,
    key: &str,
    mood: BaseMood,
    intensity: Option<MoodIntensity>,
) {
    let ids: Vec<String> = p
        .key_bindings
        .iter()
        .filter(|b| b.key_code == key)
        .flat_map(|b| b.sound_ids.iter().cloned())
        .collect();
    for sound in p
        .sounds
        .iter_mut()
        .filter(|s| ids.contains(&s.id) && s.mood == Some(mood))
    {
        sound.mood_intensity = level_field(intensity);
    }
    sync_bindings(p);
}

/// Take `mood` off the sounds of a key that carry it (other moods stay).
pub fn remove_from_key(p: &mut Profile, key: &str, mood: BaseMood) {
    let ids: Vec<String> = p
        .key_bindings
        .iter()
        .filter(|b| b.key_code == key)
        .flat_map(|b| b.sound_ids.iter().cloned())
        .collect();
    for sound in p
        .sounds
        .iter_mut()
        .filter(|s| ids.contains(&s.id) && s.mood == Some(mood))
    {
        sound.mood = None;
        sound.mood_intensity = None;
    }
    sync_bindings(p);
}

/// Tag the sounds of these keys (on `track_id`) that have no mood yet.
pub fn tag_untagged(p: &mut Profile, keys: &[String], track_id: &str, mood: BaseMood) {
    let ids: Vec<String> = p
        .key_bindings
        .iter()
        .filter(|b| b.track_id == track_id && keys.contains(&b.key_code))
        .flat_map(|b| b.sound_ids.iter().cloned())
        .collect();
    for sound in p
        .sounds
        .iter_mut()
        .filter(|s| ids.contains(&s.id) && s.mood.is_none())
    {
        sound.mood = Some(mood);
    }
    sync_bindings(p);
}

/// Keep each key's derived mood fields equal to its sounds' main mood.
pub fn sync_bindings(p: &mut Profile) {
    let derived: Vec<Option<(BaseMood, MoodIntensity)>> = p
        .key_bindings
        .iter()
        .map(|b| of_binding(p, b).first().map(|(m, l, _)| (*m, *l)))
        .collect();
    for (b, main) in p.key_bindings.iter_mut().zip(derived) {
        b.mood = main.map(|(m, _)| m);
        b.mood_intensity = main.and_then(|(_, l)| level_field(Some(l)));
    }
}

/// A whole profile saved by a UI that tags keys (the web UI): a key whose
/// mood changed since `previous` passes it to its sounds.
pub fn apply_key_edits(previous: &Profile, p: &mut Profile) {
    let edits: Vec<(Vec<String>, Option<BaseMood>, Option<MoodIntensity>)> = p
        .key_bindings
        .iter()
        .filter(|b| {
            let before = previous
                .key_bindings
                .iter()
                .find(|o| o.key_code == b.key_code && o.track_id == b.track_id);
            match before {
                Some(o) => o.mood != b.mood || o.mood_intensity != b.mood_intensity,
                None => b.mood.is_some(),
            }
        })
        .map(|b| (b.sound_ids.clone(), b.mood, b.mood_intensity))
        .collect();
    for (ids, mood, intensity) in edits {
        for sound in p.sounds.iter_mut().filter(|s| ids.contains(&s.id)) {
            sound.mood = mood;
            sound.mood_intensity = mood.and(level_field(intensity));
        }
    }
    sync_bindings(p);
}

/// Profiles from before moods moved to sounds: a key's mood goes to its
/// sounds that have none. Returns whether anything changed.
pub fn migrate(p: &mut Profile) -> bool {
    let mut changed = false;
    let tagged: Vec<(Vec<String>, BaseMood, Option<MoodIntensity>)> = p
        .key_bindings
        .iter()
        .filter_map(|b| b.mood.map(|m| (b.sound_ids.clone(), m, b.mood_intensity)))
        .collect();
    let any_sound_tagged = p.sounds.iter().any(|s| s.mood.is_some());
    if any_sound_tagged {
        // Already migrated: the sounds are the source of truth.
        return false;
    }
    for (ids, mood, intensity) in tagged {
        for sound in p
            .sounds
            .iter_mut()
            .filter(|s| ids.contains(&s.id) && s.mood.is_none())
        {
            sound.mood = Some(mood);
            sound.mood_intensity = level_field(intensity);
            changed = true;
        }
    }
    if changed {
        sync_bindings(p);
    }
    changed
}

// ── Playback ──

/// What a scene plays on one track: the key, and which of its sounds fit.
#[derive(Debug, Clone)]
pub struct MoodPick {
    pub binding: KeyBinding,
    pub sound_ids: Vec<String>,
    /// The most demanding level among those sounds.
    pub level: MoodIntensity,
}

/// Same key on the same track with the same sounds: what plays can go on.
impl PartialEq for MoodPick {
    fn eq(&self, other: &Self) -> bool {
        self.binding.key_code == other.binding.key_code
            && self.binding.track_id == other.binding.track_id
            && self.sound_ids == other.sound_ids
    }
}

/// Per track, the keys whose sounds of `mood` reach the highest level not
/// above `intensity` (a climax plays its climax pieces, a calm moment the
/// calm ones), each with those sounds. Several keys can tie (one sound per
/// key): the session picks one, for variety.
pub fn select(p: &Profile, mood: BaseMood, intensity: MoodIntensity) -> Vec<Vec<MoodPick>> {
    let mut per_track: Vec<Vec<MoodPick>> = Vec::new();
    for b in &p.key_bindings {
        let sounds: Vec<&Sound> = sounds_of(p, b)
            .into_iter()
            .filter(|s| {
                of_sound(s)
                    .is_some_and(|(m, level)| m == mood && level.as_u8() <= intensity.as_u8())
            })
            .collect();
        let Some(level) = sounds
            .iter()
            .filter_map(|s| of_sound(s))
            .map(|(_, l)| l)
            .max_by_key(|l| l.as_u8())
        else {
            continue;
        };
        // At this level only: a key mixing calm and intense pieces plays
        // its intense ones at the climax.
        let sound_ids: Vec<String> = sounds
            .iter()
            .filter(|s| of_sound(s).is_some_and(|(_, l)| l == level))
            .map(|s| s.id.clone())
            .collect();
        let pick = MoodPick {
            binding: b.clone(),
            sound_ids,
            level,
        };
        match per_track
            .iter_mut()
            .find(|c| c[0].binding.track_id == b.track_id)
        {
            Some(current) if level.as_u8() > current[0].level.as_u8() => *current = vec![pick],
            Some(current) if level == current[0].level => current.push(pick),
            Some(_) => {}
            None => per_track.push(vec![pick]),
        }
    }
    per_track
}

/// The sound a mood plays from a key, among `allowed`, following the key's
/// loop mode; returns it with the key's next `current_index` (an index in
/// the key's whole list, so key presses continue from there).
pub fn pick_sound<'a>(
    binding: &KeyBinding,
    allowed: &[String],
    sounds: &'a HashMap<&str, &'a Sound>,
    played: bool,
    mut random: impl FnMut(usize) -> usize,
) -> Option<(&'a Sound, usize)> {
    let positions: Vec<usize> = binding
        .sound_ids
        .iter()
        .enumerate()
        .filter(|(_, id)| allowed.contains(id) && sounds.contains_key(id.as_str()))
        .map(|(i, _)| i)
        .collect();
    let sound_at = |i: usize| sounds.get(binding.sound_ids[i].as_str()).copied();
    let current = binding.current_index;
    match binding.loop_mode {
        LoopMode::Off | LoopMode::Random => {
            let choices: Vec<usize> = if played && positions.len() > 1 {
                positions
                    .iter()
                    .copied()
                    .filter(|&i| i != current)
                    .collect()
            } else {
                positions.clone()
            };
            let i = *choices.get(random(choices.len().max(1)))?;
            Some((sound_at(i)?, i))
        }
        LoopMode::Single => {
            let i = if positions.contains(&current) {
                current
            } else {
                *positions.first()?
            };
            Some((sound_at(i)?, i))
        }
        LoopMode::Sequential => {
            let len = binding.sound_ids.len().max(1);
            let start = current % len;
            let i = positions
                .iter()
                .copied()
                .find(|&i| i >= start)
                .or_else(|| positions.first().copied())?;
            Some((sound_at(i)?, i + 1))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::editor::tests::{binding, profile, sound};

    fn tagged(id: &str, mood: BaseMood, level: Option<MoodIntensity>) -> Sound {
        let mut s = sound(id);
        s.mood = Some(mood);
        s.mood_intensity = level;
        s
    }

    fn sound_map(p: &Profile) -> HashMap<&str, &Sound> {
        p.sounds.iter().map(|s| (s.id.as_str(), s)).collect()
    }

    #[test]
    fn old_profiles_move_key_moods_to_sounds_once() {
        let mut p = profile();
        p.sounds = vec![sound("a"), sound("b"), sound("c")];
        let mut q = binding("KeyQ", "t1", &["a", "b"]);
        q.mood = Some(BaseMood::Epic);
        q.mood_intensity = Some(MoodIntensity::High);
        p.key_bindings = vec![q, binding("KeyW", "t1", &["c"])];
        assert!(migrate(&mut p));
        assert_eq!(
            of_sound(&p.sounds[0]),
            Some((BaseMood::Epic, MoodIntensity::High))
        );
        assert_eq!(
            of_sound(&p.sounds[1]),
            Some((BaseMood::Epic, MoodIntensity::High))
        );
        assert_eq!(p.sounds[2].mood, None);
        // Sounds are now the truth: a key mood changed by an old version is ignored.
        p.key_bindings[1].mood = Some(BaseMood::Comedy);
        assert!(!migrate(&mut p));
        assert_eq!(p.sounds[2].mood, None);
    }

    #[test]
    fn a_key_mixing_moods_answers_each_with_its_sounds() {
        let mut p = profile();
        p.sounds = vec![
            tagged("calm", BaseMood::Epic, None),
            tagged("climax", BaseMood::Epic, Some(MoodIntensity::High)),
            tagged("sad", BaseMood::Sadness, None),
        ];
        p.key_bindings = vec![binding("KeyQ", "t1", &["calm", "climax", "sad"])];
        sync_bindings(&mut p);
        assert_eq!(p.key_bindings[0].mood, Some(BaseMood::Epic));
        assert_eq!(
            of_key(&p, "KeyQ"),
            vec![
                (BaseMood::Epic, MoodIntensity::Low),
                (BaseMood::Sadness, MoodIntensity::Low)
            ]
        );

        let at = |mood, level| {
            select(&p, mood, level)
                .into_iter()
                .map(|picks| picks[0].sound_ids.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            at(BaseMood::Epic, MoodIntensity::Low),
            vec![vec!["calm".to_string()]]
        );
        assert_eq!(
            at(BaseMood::Epic, MoodIntensity::High),
            vec![vec!["climax".to_string()]]
        );
        assert_eq!(
            at(BaseMood::Sadness, MoodIntensity::Medium),
            vec![vec!["sad".to_string()]]
        );
        assert!(at(BaseMood::Horror, MoodIntensity::High).is_empty());
    }

    #[test]
    fn each_track_plays_its_most_fitting_key() {
        let mut p = profile();
        p.sounds = vec![
            tagged("a", BaseMood::Tension, None),
            tagged("b", BaseMood::Tension, Some(MoodIntensity::Medium)),
            tagged("c", BaseMood::Tension, None),
        ];
        p.key_bindings = vec![
            binding("KeyQ", "t1", &["a"]),
            binding("KeyW", "t1", &["b"]),
            binding("KeyE", "t2", &["c"]),
        ];
        let keys = |p: &Profile, level| {
            select(p, BaseMood::Tension, level)
                .into_iter()
                .map(|picks| {
                    picks
                        .into_iter()
                        .map(|pick| pick.binding.key_code)
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(keys(&p, MoodIntensity::Low), [vec!["KeyQ"], vec!["KeyE"]]);
        assert_eq!(
            keys(&p, MoodIntensity::Medium),
            [vec!["KeyW"], vec!["KeyE"]]
        );
        // Keys tying at the best level are all offered.
        p.sounds[0].mood_intensity = Some(MoodIntensity::Medium);
        assert_eq!(
            keys(&p, MoodIntensity::High),
            [vec!["KeyQ", "KeyW"], vec!["KeyE"]]
        );
    }

    #[test]
    fn picking_follows_the_loop_mode_among_allowed_sounds() {
        let mut p = profile();
        p.sounds = vec![sound("a"), sound("b"), sound("c"), sound("d")];
        let map = sound_map(&p);
        let allowed = vec!["b".to_string(), "d".to_string()];
        let mut b = binding("KeyQ", "t1", &["a", "b", "c", "d"]);

        b.loop_mode = LoopMode::Sequential;
        b.current_index = 2;
        let (s, next) = pick_sound(&b, &allowed, &map, true, |_| 0).unwrap();
        assert_eq!((s.id.as_str(), next), ("d", 4));
        b.current_index = 4;
        let (s, next) = pick_sound(&b, &allowed, &map, true, |_| 0).unwrap();
        assert_eq!((s.id.as_str(), next), ("b", 2));

        b.loop_mode = LoopMode::Random;
        b.current_index = 1;
        let (s, _) = pick_sound(&b, &allowed, &map, true, |_| 0).unwrap();
        assert_eq!(s.id, "d", "avoids repeating the sound that just played");

        b.loop_mode = LoopMode::Single;
        b.current_index = 0;
        assert_eq!(
            pick_sound(&b, &allowed, &map, true, |_| 0).unwrap().0.id,
            "b"
        );
        assert!(pick_sound(&b, &[], &map, true, |_| 0).is_none());
    }

    #[test]
    fn tagging_a_key_tags_its_sounds_and_removal_keeps_other_moods() {
        let mut p = profile();
        p.sounds = vec![sound("a"), tagged("b", BaseMood::Comedy, None)];
        p.key_bindings = vec![
            binding("KeyQ", "t1", &["a"]),
            binding("KeyW", "t1", &["a", "b"]),
        ];
        set_key(
            &mut p,
            "KeyQ",
            Some(BaseMood::Epic),
            Some(MoodIntensity::Medium),
        );
        assert_eq!(
            of_sound(&p.sounds[0]),
            Some((BaseMood::Epic, MoodIntensity::Medium))
        );
        assert_eq!(
            keys_with(&p, BaseMood::Epic),
            vec![
                ("KeyQ".to_string(), MoodIntensity::Medium),
                ("KeyW".to_string(), MoodIntensity::Medium)
            ]
        );
        remove_from_key(&mut p, "KeyW", BaseMood::Epic);
        assert_eq!(p.sounds[0].mood, None);
        assert_eq!(p.sounds[1].mood, Some(BaseMood::Comedy));
        assert_eq!(p.key_bindings[1].mood, Some(BaseMood::Comedy));
    }
}
