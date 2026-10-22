//! A complete KeyToMusic session for native hosts.
//!
//! The Tauri app keeps this logic in its web frontend; a native UI instead owns
//! one [`Session`], which:
//! - starts every core service and owns the tokio runtime,
//! - resolves key presses, loop auto-replay and Mood AI commits into playback,
//! - holds the current profile with undo/redo and debounced, ordered saves,
//! - reports everything to the host through [`SessionEvent`]s (any thread).

pub mod discovery;
pub mod editor;
pub mod keymap;
pub mod rules;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};

use crate::audio::engine::SoundEndedReason;
use crate::events::CoreEvent;
use crate::service::{self, library::MissingSoundInfo};
use crate::state::AppState;
use crate::storage::{self, ProfileSummary};
use crate::types::{AppConfig, BaseMood, KeyBinding, LoopMode, MoodIntensity, Profile, Sound};
use crate::{runtime, types};

use editor::{History, ProfileSnapshot};
use rules::{LoopContext, MomentumBehavior};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveStatus {
    Saved,
    Pending,
    Saving,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Warning,
    Error,
}

/// What the host UI should react to.
#[derive(Debug, Clone)]
pub enum SessionEvent {
    /// Raw core event (playback progress, downloads, mood status, ...).
    Core(CoreEvent),
    /// The current profile was replaced (switch, create, import, delete).
    ProfileLoaded,
    /// The current profile was edited (including undo/redo and computed durations).
    ProfileChanged,
    ProfilesListChanged,
    HistoryChanged,
    ConfigChanged,
    SaveStatus(SaveStatus),
    /// A bound key fired (for press feedback), before playback is decoded.
    KeyTriggered {
        key_code: String,
    },
    /// Profile sounds whose files are missing on disk.
    MissingSounds(Vec<MissingSoundInfo>),
    /// Mood AI started tagged keys.
    MoodPlayback {
        mood: BaseMood,
        intensity: MoodIntensity,
        previous: Option<BaseMood>,
        keys: Vec<String>,
    },
    /// The discovery carousel changed; read [`Session::discovery_snapshot`].
    DiscoveryChanged,
    Toast {
        kind: ToastKind,
        message: String,
    },
}

pub type HostSink = Arc<dyn Fn(SessionEvent) + Send + Sync>;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MoodState {
    pub detected: Option<(BaseMood, MoodIntensity)>,
    pub committed: Option<(BaseMood, MoodIntensity)>,
}

/// What a track is playing right now.
#[derive(Debug, Clone, PartialEq)]
pub struct NowPlaying {
    pub sound_id: String,
    pub position: f64,
}

#[derive(Default)]
struct Data {
    profile: Option<Profile>,
    history: History,
    loop_contexts: HashMap<String, LoopContext>,
    pending_tracks: HashSet<String>,
    playing: HashMap<String, NowPlaying>,
    active_mood: Option<(BaseMood, MoodIntensity)>,
    committed_mood: Option<(BaseMood, MoodIntensity)>,
    detected_mood: Option<(BaseMood, MoodIntensity)>,
    last_trigger: Option<Instant>,
    trigger_pending: bool,
    loading: bool,
    load_gen: u64,
    save_gen: u64,
    dirty: bool,
    save_status: Option<SaveStatus>,
}

pub struct Session {
    state: Arc<AppState>,
    runtime: tokio::runtime::Runtime,
    host: HostSink,
    data: Mutex<Data>,
    discovery: Mutex<discovery::DiscoveryData>,
    save_lock: tokio::sync::Mutex<()>,
    me: Weak<Session>,
}

const SAVE_DELAY: Duration = Duration::from_millis(300);
/// Track id used for previews so they never interrupt the profile's tracks.
pub const PREVIEW_TRACK: &str = "__preview__";

impl Session {
    /// Start every core service. Call once, after logging is initialized.
    pub fn start(host: HostSink, error_sound_candidates: Vec<PathBuf>) -> Arc<Session> {
        runtime::init_directories();
        let state = Arc::new(runtime::build_state(runtime::load_config()));
        let tokio = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("keytomusic-rt")
            .build()
            .expect("failed to build tokio runtime");

        let session = Arc::new_cyclic(|me| Session {
            state: state.clone(),
            runtime: tokio,
            host,
            data: Mutex::new(Data::default()),
            discovery: Mutex::new(discovery::DiscoveryData::default()),
            save_lock: tokio::sync::Mutex::new(()),
            me: me.clone(),
        });

        let weak = Arc::downgrade(&session);
        state.set_event_sink(move |event: CoreEvent| {
            if let Some(session) = weak.upgrade() {
                session.on_core_event(event);
            }
        });

        // Spawned threads that create their own tokio work need a runtime context.
        let _guard = session.runtime.enter();
        runtime::spawn_audio_engine(&state, error_sound_candidates);
        runtime::start_key_detection(&state);
        runtime::spawn_background_maintenance(&state);
        session
    }

