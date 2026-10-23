//! Profile editing rules and undo history.
//!
//! Every function mutates a [`Profile`] in place and keeps its invariants:
//! one binding per (key, track), no empty bindings, and sounds are dropped once
//! no binding references them.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::types::{
    BaseMood, KeyBinding, LoopMode, MoodIntensity, Profile, Sound, SoundSource, Track,
};

pub const MAX_TRACKS: usize = 20;
pub const MAX_HISTORY: usize = 50;

pub fn find_binding<'a>(profile: &'a Profile, key: &str, track_id: &str) -> Option<&'a KeyBinding> {
    profile
        .key_bindings
        .iter()
        .find(|b| b.key_code == key && b.track_id == track_id)
}

fn find_binding_mut<'a>(
    profile: &'a mut Profile,
    key: &str,
    track_id: &str,
) -> Option<&'a mut KeyBinding> {
    profile
        .key_bindings
        .iter_mut()
        .find(|b| b.key_code == key && b.track_id == track_id)
}

/// Collapse bindings sharing (key, track) into one, keeping first-seen order.
pub fn merge_key_bindings(bindings: Vec<KeyBinding>) -> Vec<KeyBinding> {
    let mut merged: Vec<KeyBinding> = Vec::with_capacity(bindings.len());
    for binding in bindings {
        if let Some(existing) = merged
            .iter_mut()
            .find(|b| b.key_code == binding.key_code && b.track_id == binding.track_id)
        {
            existing.current_index = 0;
            for id in binding.sound_ids {
                if !existing.sound_ids.contains(&id) {
                    existing.sound_ids.push(id);
                }
            }
        } else {
            merged.push(binding);
        }
    }
    merged
}

/// Remove sounds that no binding references anymore, limited to `candidates`.
fn drop_orphans(profile: &mut Profile, candidates: impl IntoIterator<Item = String>) {
    let referenced: HashSet<&String> = profile
        .key_bindings
        .iter()
        .flat_map(|b| b.sound_ids.iter())
        .collect();
    let orphaned: HashSet<String> = candidates
        .into_iter()
        .filter(|id| !referenced.contains(id))
        .collect();
    if !orphaned.is_empty() {
        profile.sounds.retain(|s| !orphaned.contains(&s.id));
    }
}

pub fn add_track(profile: &mut Profile, name: &str) -> Result<String, String> {
    if profile.tracks.len() >= MAX_TRACKS {
        return Err(format!("Maximum {} tracks allowed", MAX_TRACKS));
    }
    let id = uuid::Uuid::new_v4().to_string();
    profile.tracks.push(Track {
        id: id.clone(),
        name: name.trim().to_string(),
        volume: 1.0,
    });
    Ok(id)
}

pub fn rename_track(profile: &mut Profile, track_id: &str, name: &str) {
    if let Some(track) = profile.tracks.iter_mut().find(|t| t.id == track_id) {
        track.name = name.trim().to_string();
    }
}

pub fn set_track_volume(profile: &mut Profile, track_id: &str, volume: f32) {
    if let Some(track) = profile.tracks.iter_mut().find(|t| t.id == track_id) {
        track.volume = volume.clamp(0.0, 1.0);
    }
}

/// Remove a track with its bindings and the sounds only they used.
pub fn remove_track(profile: &mut Profile, track_id: &str) {
    profile.tracks.retain(|t| t.id != track_id);
    let removed: Vec<String> = profile
        .key_bindings
        .iter()
        .filter(|b| b.track_id == track_id)
        .flat_map(|b| b.sound_ids.clone())
        .collect();
    profile.key_bindings.retain(|b| b.track_id != track_id);
    drop_orphans(profile, removed);
}

pub fn add_sound(profile: &mut Profile, sound: Sound) {
    profile.sounds.push(sound);
}

/// Remove a sound everywhere; bindings left empty disappear.
pub fn remove_sound(profile: &mut Profile, sound_id: &str) {
    profile.sounds.retain(|s| s.id != sound_id);
    for binding in &mut profile.key_bindings {
        binding.sound_ids.retain(|id| id != sound_id);
    }
    profile.key_bindings.retain(|b| !b.sound_ids.is_empty());
}

pub fn sound_mut<'a>(profile: &'a mut Profile, sound_id: &str) -> Option<&'a mut Sound> {
    profile.sounds.iter_mut().find(|s| s.id == sound_id)
}

