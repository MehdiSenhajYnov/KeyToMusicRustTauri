//! Pure playback and assignment rules, shared by key, loop and mood triggers.

use std::collections::{HashMap, HashSet};

use crate::types::{BaseMood, KeyBinding, LoopMode, MoodIntensity, Profile, Sound, SoundSource};


/// How a loop replays its next sound's start position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MomentumBehavior {
    /// Key triggers: momentum only with Auto-Momentum (or the persisted single-loop momentum).
    ConfigOnly,
    /// Mood triggers: always use a sound's own momentum when it has one.
    PreferSoundMomentum,
}

/// What keeps a track looping after its sound finishes.
#[derive(Debug, Clone, PartialEq)]
pub struct LoopContext {
    pub key_code: String,
    pub track_id: String,
    /// Sound this context was started for; other sounds ending on the track are ignored.
    pub sound_id: String,
    pub momentum: MomentumBehavior,
    pub persist_single_momentum: bool,
}

/// Choose the sound to play and the binding's next `current_index`.
/// `played` says whether this binding already played in this session: until then
/// `current_index` is only a persisted default, so random modes must not avoid it.
/// `random` yields a value in `0..n` and is injected for tests.
pub fn select_loop_sound<'a>(
    binding: &KeyBinding,
    sounds: &'a HashMap<&str, &'a Sound>,
    played: bool,
    mut random: impl FnMut(usize) -> usize,
) -> Option<(&'a Sound, usize)> {
    let available: Vec<&Sound> = binding
        .sound_ids
        .iter()
        .filter_map(|id| sounds.get(id.as_str()).copied())
        .collect();
    if available.is_empty() {
        return None;
    }
    let current = binding.current_index;
    match binding.loop_mode {
        LoopMode::Off | LoopMode::Random => {
            if available.len() == 1 {
                return Some((available[0], 0));
            }
            let mut idx = random(available.len());
            while played && idx == current {
                idx = random(available.len());
            }
            Some((available[idx], idx))
        }
        LoopMode::Single => {
            let idx = current.min(available.len() - 1);
            Some((available[idx], idx))
        }
        LoopMode::Sequential => {
            let idx = current % available.len();
            Some((available[idx], idx + 1))
        }
    }
}

pub fn random_index(n: usize) -> usize {
    use std::sync::atomic::{AtomicU64, Ordering};
    // xorshift seeded from the clock: good enough for picking a sound.
    static STATE: AtomicU64 = AtomicU64::new(0);
    let mut x = STATE.load(Ordering::Relaxed);
    if x == 0 {
        x = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15)
            | 1;
    }
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    STATE.store(x, Ordering::Relaxed);
    (x % n.max(1) as u64) as usize
}

/// Start position when a loop moves on to its next sound.
pub fn loop_replay_start(
    sound: &Sound,
    mode: &LoopMode,
    auto_momentum: bool,
    ctx: &LoopContext,
) -> f64 {
    if matches!(mode, LoopMode::Single) && ctx.persist_single_momentum {
        return sound.momentum;
    }
    match ctx.momentum {
        MomentumBehavior::PreferSoundMomentum if auto_momentum || sound.momentum > 0.0 => {
            sound.momentum
        }
        MomentumBehavior::PreferSoundMomentum => 0.0,
        MomentumBehavior::ConfigOnly if auto_momentum => sound.momentum,
        MomentumBehavior::ConfigOnly => 0.0,
    }
}

/// Per track, the tagged binding with the highest minimum intensity not above `intensity`.
pub fn select_mood_bindings(
    bindings: &[KeyBinding],
    mood: BaseMood,
    intensity: MoodIntensity,
) -> Vec<&KeyBinding> {
    let level = |b: &KeyBinding| b.mood_intensity.map_or(1, |i| i.as_u8());
    let mut per_track: Vec<&KeyBinding> = Vec::new();
    for binding in bindings {
        if binding.mood != Some(mood)
            || level(binding) > intensity.as_u8()
            || binding.sound_ids.is_empty()
        {
            continue;
        }
        match per_track
            .iter_mut()
            .find(|b| b.track_id == binding.track_id)
        {
            Some(current) if level(binding) > level(current) => *current = binding,
            Some(_) => {}
            None => per_track.push(binding),
        }
    }
    per_track
}