    pub fn state(&self) -> &Arc<AppState> {
        &self.state
    }

    /// Runtime for host-initiated async work (`handle().spawn(...)`).
    pub fn handle(&self) -> tokio::runtime::Handle {
        self.runtime.handle().clone()
    }

    pub fn config(&self) -> AppConfig {
        self.state.get_config()
    }

    fn arc(&self) -> Arc<Session> {
        self.me.upgrade().expect("session dropped")
    }

    fn emit(&self, event: SessionEvent) {
        (self.host)(event);
    }

    fn toast(&self, kind: ToastKind, message: impl Into<String>) {
        self.emit(SessionEvent::Toast {
            kind,
            message: message.into(),
        });
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Data> {
        self.data.lock().unwrap_or_else(|e| e.into_inner())
    }

    // ─── Core events ────────────────────────────────────────────────────

    fn on_core_event(&self, event: CoreEvent) {
        match &event {
            CoreEvent::KeyPressed { key_code, .. } => {
                let session = self.arc();
                let key = key_code.clone();
                self.runtime
                    .spawn(async move { session.handle_key(key, false).await });
            }
            CoreEvent::SoundStarted { track_id, sound_id } => {
                self.lock().playing.insert(
                    track_id.clone(),
                    NowPlaying {
                        sound_id: sound_id.clone(),
                        position: 0.0,
                    },
                );
            }
            CoreEvent::PlaybackProgress { track_id, position } => {
                if let Some(now) = self.lock().playing.get_mut(track_id) {
                    now.position = *position;
                }
            }
            CoreEvent::SoundEnded {
                track_id, reason, ..
            } => {
                let finished = matches!(reason, SoundEndedReason::Finished);
                {
                    let mut data = self.lock();
                    data.playing.remove(track_id);
                    if !finished {
                        data.loop_contexts.remove(track_id);
                    }
                }
                if finished {
                    let session = self.arc();
                    let track = track_id.clone();
                    self.runtime
                        .spawn(async move { session.handle_loop_end(track).await });
                }
            }
            CoreEvent::StopAllTriggered => {
                let mut data = self.lock();
                data.loop_contexts.clear();
                data.active_mood = None;
            }
            CoreEvent::ToggleKeyDetection => {
                let enabled = !self.config().key_detection_enabled;
                service::config::set_key_detection(&self.state, enabled);
                self.emit(SessionEvent::ConfigChanged);
            }
            CoreEvent::ToggleAutoMomentum => self.set_auto_momentum(!self.config().auto_momentum),
            CoreEvent::KeyDetectionBackendWarning { message } => {
                self.toast(ToastKind::Error, message.clone())
            }
            CoreEvent::MoodDetected {
                mood, intensity, ..
            } => {
                if let Some(mood) = BaseMood::from_str_opt(mood) {
                    self.lock().detected_mood = Some((mood, MoodIntensity::from_u8(*intensity)));
                }
            }
            CoreEvent::MoodCommitted {
                mood, intensity, ..
            } => {
                if let Some(mood) = BaseMood::from_str_opt(mood) {
                    let session = self.arc();
                    let intensity = MoodIntensity::from_u8((*intensity).clamp(1, 3));
                    self.runtime
                        .spawn(async move { session.handle_mood(mood, intensity).await });
                }
            }
            _ => {}
        }
        self.discovery_on_event(&event);
        self.emit(SessionEvent::Core(event));
    }

    // ─── Playback ───────────────────────────────────────────────────────

    /// Decode and start one sound, serialized per track like the web frontend.
    async fn play_on_track(
        &self,
        track_id: &str,
        sound: &Sound,
        start: f64,
    ) -> Result<bool, String> {
        if !self.lock().pending_tracks.insert(track_id.to_string()) {
            return Ok(false);
        }
        let track_volume = self
            .lock()
            .profile
            .as_ref()
            .and_then(|p| p.tracks.iter().find(|t| t.id == track_id).map(|t| t.volume));
        let result = service::playback::play_sound(
            &self.state,
            track_id.to_string(),
            sound.id.clone(),
            service::library::sound_file_path(sound),
            start,
            sound.volume.clamp(0.0, 1.0),
            track_volume,
        )
        .await;
        self.lock().pending_tracks.remove(track_id);
        result.map(|_| true)
    }

    /// Advance a binding after it played, unless it was edited meanwhile.
    fn commit_binding_index(&self, profile_id: &str, original: &KeyBinding, next_index: usize) {
        let mut data = self.lock();
        let Some(profile) = data.profile.as_mut().filter(|p| p.id == profile_id) else {
            return;
        };
        if let Some(binding) = profile.key_bindings.iter_mut().find(|b| {
            b.key_code == original.key_code
                && b.track_id == original.track_id
                && b.sound_ids == original.sound_ids
                && std::mem::discriminant(&b.loop_mode)
                    == std::mem::discriminant(&original.loop_mode)
        }) {
            binding.current_index = next_index;
            drop(data);
            self.schedule_save();
        }
    }

    fn report_playback_error(&self, error: &str) {
        if !error.contains("not found") && !error.contains("Playback cancelled") {
            self.toast(ToastKind::Error, error.to_string());
        }
    }

    /// Plan playback for bindings: (binding, sound, next index, start position).
    fn plan(
        profile: &Profile,
        bindings: &[KeyBinding],
        start: impl Fn(&Sound) -> f64,
    ) -> Vec<(KeyBinding, Sound, usize, f64)> {
        let sounds: HashMap<&str, &Sound> =
            profile.sounds.iter().map(|s| (s.id.as_str(), s)).collect();
        bindings
            .iter()
            .filter_map(|binding| {
                rules::select_loop_sound(binding, &sounds, rules::random_index)
                    .map(|(sound, next)| (binding.clone(), sound.clone(), next, start(sound)))
            })
            .collect()
    }

    /// `manual` triggers (UI clicks) ignore the "detection disabled" preference.
    async fn handle_key(self: Arc<Self>, key_code: String, manual: bool) {
        let config = self.config();
        if !manual && !config.key_detection_enabled {
            return;
        }

        let (profile_id, plan) = {
            let mut data = self.lock();
            if data.loading {
                return;
            }
            let Some(profile) = data.profile.as_ref() else {
                return;
            };

            let find = |code: &str| -> Vec<KeyBinding> {
                profile
                    .key_bindings
                    .iter()
                    .filter(|b| b.key_code == code)
                    .cloned()
                    .collect()
            };
            let canonical = if key_code.contains('+') {
                keymap::normalize_combo(&key_code)
            } else {
                key_code.clone()
            };
            let mut bindings = find(&canonical);
            let mut use_momentum = false;
            // [Modifier]+A falls back to the "A" binding, with momentum if it is the momentum modifier.
            if bindings.is_empty() && canonical.contains('+') {
                bindings = find(keymap::base_key(&canonical));
                use_momentum = !bindings.is_empty()
                    && keymap::has_modifier(&canonical, &config.momentum_modifier);
            }
            if bindings.is_empty() {
                return;
            }

            // Reserve the trigger before decoding, which may outlast the cooldown.
            let cooldown = Duration::from_millis(config.key_cooldown as u64);
            if data.trigger_pending || data.last_trigger.is_some_and(|t| t.elapsed() < cooldown) {
                return;
            }
            let with_momentum = config.auto_momentum || use_momentum;
            let plan = Self::plan(profile, &bindings, |s| {
                if with_momentum {
                    s.momentum
                } else {
                    0.0
                }
            });
            let profile_id = profile.id.clone();
            data.trigger_pending = true;
            data.last_trigger = Some(Instant::now());
            (profile_id, plan)
        };

        self.emit(SessionEvent::KeyTriggered {
            key_code: plan
                .first()
                .map(|(b, ..)| b.key_code.clone())
                .unwrap_or(key_code),
        });

        let tasks = plan.into_iter().map(|(binding, sound, next, start)| {
            let session = self.clone();
            let profile_id = profile_id.clone();
            async move {
                match session
                    .play_on_track(&binding.track_id, &sound, start)
                    .await
                {
                    Ok(true) => {
                        session.commit_binding_index(&profile_id, &binding, next);
                        let mut data = session.lock();
                        if matches!(binding.loop_mode, LoopMode::Off) {
                            data.loop_contexts.remove(&binding.track_id);
                        } else {
                            data.loop_contexts.insert(
                                binding.track_id.clone(),
                                LoopContext {
                                    key_code: binding.key_code.clone(),
                                    track_id: binding.track_id.clone(),
                                    momentum: MomentumBehavior::ConfigOnly,
                                    persist_single_momentum: matches!(
                                        binding.loop_mode,
                                        LoopMode::Single
                                    ) && start > 0.0,
                                },
                            );
                        }
                        true
                    }
                    Ok(false) => false,
                    Err(error) => {
                        session.report_playback_error(&error);
                        false
                    }
                }
            }
        });
        let played = futures::future::join_all(tasks)
            .await
            .into_iter()
            .any(|ok| ok);

        let mut data = self.lock();
        data.trigger_pending = false;
        if !played {
            // Nothing played: give the key back immediately.
            data.last_trigger = None;
        }
    }

    async fn handle_loop_end(self: Arc<Self>, track_id: String) {
        let auto_momentum = self.config().auto_momentum;
        let planned = {
            let mut data = self.lock();
            let Some(ctx) = data.loop_contexts.get(&track_id).cloned() else {
                return;
            };
            let Some(profile) = data.profile.as_ref() else {
                data.loop_contexts.remove(&track_id);
                return;
            };
            let binding = editor::find_binding(profile, &ctx.key_code, &ctx.track_id).cloned();
            match binding.filter(|b| !matches!(b.loop_mode, LoopMode::Off)) {
                None => {
                    data.loop_contexts.remove(&track_id);
                    return;
                }
                Some(binding) => {
                    let mode = binding.loop_mode.clone();
                    let plan = Self::plan(profile, std::slice::from_ref(&binding), |s| {
                        rules::loop_replay_start(s, &mode, auto_momentum, &ctx)
                    });
                    match plan.into_iter().next() {
                        Some(p) => (profile.id.clone(), ctx, p),
                        None => {
                            data.loop_contexts.remove(&track_id);
                            return;
                        }
                    }
                }
            }
        };
        let (profile_id, ctx, (binding, sound, next, start)) = planned;
        match self.play_on_track(&track_id, &sound, start).await {
            Ok(true) => {
                self.commit_binding_index(&profile_id, &binding, next);
                self.lock().loop_contexts.insert(
                    track_id,
                    LoopContext {
                        persist_single_momentum: matches!(binding.loop_mode, LoopMode::Single)
                            && (ctx.persist_single_momentum || start > 0.0),
                        ..ctx
                    },
                );
            }
            Ok(false) => {}
            Err(error) => {
                self.lock().loop_contexts.remove(&track_id);
                self.report_playback_error(&error);
            }
        }
    }

    async fn handle_mood(self: Arc<Self>, mood: BaseMood, intensity: MoodIntensity) {
        if !self.config().mood_ai_enabled {
            return;
        }
        let auto_momentum = self.config().auto_momentum;
        let (profile_id, previous, plan) = {
            let mut data = self.lock();
            if data.loading || data.active_mood == Some((mood, intensity)) {
                return;
            }
            let Some(profile) = data.profile.as_ref() else {
                return;
            };
            let bindings: Vec<KeyBinding> =
                rules::select_mood_bindings(&profile.key_bindings, mood, intensity)
                    .into_iter()
                    .cloned()
                    .collect();
            let plan = Self::plan(profile, &bindings, |s| {
                if auto_momentum || s.momentum > 0.0 {
                    s.momentum
                } else {
                    0.0
                }
            });
            let profile_id = profile.id.clone();
            let previous = data.active_mood.map(|(m, _)| m);
            data.active_mood = Some((mood, intensity));
            if bindings.is_empty() {
                drop(data);
                self.toast(
                    ToastKind::Info,
                    format!(
                        "Mood: {} {} (no tagged keys)",
                        rules::mood_display(mood),
                        rules::intensity_display(intensity)
                    ),
                );
                return;
            }
            (profile_id, previous, plan)
        };

        let keys: Vec<String> = plan.iter().map(|(b, ..)| b.key_code.clone()).collect();
        let tasks = plan.into_iter().map(|(binding, sound, next, start)| {
            let session = self.clone();
            let profile_id = profile_id.clone();
            async move {
                let ok = matches!(
                    session
                        .play_on_track(&binding.track_id, &sound, start)
                        .await,
                    Ok(true)
                );
                if ok {
                    session.commit_binding_index(&profile_id, &binding, next);
                    let mut data = session.lock();
                    if matches!(binding.loop_mode, LoopMode::Off) {
                        data.loop_contexts.remove(&binding.track_id);
                    } else {
                        data.loop_contexts.insert(
                            binding.track_id.clone(),
                            LoopContext {
                                key_code: binding.key_code.clone(),
                                track_id: binding.track_id.clone(),
                                momentum: MomentumBehavior::PreferSoundMomentum,
                                persist_single_momentum: matches!(
                                    binding.loop_mode,
                                    LoopMode::Single
                                ) && start > 0.0,
                            },
                        );
                    }
                }
                ok
            }
        });
        let played = futures::future::join_all(tasks)
            .await
            .into_iter()
            .any(|ok| ok);

        if played {
            self.lock().committed_mood = Some((mood, intensity));
            self.emit(SessionEvent::MoodPlayback {
                mood,
                intensity,
                previous,
                keys,
            });
        } else {
            let mut data = self.lock();
            if data.active_mood == Some((mood, intensity)) {
                data.active_mood = None;
            }
        }
    }

    /// Trigger a key as if it had been pressed (click on a key, palette, ...).
    pub fn trigger_key(&self, key_code: &str) {
        let session = self.arc();
        let key = key_code.to_string();
        self.runtime
            .spawn(async move { session.handle_key(key, true).await });
    }

    /// Preview a sound outside the profile tracks (one preview at a time).
    pub async fn preview(&self, path: String, start: f64, volume: f32) -> Result<(), String> {
        service::playback::play_sound(
            &self.state,
            PREVIEW_TRACK.to_string(),
            format!("preview:{path}"),
            path,
            start.max(0.0),
            volume.clamp(0.0, 1.0),
            Some(1.0),
        )
        .await
    }

    pub fn stop_preview(&self) {
        if let Ok(engine) = self.state.get_audio_engine() {
            let _ = engine.stop_track(PREVIEW_TRACK.to_string());
        }
    }

    /// Play one sound of the profile on its track from `start` (inspector preview, seek).
    pub async fn play_sound_on_track(
        &self,
        track_id: &str,
        sound_id: &str,
        start: f64,
    ) -> Result<(), String> {
        let sound = self
            .lock()
            .profile
            .as_ref()
            .and_then(|p| p.sounds.iter().find(|s| s.id == sound_id).cloned())
            .ok_or("Sound not found in profile")?;
        match self.play_on_track(track_id, &sound, start).await {
            Ok(_) => Ok(()),
            Err(error) => {
                self.report_playback_error(&error);
                Err(error)
            }
        }
    }

    pub fn stop_track(&self, track_id: &str) {
        self.lock().loop_contexts.remove(track_id);
        if let Ok(engine) = self.state.get_audio_engine() {
            let _ = engine.stop_track(track_id.to_string());
        }
    }

    pub fn stop_all(&self) {
        if let Err(error) = service::playback::stop_all_sounds(&self.state) {
            self.toast(ToastKind::Error, error);
        }
    }

    pub fn now_playing(&self) -> HashMap<String, NowPlaying> {
        self.lock().playing.clone()
    }

    /// Last mood seen by the AI, and the last one that actually started sounds.
    pub fn mood_state(&self) -> MoodState {
        let data = self.lock();
        MoodState {
            detected: data.detected_mood,
            committed: data.committed_mood,
        }
    }

    // ─── Config shortcuts ───────────────────────────────────────────────

    pub fn set_auto_momentum(&self, enabled: bool) {
        self.state.update_config(|c| c.auto_momentum = enabled);
        self.state.schedule_config_save();
        self.emit(SessionEvent::ConfigChanged);
    }

    pub fn set_key_detection(&self, enabled: bool) {
        service::config::set_key_detection(&self.state, enabled);
        self.emit(SessionEvent::ConfigChanged);
    }

    pub fn set_master_volume(&self, volume: f32) {
        service::config::set_master_volume(&self.state, volume.clamp(0.0, 1.0));
        self.emit(SessionEvent::ConfigChanged);
    }

    /// Apply a partial camelCase config update (same contract as the Tauri `update_config`).
    pub async fn update_config(&self, updates: serde_json::Value) -> Result<(), String> {
        let result = service::config::update_config(&self.state, updates).await;
        self.emit(SessionEvent::ConfigChanged);
        result
    }

    // ─── Profile ────────────────────────────────────────────────────────

    pub fn profile(&self) -> Option<Profile> {
        self.lock().profile.clone()
    }

    /// Read the current profile without cloning it.
    pub fn with_profile<R>(&self, f: impl FnOnce(Option<&Profile>) -> R) -> R {
        f(self.lock().profile.as_ref())
    }

    pub fn is_loading(&self) -> bool {
        self.lock().loading
    }

    pub fn save_status(&self) -> SaveStatus {
        self.lock().save_status.unwrap_or(SaveStatus::Saved)
    }

    pub fn list_profiles(&self) -> Vec<ProfileSummary> {
        storage::list_profiles().unwrap_or_else(|error| {
            self.toast(
                ToastKind::Error,
                format!("Could not list profiles: {error}"),
            );
            Vec::new()
        })
    }

    fn sync_detector_bindings(&self, profile: Option<&Profile>) {
        let mut keys: Vec<String> = profile
            .map(|p| p.key_bindings.iter().map(|b| b.key_code.clone()).collect())
            .unwrap_or_default();
        keys.sort();
        keys.dedup();
        self.state.key_detector.set_profile_bindings(&keys);
    }

    /// Edit the current profile. `action` names an undo step (None = not undoable,
    /// e.g. volume drags or computed data). Returns None without a profile.
    pub fn edit<R>(&self, action: Option<&str>, f: impl FnOnce(&mut Profile) -> R) -> Option<R> {
        let result = {
            let mut data = self.lock();
            let data = &mut *data;
            let profile = data.profile.as_mut()?;
            let before = action.map(|_| ProfileSnapshot::capture(profile));
            let result = f(profile);
            if let (Some(action), Some(before)) = (action, before) {
                data.history
                    .push(action, before, ProfileSnapshot::capture(profile));
            }
            self.sync_detector_bindings(Some(profile));
            result
        };
        self.schedule_save();
        self.emit(SessionEvent::ProfileChanged);
        if action.is_some() {
            self.emit(SessionEvent::HistoryChanged);
        }
        Some(result)
    }

    pub fn undo(&self) -> Option<String> {
        let name = {
            let mut data = self.lock();
            let data = &mut *data;
            let profile = data.profile.as_mut()?;
            let name = data.history.undo(profile)?;
            self.sync_detector_bindings(Some(profile));
            name
        };
        self.after_history_move();
        Some(name)
    }

    pub fn redo(&self) -> Option<String> {
        let name = {
            let mut data = self.lock();
            let data = &mut *data;
            let profile = data.profile.as_mut()?;
            let name = data.history.redo(profile)?;
            self.sync_detector_bindings(Some(profile));
            name
        };
        self.after_history_move();
        Some(name)
    }

    fn after_history_move(&self) {
        self.schedule_save();
        self.emit(SessionEvent::ProfileChanged);
        self.emit(SessionEvent::HistoryChanged);
    }

    pub fn undo_name(&self) -> Option<String> {
        self.lock().history.undo_name().map(String::from)
    }

    pub fn redo_name(&self) -> Option<String> {
        self.lock().history.redo_name().map(String::from)
    }

    pub fn history(&self) -> Vec<String> {
        self.lock().history.past_actions()
    }

    /// Update a sound's volume live (engine) and in the profile, without an undo step.
    pub fn set_sound_volume(&self, sound_id: &str, volume: f32) {
        let tracks: Vec<String> = self.with_profile(|p| {
            p.map(|p| p.tracks.iter().map(|t| t.id.clone()).collect())
                .unwrap_or_default()
        });
        self.edit(None, |p| {
            if let Some(sound) = editor::sound_mut(p, sound_id) {
                sound.volume = volume.clamp(0.0, 1.0);
            }
        });
        if let Ok(engine) = self.state.get_audio_engine() {
            for track in tracks {
                let _ = engine.set_sound_volume(track, sound_id.to_string(), volume);
            }
        }
    }

    pub fn set_track_volume(&self, track_id: &str, volume: f32) {
        self.edit(None, |p| editor::set_track_volume(p, track_id, volume));
        if let Ok(engine) = self.state.get_audio_engine() {
            let _ = engine.set_track_volume(track_id.to_string(), volume.clamp(0.0, 1.0));
        }
    }

    fn schedule_save(&self) {
        let gen = {
            let mut data = self.lock();
            if data.profile.is_none() {
                return;
            }
            data.dirty = true;
            data.save_gen += 1;
            data.save_gen
        };
        self.set_save_status(SaveStatus::Pending);
        let session = self.arc();
        self.runtime.spawn(async move {
            tokio::time::sleep(SAVE_DELAY).await;
            if session.lock().save_gen == gen {
                session.flush().await;
            }
        });
    }

    fn set_save_status(&self, status: SaveStatus) {
        let changed = {
            let mut data = self.lock();
            let changed = data.save_status != Some(status);
            data.save_status = Some(status);
            changed
        };
        if changed {
            self.emit(SessionEvent::SaveStatus(status));
        }
    }

    /// Write the latest profile now if it has unsaved changes. Writes are ordered.
    pub async fn flush(&self) -> bool {
        let _ordered = self.save_lock.lock().await;
        let snapshot = {
            let mut data = self.lock();
            if !data.dirty {
                return true;
            }
            data.dirty = false;
            data.profile.clone()
        };
        let Some(mut profile) = snapshot else {
            return true;
        };
        self.set_save_status(SaveStatus::Saving);
        profile.updated_at = chrono::Utc::now().to_rfc3339();
        let result = tokio::task::spawn_blocking(move || storage::save_profile(&profile))
            .await
            .map_err(|e| e.to_string())
            .and_then(|r| r);
        match result {
            Ok(()) => {
                if !self.lock().dirty {
                    self.set_save_status(SaveStatus::Saved);
                }
                true
            }
            Err(error) => {
                self.lock().dirty = true;
                self.set_save_status(SaveStatus::Error);
                self.toast(
                    ToastKind::Error,
                    format!("Changes could not be saved: {error}"),
                );
                false
            }
        }
    }

    /// Flush the profile and config; call before quitting.
    pub fn shutdown(&self) {
        let session = self.arc();
        let _ = self.runtime.block_on(async move { session.flush().await });
        let _ = self.state.flush_config();
    }

    fn reset_runtime_state(&self, data: &mut Data) {
        data.history.clear();
        data.loop_contexts.clear();
        data.active_mood = None;
        data.committed_mood = None;
        data.playing.clear();
    }

    /// Open the profile saved in config, if any (startup).
    pub async fn restore_last_profile(&self) -> Result<(), String> {
        match self.config().current_profile_id {
            Some(id) => self.load_profile(&id).await,
            None => Ok(()),
        }
    }

    /// Switch profiles: save, stop everything, reset history, then prepare the new one.
    pub async fn load_profile(&self, id: &str) -> Result<(), String> {
        let gen = {
            let mut data = self.lock();
            data.load_gen += 1;
            data.loading = true;
            data.load_gen
        };
        if !self.flush().await {
            self.lock().loading = false;
            return Err("Save your changes before switching profiles.".into());
        }
        let id_owned = id.to_string();
        let loaded = tokio::task::spawn_blocking(move || storage::load_profile(id_owned))
            .await
            .map_err(|e| e.to_string())
            .and_then(|r| r);
        let profile = match loaded {
            Ok(profile) => profile,
            Err(error) => {
                let mut data = self.lock();
                if data.load_gen == gen {
                    data.loading = false;
                }
                return Err(error);
            }
        };
        if self.lock().load_gen != gen {
            return Ok(());
        }
        if let Ok(engine) = self.state.get_audio_engine() {
            let _ = engine.stop_all();
        }
        {
            let mut data = self.lock();
            self.reset_runtime_state(&mut data);
            self.sync_detector_bindings(Some(&profile));
            data.profile = Some(profile.clone());
            data.dirty = false;
            data.loading = false;
        }
        self.state
            .update_config(|c| c.current_profile_id = Some(id.to_string()));
        self.state.schedule_config_save();
        self.emit(SessionEvent::ProfileLoaded);
        self.emit(SessionEvent::HistoryChanged);
        self.set_save_status(SaveStatus::Saved);
        self.discovery_open_profile();

        let session = self.arc();
        self.runtime
            .spawn(async move { session.prepare_profile(profile, gen).await });
        Ok(())
    }

    /// Missing files, durations still at 0, then waveform cache warm-up.
    async fn prepare_profile(self: Arc<Self>, profile: Profile, gen: u64) {
        let is_current = |s: &Session| {
            let data = s.lock();
            data.load_gen == gen && data.profile.as_ref().is_some_and(|p| p.id == profile.id)
        };
        let missing = service::library::verify_profile_sounds(&profile);
        if !missing.is_empty() && is_current(&self) {
            self.emit(SessionEvent::MissingSounds(missing));
        }

        let entries: Vec<service::playback::SoundPreloadEntry> = profile
            .sounds
            .iter()
            .filter(|s| s.duration == 0.0)
            .map(|s| service::playback::SoundPreloadEntry {
                sound_id: s.id.clone(),
                file_path: service::library::sound_file_path(s),
                needs_duration: true,
            })
            .collect();
        if !entries.is_empty() {
            if let Ok(durations) =
                service::playback::preload_profile_sounds(&self.state, entries).await
            {
                if !durations.is_empty() && is_current(&self) {
                    self.edit(None, |p| {
                        for sound in &mut p.sounds {
                            if let Some(d) = durations.get(&sound.id) {
                                sound.duration = *d;
                            }
                        }
                    });
                }
            }
        }

        if !is_current(&self) {
            return;
        }
        let waveforms: Vec<service::waveform::WaveformBatchEntry> = self.with_profile(|p| {
            p.map(|p| {
                p.sounds
                    .iter()
                    .filter(|s| s.duration > 0.0)
                    .map(|s| service::waveform::WaveformBatchEntry {
                        path: service::library::sound_file_path(s),
                        num_points: WAVEFORM_POINTS,
                    })
                    .collect()
            })
            .unwrap_or_default()
        });
        let _ = service::waveform::get_waveforms_batch(&self.state, waveforms).await;
    }

    /// Create a profile with a first track and open it.
    pub async fn create_profile(&self, name: &str) -> Result<String, String> {
        let name = name.to_string();
        let mut profile = tokio::task::spawn_blocking(move || storage::create_profile(name))
            .await
            .map_err(|e| e.to_string())??;
        editor::add_track(&mut profile, "Principale")?;
        let to_save = profile.clone();
        tokio::task::spawn_blocking(move || storage::save_profile(&to_save))
            .await
            .map_err(|e| e.to_string())??;
        self.emit(SessionEvent::ProfilesListChanged);
        self.load_profile(&profile.id).await?;
        Ok(profile.id)
    }

    /// First launch: make sure there is a profile with a track to land on.
    pub async fn ensure_profile(&self) -> Result<(), String> {
        if self.lock().profile.is_some() {
            return Ok(());
        }
        match self.list_profiles().first() {
            Some(summary) => self.load_profile(&summary.id.clone()).await,
            None => self.create_profile("Mon profil").await.map(|_| ()),
        }
    }

    pub async fn rename_profile(&self, id: &str, name: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Profile name cannot be empty".into());
        }
        let is_current = self.lock().profile.as_ref().is_some_and(|p| p.id == id);
        if is_current {
            self.edit(None, |p| p.name = name.to_string());
            if !self.flush().await {
                return Err("Could not save the profile".into());
            }
        } else {
            let mut profile = storage::load_profile(id.to_string())?;
            profile.name = name.to_string();
            profile.updated_at = chrono::Utc::now().to_rfc3339();
            storage::save_profile(&profile)?;
        }
        self.emit(SessionEvent::ProfilesListChanged);
        Ok(())
    }