/// Point a sound at a new file (e.g. "Locate file" for a missing sound).
pub fn relocate_sound(profile: &mut Profile, sound_id: &str, source: SoundSource, duration: f64) {
    if let Some(sound) = sound_mut(profile, sound_id) {
        sound.source = source;
        sound.duration = duration;
        sound.momentum = 0.0;
    }
}

/// Add or replace the binding for (key, track).
pub fn put_binding(profile: &mut Profile, binding: KeyBinding) {
    profile
        .key_bindings
        .retain(|b| !(b.key_code == binding.key_code && b.track_id == binding.track_id));
    profile.key_bindings.push(binding);
}

/// Remove one binding (or every track's binding of `key` when `track_id` is None).
pub fn remove_binding(profile: &mut Profile, key: &str, track_id: Option<&str>) {
    let matches = |b: &KeyBinding| b.key_code == key && track_id.is_none_or(|t| b.track_id == t);
    let removed: Vec<String> = profile
        .key_bindings
        .iter()
        .filter(|b| matches(b))
        .flat_map(|b| b.sound_ids.clone())
        .collect();
    profile.key_bindings.retain(|b| !matches(b));
    drop_orphans(profile, removed);
}

/// Remove one sound from a binding; drop the sound if nothing else uses it.
pub fn remove_sound_from_binding(profile: &mut Profile, key: &str, track_id: &str, sound_id: &str) {
    if let Some(binding) = find_binding_mut(profile, key, track_id) {
        binding.sound_ids.retain(|id| id != sound_id);
        binding.current_index = 0;
    }
    profile.key_bindings.retain(|b| !b.sound_ids.is_empty());
    drop_orphans(profile, [sound_id.to_string()]);
}

/// Move a whole binding (or one of its sounds) to another key, merging by track.
pub fn move_binding(
    profile: &mut Profile,
    key: &str,
    track_id: &str,
    new_key: &str,
    sound_id: Option<&str>,
) -> bool {
    let Some(source) = find_binding(profile, key, track_id).cloned() else {
        return false;
    };
    if new_key == key || sound_id.is_some_and(|id| !source.sound_ids.iter().any(|s| s == id)) {
        return false;
    }
    let moved: Vec<String> = match sound_id {
        Some(id) => vec![id.to_string()],
        None => source.sound_ids.clone(),
    };
    let remaining: Vec<String> = source
        .sound_ids
        .iter()
        .filter(|id| !moved.contains(id))
        .cloned()
        .collect();
    let mut bindings: Vec<KeyBinding> = Vec::with_capacity(profile.key_bindings.len() + 1);
    for binding in profile.key_bindings.drain(..) {
        if binding.key_code == key && binding.track_id == track_id {
            if !remaining.is_empty() {
                bindings.push(KeyBinding {
                    sound_ids: remaining.clone(),
                    current_index: 0,
                    ..binding
                });
            }
        } else {
            bindings.push(binding);
        }
    }
    bindings.push(KeyBinding {
        key_code: new_key.to_string(),
        sound_ids: moved,
        current_index: 0,
        ..source
    });
    profile.key_bindings = merge_key_bindings(bindings);
    true
}

/// Move a binding to another track (merging with that track's binding of the same key).
pub fn set_binding_track(profile: &mut Profile, key: &str, track_id: &str, new_track_id: &str) {
    if let Some(binding) = find_binding_mut(profile, key, track_id) {
        binding.track_id = new_track_id.to_string();
    }
    profile.key_bindings = merge_key_bindings(std::mem::take(&mut profile.key_bindings));
}

pub fn set_binding_loop_mode(profile: &mut Profile, key: &str, track_id: &str, mode: LoopMode) {
    if let Some(binding) = find_binding_mut(profile, key, track_id) {
        binding.loop_mode = mode;
        binding.current_index = 0;
    }
}

pub fn set_binding_name(profile: &mut Profile, key: &str, track_id: &str, name: Option<&str>) {
    if let Some(binding) = find_binding_mut(profile, key, track_id) {
        binding.name = name
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map(String::from);
    }
}

pub fn set_binding_mood(
    profile: &mut Profile,
    key: &str,
    track_id: &str,
    mood: Option<BaseMood>,
    intensity: Option<MoodIntensity>,
) {
    if let Some(binding) = find_binding_mut(profile, key, track_id) {
        binding.mood = mood;
        binding.mood_intensity = mood.and(intensity);
    }
}

/// Tag every binding of `key` (all tracks) with a mood.
pub fn set_key_mood(
    profile: &mut Profile,
    key: &str,
    mood: Option<BaseMood>,
    intensity: Option<MoodIntensity>,
) {
    for binding in profile
        .key_bindings
        .iter_mut()
        .filter(|b| b.key_code == key)
    {
        binding.mood = mood;
        binding.mood_intensity = mood.and(intensity);
    }
}