pub fn mood_display(mood: BaseMood) -> &'static str {
    match mood {
        BaseMood::Epic => "Epic",
        BaseMood::Tension => "Tension",
        BaseMood::Sadness => "Sadness",
        BaseMood::Comedy => "Comedy",
        BaseMood::Romance => "Romance",
        BaseMood::Horror => "Horror",
        BaseMood::Peaceful => "Peaceful",
        BaseMood::Mystery => "Mystery",
    }
}

pub fn intensity_display(intensity: MoodIntensity) -> &'static str {
    match intensity {
        MoodIntensity::Low => "Calm",
        MoodIntensity::Medium => "Moderate",
        MoodIntensity::High => "Intense",
    }
}

pub(crate) fn youtube_video_id(url: &str) -> Option<&str> {
    if let Some(pos) = url.find("v=") {
        let start = pos + 2;
        let preceded_ok = pos > 0 && matches!(url.as_bytes()[pos - 1], b'?' | b'&');
        if preceded_ok {
            let rest = &url[start..];
            return Some(rest.split(['&', '#']).next().unwrap_or(rest));
        }
    }
    url.split("youtu.be/")
        .nth(1)
        .map(|rest| rest.split(['?', '&', '#']).next().unwrap_or(rest))
}

/// The mood a sound found from `seed_video_ids` most likely shares: the most
/// common mood among the bindings that play those seeds (None if untagged).
pub fn seed_mood(profile: &Profile, seed_video_ids: &[String]) -> Option<BaseMood> {
    let seeds: HashSet<&str> = seed_video_ids.iter().map(String::as_str).collect();
    let seed_sounds: HashSet<&str> = profile
        .sounds
        .iter()
        .filter(|s| {
            let video = match &s.source {
                SoundSource::YouTube { url, .. } => youtube_video_id(url),
                SoundSource::Local { .. } => s.resolved_video_id.as_deref(),
            };
            video.is_some_and(|v| seeds.contains(v))
        })
        .map(|s| s.id.as_str())
        .collect();
    let mut counts: Vec<(BaseMood, usize)> = Vec::new();
    for binding in &profile.key_bindings {
        let Some(mood) = binding.mood else { continue };
        if !binding.sound_ids.iter().any(|id| seed_sounds.contains(id.as_str())) {
            continue;
        }
        match counts.iter_mut().find(|(m, _)| *m == mood) {
            Some((_, n)) => *n += 1,
            None => counts.push((mood, 1)),
        }
    }
    // First seen wins a tie.
    counts
        .into_iter()
        .rev()
        .max_by_key(|(_, n)| *n)
        .map(|(mood, _)| mood)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::editor::tests::{binding, profile, sound};

    fn sound_map(sounds: &[Sound]) -> HashMap<&str, &Sound> {
        sounds.iter().map(|s| (s.id.as_str(), s)).collect()
    }

    #[test]
    fn sequential_cycles_and_random_avoids_repeat() {
        let sounds = vec![sound("a"), sound("b"), sound("c")];
        let map = sound_map(&sounds);
        let mut b = binding("KeyA", "t", &["a", "b", "c"]);
        b.loop_mode = LoopMode::Sequential;
        b.current_index = 4;
        let (s, next) = select_loop_sound(&b, &map, true, |_| 0).unwrap();
        assert_eq!((s.id.as_str(), next), ("b", 2));

        b.loop_mode = LoopMode::Random;
        b.current_index = 0;
        let mut seq = [0, 0, 2].into_iter();
        let (s, next) = select_loop_sound(&b, &map, true, |_| seq.next().unwrap()).unwrap();
        assert_eq!((s.id.as_str(), next), ("c", 2));
    }

    #[test]
    fn random_first_play_can_pick_the_first_sound() {
        let sounds = vec![sound("a"), sound("b")];
        let map = sound_map(&sounds);
        for mode in [LoopMode::Off, LoopMode::Random] {
            let mut b = binding("KeyA", "t", &["a", "b"]);
            b.loop_mode = mode;
            let (s, next) = select_loop_sound(&b, &map, false, |_| 0).unwrap();
            assert_eq!((s.id.as_str(), next), ("a", 0));
        }
    }

    #[test]
    fn single_clamps_and_missing_sounds_are_skipped() {
        let sounds = vec![sound("a")];
        let map = sound_map(&sounds);
        let mut b = binding("KeyA", "t", &["gone", "a"]);
        b.loop_mode = LoopMode::Single;
        b.current_index = 5;
        let (s, next) = select_loop_sound(&b, &map, true, |_| 0).unwrap();
        assert_eq!((s.id.as_str(), next), ("a", 0));
        assert!(select_loop_sound(&binding("K", "t", &["gone"]), &map, true, |_| 0).is_none());
    }

    #[test]
    fn mood_selection_prefers_highest_eligible_intensity_per_track() {
        let mut low = binding("KeyA", "t1", &["s1"]);
        low.mood = Some(BaseMood::Epic);
        let mut high = binding("KeyB", "t1", &["s2"]);
        high.mood = Some(BaseMood::Epic);
        high.mood_intensity = Some(MoodIntensity::High);
        let mut mid = binding("KeyC", "t2", &["s3"]);
        mid.mood = Some(BaseMood::Epic);
        mid.mood_intensity = Some(MoodIntensity::Medium);
        let bindings = vec![low, high, mid];

        let picked = select_mood_bindings(&bindings, BaseMood::Epic, MoodIntensity::Medium);
        let keys: Vec<&str> = picked.iter().map(|b| b.key_code.as_str()).collect();
        assert_eq!(keys, vec!["KeyA", "KeyC"]);
        let picked = select_mood_bindings(&bindings, BaseMood::Epic, MoodIntensity::High);
        assert_eq!(picked[0].key_code, "KeyB");
    }

    #[test]
    fn replay_start_follows_behavior() {
        let mut s = sound("a");
        s.momentum = 12.0;
        let ctx = LoopContext {
            key_code: "KeyA".into(),
            track_id: "t".into(),
            sound_id: "a".into(),
            momentum: MomentumBehavior::ConfigOnly,
            persist_single_momentum: false,
        };
        assert_eq!(loop_replay_start(&s, &LoopMode::Random, false, &ctx), 0.0);
        let mood_ctx = LoopContext {
            momentum: MomentumBehavior::PreferSoundMomentum,
            ..ctx.clone()
        };
        assert_eq!(
            loop_replay_start(&s, &LoopMode::Random, false, &mood_ctx),
            12.0
        );
        let single = LoopContext {
            persist_single_momentum: true,
            ..ctx
        };
        assert_eq!(
            loop_replay_start(&s, &LoopMode::Single, false, &single),
            12.0
        );
    }

    #[test]
    fn suggestions_inherit_the_mood_of_their_seeds() {
        let mut p = profile();
        p.sounds[0].source = SoundSource::YouTube {
            url: "https://www.youtube.com/watch?v=seed1".into(),
            cached_path: String::new(),
        };
        p.sounds[2].resolved_video_id = Some("seed2".into());
        p.key_bindings[0].mood = Some(BaseMood::Epic); // KeyA: s1, s2
        p.key_bindings[1].mood = Some(BaseMood::Sadness); // KeyB: s3
        assert_eq!(seed_mood(&p, &["seed1".into()]), Some(BaseMood::Epic));
        assert_eq!(seed_mood(&p, &["seed2".into()]), Some(BaseMood::Sadness));
        assert_eq!(seed_mood(&p, &["other".into()]), None);
    }

    #[test]
    fn extracts_video_ids() {
        assert_eq!(
            youtube_video_id("https://www.youtube.com/watch?v=abc&t=1"),
            Some("abc")
        );
        assert_eq!(youtube_video_id("https://youtu.be/xyz?si=1"), Some("xyz"));
        assert_eq!(youtube_video_id("https://example.com/?nov=1"), None);
    }
}