    pub async fn duplicate_profile(
        &self,
        id: &str,
        name: Option<String>,
    ) -> Result<String, String> {
        if !self.flush().await {
            return Err("Could not save the profile".into());
        }
        let profile = storage::duplicate_profile(id.to_string(), name)?;
        self.emit(SessionEvent::ProfilesListChanged);
        Ok(profile.id)
    }

    pub async fn delete_profile(&self, id: &str) -> Result<(), String> {
        let is_current = self.lock().profile.as_ref().is_some_and(|p| p.id == id);
        if is_current {
            let mut data = self.lock();
            data.dirty = false;
        }
        service::library::delete_profile(&self.state, id.to_string())?;
        if is_current {
            if let Ok(engine) = self.state.get_audio_engine() {
                let _ = engine.stop_all();
            }
            {
                let mut data = self.lock();
                data.load_gen += 1;
                self.reset_runtime_state(&mut data);
                data.profile = None;
            }
            self.sync_detector_bindings(None);
            self.state.update_config(|c| c.current_profile_id = None);
            self.state.schedule_config_save();
            self.emit(SessionEvent::ProfileLoaded);
            self.discovery_open_profile();
        }
        self.emit(SessionEvent::ProfilesListChanged);
        Ok(())
    }

    /// Import a `.ktm` and open it.
    pub async fn import_ktm(&self, path: String) -> Result<(), String> {
        let id = service::library::import_profile(&self.state, path).await?;
        self.emit(SessionEvent::ProfilesListChanged);
        self.load_profile(&id).await
    }