/// Insert or move sounds within a binding's playback order.
pub fn reorder_binding_sound(
    profile: &mut Profile,
    key: &str,
    track_id: &str,
    sound_id: &str,
    to: usize,
) {
    if let Some(binding) = find_binding_mut(profile, key, track_id) {
        if let Some(from) = binding.sound_ids.iter().position(|id| id == sound_id) {
            let id = binding.sound_ids.remove(from);
            let to = to.min(binding.sound_ids.len());
            binding.sound_ids.insert(to, id);
            binding.current_index = 0;
        }
    }
}

/// A sound staged for [`add_sounds`].
#[derive(Debug, Clone)]
pub struct NewSound {
    pub name: String,
    pub source: SoundSource,
    pub momentum: f64,
    pub duration: f64,
}

/// Add sounds and spread them round-robin over `keys` on `track_id`. A key that
/// already has a binding on this track gets the sounds appended; otherwise a new
/// binding is created with `loop_mode`. Returns the new sound ids in order.
pub fn add_sounds(
    profile: &mut Profile,
    sounds: Vec<NewSound>,
    keys: &[String],
    track_id: &str,
    volume: f32,
    loop_mode: LoopMode,
) -> Vec<String> {
    if keys.is_empty() {
        return Vec::new();
    }
    let mut groups: Vec<(String, Vec<String>)> = Vec::new();
    let mut ids = Vec::with_capacity(sounds.len());
    for (i, new_sound) in sounds.into_iter().enumerate() {
        let id = uuid::Uuid::new_v4().to_string();
        profile.sounds.push(Sound {
            id: id.clone(),
            name: new_sound.name,
            source: new_sound.source,
            momentum: new_sound.momentum.max(0.0),
            volume: volume.clamp(0.0, 1.0),
            duration: new_sound.duration,
            resolved_video_id: None,
        });
        let key = &keys[i % keys.len()];
        match groups.iter_mut().find(|(k, _)| k == key) {
            Some((_, list)) => list.push(id.clone()),
            None => groups.push((key.clone(), vec![id.clone()])),
        }
        ids.push(id);
    }
    for (key, new_ids) in groups {
        if let Some(binding) = find_binding_mut(profile, &key, track_id) {
            binding.sound_ids.extend(new_ids);
        } else {
            profile.key_bindings.push(KeyBinding {
                key_code: key,
                track_id: track_id.to_string(),
                sound_ids: new_ids,
                loop_mode: loop_mode.clone(),
                current_index: 0,
                name: None,
                mood: None,
                mood_intensity: None,
            });
        }
    }
    ids
}

/// Track with the fewest bindings (first track on ties).
pub fn least_used_track(profile: &Profile) -> Option<String> {
    let mut counts: HashMap<&str, usize> =
        profile.tracks.iter().map(|t| (t.id.as_str(), 0)).collect();
    for binding in &profile.key_bindings {
        *counts.entry(binding.track_id.as_str()).or_default() += 1;
    }
    profile
        .tracks
        .iter()
        .min_by_key(|t| counts.get(t.id.as_str()).copied().unwrap_or(0))
        .map(|t| t.id.clone())
}

/// Snapshot of the undoable parts of a profile.
#[derive(Debug, Clone)]
pub struct ProfileSnapshot {
    pub sounds: Vec<Sound>,
    pub key_bindings: Vec<KeyBinding>,
    pub tracks: Vec<Track>,
}

impl ProfileSnapshot {
    pub fn capture(profile: &Profile) -> Self {
        Self {
            sounds: profile.sounds.clone(),
            key_bindings: profile.key_bindings.clone(),
            tracks: profile.tracks.clone(),
        }
    }

    /// Restore the snapshot, keeping the live non-undoable state of whatever still
    /// exists: sound and track volumes, computed durations and loop positions.
    pub fn apply(mut self, profile: &mut Profile) {
        let live_sounds: HashMap<&str, &Sound> =
            profile.sounds.iter().map(|s| (s.id.as_str(), s)).collect();
        for sound in &mut self.sounds {
            if let Some(live) = live_sounds.get(sound.id.as_str()) {
                sound.volume = live.volume;
                if same_source(&sound.source, &live.source) {
                    sound.duration = live.duration;
                    sound.resolved_video_id = live.resolved_video_id.clone();
                }
            }
        }
        for track in &mut self.tracks {
            if let Some(live) = profile.tracks.iter().find(|t| t.id == track.id) {
                track.volume = live.volume;
            }
        }
        for binding in &mut self.key_bindings {
            if let Some(live) = find_binding(profile, &binding.key_code, &binding.track_id)
                .filter(|live| live.sound_ids == binding.sound_ids)
            {
                binding.current_index = live.current_index;
            }
        }
        profile.sounds = self.sounds;
        profile.key_bindings = self.key_bindings;
        profile.tracks = self.tracks;
    }
}

fn same_source(a: &SoundSource, b: &SoundSource) -> bool {
    match (a, b) {
        (SoundSource::Local { path: a }, SoundSource::Local { path: b }) => a == b,
        (
            SoundSource::YouTube {
                url: a,
                cached_path: ca,
            },
            SoundSource::YouTube {
                url: b,
                cached_path: cb,
            },
        ) => a == b && ca == cb,
        _ => false,
    }
}

#[derive(Debug, Clone)]
struct HistoryEntry {
    action: String,
    before: ProfileSnapshot,
    after: ProfileSnapshot,
}

/// Bounded undo/redo stacks of profile snapshots.
#[derive(Debug, Default)]
pub struct History {
    past: VecDeque<HistoryEntry>,
    future: Vec<HistoryEntry>,
}

impl History {
    pub fn push(
        &mut self,
        action: impl Into<String>,
        before: ProfileSnapshot,
        after: ProfileSnapshot,
    ) {
        self.past.push_back(HistoryEntry {
            action: action.into(),
            before,
            after,
        });
        while self.past.len() > MAX_HISTORY {
            self.past.pop_front();
        }
        self.future.clear();
    }

    /// Restore the state before the last action; returns its name.
    pub fn undo(&mut self, profile: &mut Profile) -> Option<String> {
        let entry = self.past.pop_back()?;
        entry.before.clone().apply(profile);
        let name = entry.action.clone();
        self.future.push(entry);
        Some(name)
    }

    pub fn redo(&mut self, profile: &mut Profile) -> Option<String> {
        let entry = self.future.pop()?;
        entry.after.clone().apply(profile);
        let name = entry.action.clone();
        self.past.push_back(entry);
        Some(name)
    }

    pub fn clear(&mut self) {
        self.past.clear();
        self.future.clear();
    }

    pub fn undo_name(&self) -> Option<&str> {
        self.past.back().map(|e| e.action.as_str())
    }

    pub fn redo_name(&self) -> Option<&str> {
        self.future.last().map(|e| e.action.as_str())
    }