    /// Import an old Unity save and open it.
    pub async fn import_legacy(&self, path: String) -> Result<(), String> {
        let profile =
            tokio::task::spawn_blocking(move || service::library::import_legacy_save(path))
                .await
                .map_err(|e| e.to_string())??;
        self.emit(SessionEvent::ProfilesListChanged);
        self.load_profile(&profile.id).await
    }

    pub async fn export_current(&self, output_path: String) -> Result<(), String> {
        let id = self
            .lock()
            .profile
            .as_ref()
            .map(|p| p.id.clone())
            .ok_or("No profile open")?;
        if !self.flush().await {
            return Err("Could not save the profile before exporting".into());
        }
        service::library::export_profile(&self.state, id, output_path).await
    }

    /// Duration of an audio file (for staged sounds).
    pub async fn audio_duration(&self, path: String) -> Result<f64, String> {
        service::playback::get_audio_duration(&self.state, path).await
    }

    /// Download a YouTube video's audio as a profile-ready `Sound` (not added yet).
    pub async fn download_youtube(
        &self,
        url: String,
        download_id: String,
    ) -> Result<Sound, String> {
        service::youtube::add_sound_from_youtube(&self.state, url, download_id).await
    }

    /// Mark a missing local sound as found at `path`.
    pub async fn relocate_sound(&self, sound_id: &str, path: String) -> Result<(), String> {
        let duration = self.audio_duration(path.clone()).await.unwrap_or(0.0);
        self.edit(Some("Locate file"), |p| {
            editor::relocate_sound(p, sound_id, types::SoundSource::Local { path }, duration)
        });
        Ok(())
    }

    /// Re-download a missing YouTube sound in place.
    pub async fn redownload_sound(&self, sound_id: &str) -> Result<(), String> {
        let url = self
            .with_profile(|p| {
                p.and_then(|p| p.sounds.iter().find(|s| s.id == sound_id))
                    .and_then(|s| match &s.source {
                        types::SoundSource::YouTube { url, .. } => Some(url.clone()),
                        types::SoundSource::Local { .. } => None,
                    })
            })
            .ok_or("Only YouTube sounds can be re-downloaded")?;
        let fresh = self
            .download_youtube(url, format!("redownload-{sound_id}"))
            .await?;
        self.edit(Some("Re-download sound"), |p| {
            if let Some(sound) = editor::sound_mut(p, sound_id) {
                sound.source = fresh.source;
                sound.duration = fresh.duration;
            }
        });
        Ok(())
    }
}

/// Waveform resolution used for profile sounds.
pub const WAVEFORM_POINTS: usize = 100;