    /// Most recent first.
    pub fn past_actions(&self) -> Vec<String> {
        self.past.iter().rev().map(|e| e.action.clone()).collect()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn sound(id: &str) -> Sound {
        Sound {
            id: id.into(),
            name: id.into(),
            source: SoundSource::Local {
                path: format!("/tmp/{id}.mp3"),
            },
            momentum: 0.0,
            volume: 1.0,
            duration: 10.0,
            resolved_video_id: None,
        }
    }

    pub fn binding(key: &str, track: &str, ids: &[&str]) -> KeyBinding {
        KeyBinding {
            key_code: key.into(),
            track_id: track.into(),
            sound_ids: ids.iter().map(|s| s.to_string()).collect(),
            loop_mode: LoopMode::Off,
            current_index: 0,
            name: None,
            mood: None,
            mood_intensity: None,
        }
    }

    pub fn profile() -> Profile {
        Profile {
            id: "p".into(),
            name: "P".into(),
            created_at: String::new(),
            updated_at: String::new(),
            sounds: vec![sound("s1"), sound("s2"), sound("s3")],
            tracks: vec![
                Track {
                    id: "t1".into(),
                    name: "OST".into(),
                    volume: 1.0,
                },
                Track {
                    id: "t2".into(),
                    name: "SFX".into(),
                    volume: 1.0,
                },
            ],
            key_bindings: vec![
                binding("KeyA", "t1", &["s1", "s2"]),
                binding("KeyB", "t2", &["s3"]),
            ],
            disliked_videos: vec![],
        }
    }

    #[test]
    fn removing_a_binding_drops_orphan_sounds() {
        let mut p = profile();
        remove_binding(&mut p, "KeyA", Some("t1"));
        assert_eq!(p.sounds.len(), 1);
        assert_eq!(p.key_bindings.len(), 1);
    }

    #[test]
    fn moving_a_sound_merges_with_existing_binding() {
        let mut p = profile();
        p.key_bindings.push(binding("KeyB", "t1", &["s3"]));
        assert!(move_binding(&mut p, "KeyA", "t1", "KeyB", Some("s1")));
        let b = find_binding(&p, "KeyB", "t1").unwrap();
        assert_eq!(b.sound_ids, vec!["s3", "s1"]);
        assert_eq!(
            find_binding(&p, "KeyA", "t1").unwrap().sound_ids,
            vec!["s2"]
        );
    }

    #[test]
    fn moving_whole_binding_leaves_no_empty_binding() {
        let mut p = profile();
        assert!(move_binding(&mut p, "KeyA", "t1", "KeyC", None));
        assert!(find_binding(&p, "KeyA", "t1").is_none());
        assert_eq!(find_binding(&p, "KeyC", "t1").unwrap().sound_ids.len(), 2);
    }

    #[test]
    fn add_sounds_round_robin_and_append() {
        let mut p = profile();
        let staged = (0..3)
            .map(|i| NewSound {
                name: format!("n{i}"),
                source: SoundSource::Local {
                    path: format!("/x/{i}.mp3"),
                },
                momentum: 0.0,
                duration: 1.0,
            })
            .collect();
        let ids = add_sounds(
            &mut p,
            staged,
            &["KeyA".into(), "KeyC".into()],
            "t1",
            0.5,
            LoopMode::Random,
        );
        assert_eq!(ids.len(), 3);
        assert_eq!(find_binding(&p, "KeyA", "t1").unwrap().sound_ids.len(), 4);
        let c = find_binding(&p, "KeyC", "t1").unwrap();
        assert_eq!(c.sound_ids.len(), 1);
        assert!(matches!(c.loop_mode, LoopMode::Random));
    }

    #[test]
    fn removing_a_track_drops_its_bindings_and_orphans() {
        let mut p = profile();
        remove_track(&mut p, "t2");
        assert_eq!(p.tracks.len(), 1);
        assert!(p.sounds.iter().all(|s| s.id != "s3"));
    }

    #[test]
    fn history_undo_redo_round_trip() {
        let mut p = profile();
        let mut history = History::default();
        let before = ProfileSnapshot::capture(&p);
        remove_sound(&mut p, "s1");
        history.push("Remove sound", before, ProfileSnapshot::capture(&p));
        assert_eq!(history.undo(&mut p).as_deref(), Some("Remove sound"));
        assert_eq!(p.sounds.len(), 3);
        assert_eq!(history.redo(&mut p).as_deref(), Some("Remove sound"));
        assert_eq!(p.sounds.len(), 2);
    }

    #[test]
    fn undo_keeps_live_volumes_durations_and_positions() {
        let mut p = profile();
        let mut history = History::default();
        let before = ProfileSnapshot::capture(&p);
        set_binding_name(&mut p, "KeyA", "t1", Some("Theme"));
        history.push("Rename", before, ProfileSnapshot::capture(&p));

        // Non-undoable changes made after the action.
        sound_mut(&mut p, "s1").unwrap().volume = 0.3;
        sound_mut(&mut p, "s2").unwrap().duration = 42.0;
        set_track_volume(&mut p, "t1", 0.5);
        p.key_bindings[0].current_index = 1;

        history.undo(&mut p);
        assert_eq!(find_binding(&p, "KeyA", "t1").unwrap().name, None);
        assert_eq!(sound_mut(&mut p, "s1").unwrap().volume, 0.3);
        assert_eq!(sound_mut(&mut p, "s2").unwrap().duration, 42.0);
        assert_eq!(p.tracks[0].volume, 0.5);
        assert_eq!(find_binding(&p, "KeyA", "t1").unwrap().current_index, 1);
    }

    #[test]
    fn undo_relocate_restores_the_old_duration() {
        let mut p = profile();
        let mut history = History::default();
        let before = ProfileSnapshot::capture(&p);
        let new_source = SoundSource::Local {
            path: "/elsewhere.mp3".into(),
        };
        relocate_sound(&mut p, "s1", new_source, 99.0);
        history.push("Locate file", before, ProfileSnapshot::capture(&p));
        history.undo(&mut p);
        assert_eq!(sound_mut(&mut p, "s1").unwrap().duration, 10.0);
    }

    #[test]
    fn least_used_track_prefers_fewest_bindings() {
        let mut p = profile();
        p.key_bindings.push(binding("KeyC", "t1", &["s1"]));
        assert_eq!(least_used_track(&p).as_deref(), Some("t2"));
    }
}
