//! Discovery carousel for native hosts (port of the web store + predownload hook).
//!
//! The pool (`all`) streams in from the backend; the user browses the first
//! `revealed` items. Items up to `visited` are locked so streaming updates never
//! reorder what the user already saw. Downloads, waveforms and the editable key
//! assignment live per video id, so they survive refreshes of the pool.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use crate::audio::analysis::WaveformData;
use crate::discovery::engine::DiscoverySuggestion;
use crate::service;
use crate::types::{KeyBinding, LoopMode, SoundSource};

use super::{editor, placement, rules, Session, SessionEvent, ToastKind, PREVIEW_TRACK};

const INITIAL_REVEAL: usize = 10;
const REVEAL_INCREMENT: usize = 10;
const BACKGROUND_THRESHOLD: usize = 20;
const MAX_PREDOWNLOADS: usize = 3;
const WAVEFORM_POINTS: usize = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DownloadStatus {
    #[default]
    Idle,
    Downloading,
    Ready,
    Failed,
}

/// Per-suggestion state kept across pool refreshes.
#[derive(Debug, Clone, Default)]
pub struct Enrichment {
    pub status: DownloadStatus,
    pub progress: f64,
    pub cached_path: Option<String>,
    pub duration: Option<f64>,
    pub waveform: Option<WaveformData>,
    pub key: Option<String>,
    pub track_id: Option<String>,
    pub momentum: f64,
    /// The user moved the start point: waveforms no longer set it.
    momentum_chosen: bool,
    /// What the placement guessed: kind, mood, loop mode, and why.
    pub role: Option<placement::SoundRole>,
    pub mood: Option<crate::types::BaseMood>,
    pub loop_mode: Option<LoopMode>,
    pub reason: Option<String>,
    /// Name of the new key (a new group reads as its mood).
    key_name: Option<String>,
    download_id: Option<String>,
}

impl Enrichment {
    /// Start point from a waveform: music at its strong part, the rest
    /// where it becomes audible (unless the user chose one).
    fn take_waveform_start(&mut self, waveform: &WaveformData) {
        if self.momentum_chosen {
            return;
        }
        self.momentum = placement::start_point(
            self.role.unwrap_or(placement::SoundRole::Music),
            &waveform.points,
            waveform.duration,
            waveform.suggested_momentum,
            // A card proposes its strong part (shown on its waveform).
            true,
        );
    }

    fn take_placement(&mut self, placed: placement::Placement) {
        self.key_name = placed.key_name();
        self.key = placed.key;
        self.track_id = placed.track_id;
        self.role = Some(placed.role);
        self.mood = placed.mood;
        self.loop_mode = Some(placed.loop_mode);
        self.reason = Some(placed.reason);
    }
}

fn hint_for(s: &DiscoverySuggestion, track_id: Option<String>) -> placement::SoundHint {
    placement::SoundHint {
        name: s.title.clone(),
        duration: s.duration,
        seed_video_ids: s.source_seed_ids.clone(),
        track_id,
        ..Default::default()
    }
}

/// One visible card.
#[derive(Debug, Clone)]
pub struct DiscoveryCard {
    pub suggestion: DiscoverySuggestion,
    pub info: Enrichment,
    pub previewing: bool,
}

#[derive(Debug, Clone, Default)]
pub struct DiscoverySnapshot {
    pub cards: Vec<DiscoveryCard>,
    pub cursor: usize,
    pub pool_size: usize,
    pub generating: bool,
    pub background_fetching: bool,
    pub resolving_locals: Option<usize>,
    pub progress: Option<(usize, usize, String)>,
    pub preview_volume: f32,
    /// The mood suggestions come from (None: all the profile's sounds).
    pub focus: Option<crate::types::BaseMood>,
    /// The video suggestions come from ("more like this"), by title.
    pub like: Option<String>,
}

#[derive(Default)]
pub(super) struct DiscoveryData {
    profile_id: Option<String>,
    all: Vec<DiscoverySuggestion>,
    revealed: usize,
    cursor: usize,
    visited: i64,
    enrich: HashMap<String, Enrichment>,
    generating: bool,
    background: bool,
    resolving: Option<usize>,
    progress: Option<(usize, usize, String)>,
    previewing: Option<String>,
    preview_volume: Option<f32>,
    gen: u64,
    cursor_save_gen: u64,
    active_downloads: HashSet<String>,
    active_waveforms: HashSet<String>,
    download_seq: u64,
    /// The last refill found nothing new: don't rerun it on every move.
    exhausted: bool,
    /// The saved pool came from other seeds: refill once even if not low.
    stale: bool,
    /// Suggestions only from the sounds of this mood.
    focus: Option<crate::types::BaseMood>,
    /// Suggestions only from this one video ("more like this").
    like: Option<crate::discovery::engine::SeedInfo>,
}

impl DiscoveryData {
    /// A selection narrowed to a mood or a video: not the saved one.
    fn narrowed(&self) -> bool {
        self.focus.is_some() || self.like.is_some()
    }

    fn visible(&self) -> &[DiscoverySuggestion] {
        &self.all[..self.revealed.min(self.all.len())]
    }

    /// Forget work started for an older generation (it will never report back).
    fn reset_in_flight(&mut self) {
        self.background = false;
        self.active_downloads.clear();
        self.active_waveforms.clear();
        for e in self.enrich.values_mut() {
            if e.status == DownloadStatus::Downloading {
                e.status = DownloadStatus::Idle;
                e.download_id = None;
            }
        }
    }

    fn clear(&mut self) {
        let volume = self.preview_volume;
        let gen = self.gen;
        *self = DiscoveryData {
            preview_volume: volume,
            gen,
            ..Default::default()
        };
    }
}

impl Session {
    fn disco(&self) -> std::sync::MutexGuard<'_, DiscoveryData> {
        self.discovery.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn discovery_changed(&self) {
        self.emit(SessionEvent::DiscoveryChanged);
    }

    pub fn discovery_snapshot(&self) -> DiscoverySnapshot {
        let d = self.disco();
        DiscoverySnapshot {
            cards: d
                .visible()
                .iter()
                .map(|s| DiscoveryCard {
                    suggestion: s.clone(),
                    info: d.enrich.get(&s.video_id).cloned().unwrap_or_default(),
                    previewing: d.previewing.as_deref() == Some(s.video_id.as_str()),
                })
                .collect(),
            cursor: d.cursor,
            pool_size: d.all.len(),
            generating: d.generating,
            background_fetching: d.background,
            resolving_locals: d.resolving,
            progress: d.progress.clone(),
            preview_volume: d.preview_volume.unwrap_or(0.5),
            focus: d.focus,
            like: d.like.as_ref().map(|s| s.sound_name.clone()),
        }
    }

    /// Give every visible card without an assignment its guessed key, track,
    /// mood and loop mode (see [`placement`]).
    fn assign_visible(&self, d: &mut DiscoveryData) {
        let assignments: Vec<(String, placement::Placement)> = self.with_profile(|profile| {
            let Some(profile) = profile else {
                return Vec::new();
            };
            let reserved: HashSet<String> = d
                .visible()
                .iter()
                .filter_map(|s| d.enrich.get(&s.video_id).and_then(|e| e.key.clone()))
                .collect();
            let todo: Vec<&DiscoverySuggestion> = d
                .visible()
                .into_iter()
                .filter(|s| d.enrich.get(&s.video_id).is_none_or(|e| e.key.is_none()))
                .collect();
            let hints: Vec<placement::SoundHint> =
                todo.iter().map(|s| hint_for(s, None)).collect();
            // A card needs a track to be added: the first one, if no other fits.
            let first = profile.tracks.first().map(|t| t.id.clone());
            todo.iter()
                .map(|s| s.video_id.clone())
                .zip(
                    placement::place_all_reserving(profile, &hints, &reserved)
                        .into_iter()
                        .map(|mut placed| {
                            if placed.track_id.is_none() {
                                placed.track_id = first.clone();
                            }
                            placed
                        }),
                )
                .collect()
        });
        for (video_id, placed) in assignments {
            let e = d.enrich.entry(video_id).or_default();
            e.take_placement(placed);
            if let Some(waveform) = e.waveform.clone() {
                e.take_waveform_start(&waveform);
            }
        }
    }

    /// Called when a profile is opened: restore the cached carousel or start discovering.
    pub(super) fn discovery_open_profile(&self) {
        let profile = self.with_profile(|p| p.map(|p| (p.id.clone(), !p.sounds.is_empty())));
        service::discovery::cancel_discovery(&self.state);
        let gen = {
            let mut d = self.disco();
            d.gen += 1;
            d.clear();
            d.profile_id = profile.as_ref().map(|(id, _)| id.clone());
            d.gen
        };
        self.stop_preview();
        self.discovery_changed();
        let Some((profile_id, has_sounds)) = profile else {
            return;
        };

        match service::discovery::cached_suggestions(&profile_id) {
            Some(cached) if !cached.suggestions.is_empty() => {
                {
                    let mut d = self.disco();
                    let revealed = if cached.revealed_count > 0 {
                        cached.revealed_count
                    } else {
                        INITIAL_REVEAL
                    };
                    d.revealed = revealed.min(cached.suggestions.len());
                    d.cursor = cached.cursor_index.min(d.revealed.saturating_sub(1));
                    d.visited = cached.visited_index as i64;
                    d.stale = cached.stale;
                    d.all = cached.suggestions;
                    self.assign_visible(&mut d);
                }
                self.discovery_changed();
                self.discovery_after_move();
            }
            _ if has_sounds => self.discovery_generate(gen, Vec::new()),
            _ => {}
        }
    }

    fn discovery_generate(&self, gen: u64, exclude: Vec<String>) {
        let Some(profile_id) = self.disco().profile_id.clone() else {
            return;
        };
        {
            let mut d = self.disco();
            d.generating = true;
            d.progress = None;
        }
        self.discovery_changed();
        let session = self.arc();
        let (focus, like) = {
            let d = self.disco();
            (d.focus, d.like.clone())
        };
        self.runtime.spawn(async move {
            let result = service::discovery::start_discovery_focused(
                &session.state,
                profile_id,
                exclude,
                false,
                focus,
                like,
            )
            .await;
            if session.disco().gen != gen {
                return;
            }
            match result {
                Ok(results) => {
                    session.discovery_merge(results.clone());
                    if results.is_empty() {
                        session.toast(ToastKind::Info, "Pas de nouvelle suggestion");
                    }
                    // Save the carousel's own order (visited items stay locked
                    // in front), so a reload lands on the same card.
                    let (pool, cursor, revealed, visited) = {
                        let d = session.disco();
                        (d.all.clone(), d.cursor, d.revealed, d.visited as i32)
                    };
                    let saved = !session.disco().narrowed();
                    if let (true, Some(profile_id)) = (saved, session.disco().profile_id.clone()) {
                        let _ = crate::discovery::cache::DiscoveryCache::update_pool(
                            &profile_id,
                            pool,
                            cursor,
                            revealed,
                            visited,
                        );
                    }
                }
                Err(error) if error.contains("cancelled") => {}
                Err(error) if error.contains("of this mood") => session.toast(
                    ToastKind::Info,
                    "Aucun son YouTube de cette ambiance pour démarrer",
                ),
                Err(error) => {
                    session.toast(ToastKind::Error, format!("Découverte impossible : {error}"))
                }
            }
            {
                let mut d = session.disco();
                d.generating = false;
                d.progress = None;
                d.resolving = None;
            }
            session.discovery_changed();
            session.discovery_after_move();
        });
    }

    /// Streaming merge: locked (visited) items stay in place, new ones fill in after.
    fn discovery_merge(&self, incoming: Vec<DiscoverySuggestion>) {
        {
            let mut d = self.disco();
            let locked = ((d.visited + 1).max(0) as usize).min(d.all.len());
            let mut all: Vec<DiscoverySuggestion> = d.all[..locked].to_vec();
            let locked_ids: HashSet<String> = all.iter().map(|s| s.video_id.clone()).collect();
            all.extend(
                incoming
                    .into_iter()
                    .filter(|s| !locked_ids.contains(&s.video_id)),
            );
            d.revealed = d.revealed.max(INITIAL_REVEAL).min(all.len());
            d.all = all;
            if d.revealed > 0 {
                d.visited = d.visited.max(d.cursor.min(d.revealed - 1) as i64);
            }
            self.assign_visible(&mut d);
        }
        self.discovery_changed();
    }

    /// Route discovery core events (called from the session's event handler).
    pub(super) fn discovery_on_event(&self, event: &crate::events::CoreEvent) {
        use crate::events::CoreEvent;
        match event {
            CoreEvent::DiscoveryPartial(partial) => {
                if self.disco().generating && !partial.is_empty() {
                    self.discovery_merge(partial.clone());
                }
            }
            CoreEvent::DiscoveryResolving { count } => {
                // Background refills resolve silently; only a visible run shows it.
                let mut d = self.disco();
                if d.generating {
                    d.resolving = Some(*count);
                    drop(d);
                    self.discovery_changed();
                }
            }
            CoreEvent::DiscoveryProgress {
                current,
                total,
                seed_name,
            } => {
                let mut d = self.disco();
                d.resolving = None;
                d.progress = Some((*current, *total, seed_name.clone()));
                drop(d);
                self.discovery_changed();
            }
            CoreEvent::YoutubeDownloadProgress {
                download_id,
                progress: Some(progress),
                ..
            } if download_id.starts_with("predl_") => {
                let mut d = self.disco();
                if let Some(e) = d
                    .enrich
                    .values_mut()
                    .find(|e| e.download_id.as_deref() == Some(download_id.as_str()))
                {
                    e.progress = *progress;
                    drop(d);
                    self.discovery_changed();
                }
            }
            CoreEvent::SoundEnded {
                track_id, sound_id, ..
            } if track_id == PREVIEW_TRACK => {
                let mut d = self.disco();
                if d.previewing.as_deref() == Some(sound_id.as_str()) {
                    d.previewing = None;
                    drop(d);
                    self.discovery_changed();
                }
            }
            _ => {}
        }
    }

    pub fn discovery_go_to(&self, index: usize) {
        {
            let mut d = self.disco();
            if d.revealed == 0 {
                return;
            }
            // Reaching the end reveals the next page.
            if index >= d.revealed && d.revealed < d.all.len() {
                d.revealed = (d.revealed + REVEAL_INCREMENT).min(d.all.len());
                self.assign_visible(&mut d);
            }
            d.cursor = index.min(d.revealed - 1);
            d.visited = d.visited.max(d.cursor as i64);
        }
        self.discovery_changed();
        self.discovery_after_move();
    }

    pub fn discovery_next(&self) {
        let cursor = self.disco().cursor;
        self.discovery_go_to(cursor + 1);
    }

    pub fn discovery_prev(&self) {
        let cursor = self.disco().cursor;
        if cursor > 0 {
            self.discovery_go_to(cursor - 1);
        }
    }

    /// Persist the cursor, pre-download around it, and refill the pool when low.
    fn discovery_after_move(&self) {
        self.discovery_persist_cursor();
        self.discovery_predownload();
        self.discovery_background_fetch();
    }

    fn discovery_persist_cursor(&self) {
        let (profile_id, gen) = {
            let mut d = self.disco();
            // A narrowed selection is not the saved one.
            if d.narrowed() {
                return;
            }
            d.cursor_save_gen += 1;
            (d.profile_id.clone(), d.cursor_save_gen)
        };
        let Some(profile_id) = profile_id else { return };
        let session = self.arc();
        self.runtime.spawn(async move {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let (cursor, revealed, visited, seen) = {
                let d = session.disco();
                if d.cursor_save_gen != gen {
                    return;
                }
                let seen_end = ((d.visited + 1).max(0) as usize).min(d.all.len());
                let seen: Vec<String> = d.all[..seen_end]
                    .iter()
                    .map(|s| s.video_id.clone())
                    .collect();
                (d.cursor, d.revealed, d.visited as i32, seen)
            };
            let _ = crate::discovery::cache::DiscoveryCache::save_cursor(
                &profile_id,
                cursor,
                revealed,
                visited,
            );
            let _ = crate::discovery::cache::DiscoveryCache::mark_seen(&profile_id, &seen);
        });
    }

    fn discovery_background_fetch(&self) {
        let (profile_id, pool_ids, gen, focus, like) = {
            let mut d = self.disco();
            let remaining = d.all.len().saturating_sub(d.revealed);
            if (remaining > BACKGROUND_THRESHOLD && !d.stale)
                || d.exhausted
                || d.background
                || d.generating
                || d.all.is_empty()
            {
                return;
            }
            d.stale = false;
            let Some(profile_id) = d.profile_id.clone() else {
                return;
            };
            d.background = true;
            (
                profile_id,
                d.all.iter().map(|s| s.video_id.clone()).collect::<Vec<_>>(),
                d.gen,
                d.focus,
                d.like.clone(),
            )
        };
        let session = self.arc();
        self.runtime.spawn(async move {
            let result =
                service::discovery::start_discovery_focused(
                    &session.state,
                    profile_id,
                    pool_ids,
                    true,
                    focus,
                    like,
                )
                .await;
            let mut d = session.disco();
            if d.gen != gen {
                return;
            }
            d.background = false;
            if let Ok(results) = result {
                let known: HashSet<String> = d.all.iter().map(|s| s.video_id.clone()).collect();
                let before = d.all.len();
                d.all
                    .extend(results.into_iter().filter(|s| !known.contains(&s.video_id)));
                d.exhausted = d.all.len() == before;
            }
            drop(d);
            session.discovery_changed();
        });
    }

    /// Download [cursor-2, cursor+3] plus the first two unseen items, 3 at a time.
    fn discovery_predownload(&self) {
        let jobs: Vec<(String, String, String, u64)> = {
            let mut d = self.disco();
            let gen = d.gen;
            let mut wanted: Vec<usize> = (d.cursor.saturating_sub(2)..=d.cursor + 3)
                .filter(|&i| i < d.revealed.min(d.all.len()))
                .collect();
            wanted.extend((0..2).map(|i| d.revealed + i).filter(|&i| i < d.all.len()));
            let mut jobs = Vec::new();
            for i in wanted {
                if d.active_downloads.len() >= MAX_PREDOWNLOADS {
                    break;
                }
                let s = d.all[i].clone();
                let idle = d
                    .enrich
                    .get(&s.video_id)
                    .is_none_or(|e| e.status == DownloadStatus::Idle);
                if !idle || d.active_downloads.contains(&s.video_id) {
                    continue;
                }
                d.download_seq += 1;
                let download_id = format!("predl_{}_{}", gen, d.download_seq);
                d.active_downloads.insert(s.video_id.clone());
                let e = d.enrich.entry(s.video_id.clone()).or_default();
                e.status = DownloadStatus::Downloading;
                e.progress = 0.0;
                e.download_id = Some(download_id.clone());
                jobs.push((s.url, s.video_id, download_id, gen));
            }
            jobs
        };
        if !jobs.is_empty() {
            self.discovery_changed();
        }
        for (url, video_id, download_id, gen) in jobs {
            let session = self.arc();
            self.runtime.spawn(async move {
                let result = service::youtube::predownload_suggestion(
                    &session.state,
                    url,
                    video_id.clone(),
                    download_id,
                )
                .await;
                {
                    let mut d = session.disco();
                    if d.gen != gen {
                        return;
                    }
                    d.active_downloads.remove(&video_id);
                    let e = d.enrich.entry(video_id).or_default();
                    match result {
                        Ok(r) => {
                            e.status = DownloadStatus::Ready;
                            e.progress = 100.0;
                            e.cached_path = Some(r.cached_path);
                            e.duration = Some(r.duration);
                            if let Some(waveform) = &r.waveform {
                                e.take_waveform_start(waveform);
                            }
                            e.waveform = r.waveform;
                        }
                        Err(_) => e.status = DownloadStatus::Failed,
                    }
                }
                session.discovery_changed();
                session.discovery_predownload();
                session.discovery_lazy_waveforms();
            });
        }
        self.discovery_lazy_waveforms();
    }

    /// Ready items next to the cursor that still lack a waveform.
    fn discovery_lazy_waveforms(&self) {
        let jobs: Vec<(String, String, u64)> = {
            let mut d = self.disco();
            let gen = d.gen;
            let near: Vec<String> = (d.cursor.saturating_sub(1)..=d.cursor + 1)
                .filter(|&i| i < d.revealed.min(d.all.len()))
                .map(|i| d.all[i].video_id.clone())
                .collect();
            let mut jobs = Vec::new();
            for id in near {
                let Some(e) = d.enrich.get(&id) else { continue };
                if e.status != DownloadStatus::Ready
                    || e.waveform.is_some()
                    || d.active_waveforms.contains(&id)
                {
                    continue;
                }
                if let Some(path) = e.cached_path.clone() {
                    d.active_waveforms.insert(id.clone());
                    jobs.push((id, path, gen));
                }
            }
            jobs
        };
        for (video_id, path, gen) in jobs {
            let session = self.arc();
            self.runtime.spawn(async move {
                let result =
                    service::waveform::get_waveform(&session.state, path, WAVEFORM_POINTS).await;
                let mut d = session.disco();
                if d.gen != gen {
                    return;
                }
                d.active_waveforms.remove(&video_id);
                if let (Ok(waveform), Some(e)) = (result, d.enrich.get_mut(&video_id)) {
                    e.take_waveform_start(&waveform);
                    e.waveform = Some(waveform);
                    drop(d);
                    session.discovery_changed();
                }
            });
        }
    }

    /// Next batch: drop what the user has seen, or discover again when the pool is exhausted.
    pub fn discovery_refresh(&self) {
        let (unseen, profile_id, gen, exclude, seen) = {
            let mut d = self.disco();
            if d.generating {
                return;
            }
            let start = ((d.visited + 1).max(0) as usize).min(d.all.len());
            let unseen: Vec<DiscoverySuggestion> = d.all[start..].to_vec();
            let exclude: Vec<String> = d.all.iter().map(|s| s.video_id.clone()).collect();
            let seen: Vec<String> = exclude[..start].to_vec();
            if unseen.is_empty() {
                d.gen += 1;
                d.reset_in_flight();
                d.exhausted = false;
                d.all.clear();
                d.revealed = 0;
                d.cursor = 0;
                d.visited = -1;
            }
            (unseen, d.profile_id.clone(), d.gen, exclude, seen)
        };
        self.discovery_stop_preview();
        if let Some(profile_id) = &profile_id {
            let _ = crate::discovery::cache::DiscoveryCache::mark_seen(profile_id, &seen);
        }
        if unseen.is_empty() {
            self.discovery_generate(gen, exclude);
            return;
        }
        {
            let mut d = self.disco();
            d.revealed = INITIAL_REVEAL.min(unseen.len());
            d.cursor = 0;
            d.visited = 0;
            d.all = unseen.clone();
            self.assign_visible(&mut d);
        }
        if let Some(profile_id) = profile_id {
            let revealed = self.disco().revealed;
            let _ = crate::discovery::cache::DiscoveryCache::update_pool(
                &profile_id,
                unseen,
                0,
                revealed,
                -1,
            );
        }
        self.discovery_changed();
        self.discovery_after_move();
    }

    /// Suggestions from the sounds of `focus` only (None: all of them): a
    /// new selection starts at once.
    pub fn discovery_set_focus(&self, focus: Option<crate::types::BaseMood>) {
        {
            let d = self.disco();
            if d.focus == focus && d.like.is_none() {
                return;
            }
        }
        // Back to all sounds: the saved selection.
        let Some(mood) = focus else {
            self.discovery_open_profile();
            return;
        };
        // Check before dropping the current selection.
        let startable = self.with_profile(|p| {
            p.is_some_and(|p| service::discovery::has_mood_seeds(p, mood))
        });
        if !startable {
            self.toast(
                ToastKind::Info,
                "Aucun son YouTube de cette ambiance pour démarrer",
            );
            return;
        }
        let (gen, exclude) = {
            let mut d = self.disco();
            if d.profile_id.is_none() {
                return;
            }
            d.focus = focus;
            d.like = None;
            d.gen += 1;
            d.reset_in_flight();
            d.exhausted = false;
            // Seen ones do not come back; the rest of the pool may.
            let start = ((d.visited + 1).max(0) as usize).min(d.all.len());
            let seen: Vec<String> = d.all[..start].iter().map(|s| s.video_id.clone()).collect();
            d.all.clear();
            d.revealed = 0;
            d.cursor = 0;
            d.visited = -1;
            (d.gen, seen)
        };
        self.discovery_stop_preview();
        self.discovery_changed();
        self.discovery_generate(gen, exclude);
    }

    /// Suggestions close to one card ("more like this"): a new selection
    /// from that video alone.
    pub fn discovery_more_like(&self, video_id: &str) {
        let (gen, exclude) = {
            let mut d = self.disco();
            let Some(card) = d.all.iter().find(|s| s.video_id == video_id).cloned() else {
                return;
            };
            if d.profile_id.is_none() {
                return;
            }
            d.like = Some(crate::discovery::engine::SeedInfo {
                video_id: card.video_id.clone(),
                sound_name: card.title.clone(),
            });
            d.focus = None;
            d.gen += 1;
            d.reset_in_flight();
            d.exhausted = false;
            // Seen ones (and the card itself) do not come back.
            let start = ((d.visited + 1).max(0) as usize).min(d.all.len());
            let mut seen: Vec<String> =
                d.all[..start].iter().map(|s| s.video_id.clone()).collect();
            seen.push(card.video_id);
            d.all.clear();
            d.revealed = 0;
            d.cursor = 0;
            d.visited = -1;
            (d.gen, seen)
        };
        self.discovery_stop_preview();
        self.discovery_changed();
        self.discovery_generate(gen, exclude);
    }

    pub fn discovery_cancel(&self) {
        service::discovery::cancel_discovery(&self.state);
    }

    fn discovery_remove(&self, video_id: &str) {
        {
            let mut d = self.disco();
            let Some(index) = d.all.iter().position(|s| s.video_id == video_id) else {
                return;
            };
            d.all.remove(index);
            if index < d.revealed {
                d.revealed -= 1;
                if (index as i64) <= d.visited {
                    d.visited = (d.visited - 1).max(-1);
                }
            }
            d.cursor = d.cursor.min(d.revealed.saturating_sub(1));
            if d.revealed < INITIAL_REVEAL.min(d.all.len()) {
                d.revealed = INITIAL_REVEAL.min(d.all.len());
                self.assign_visible(&mut d);
            }
        }
        self.discovery_changed();
        self.discovery_after_move();
    }

    /// Never suggest this video again (undo from the Library's dislikes list).
    pub fn discovery_dislike(&self, video_id: &str) {
        self.discovery_stop_preview();
        let profile_id = self.disco().profile_id.clone();
        self.discovery_remove(video_id);
        if let Some(profile_id) = profile_id {
            if let Err(error) =
                service::discovery::dislike(&self.state, &profile_id, video_id.to_string())
            {
                self.toast(ToastKind::Error, error);
            }
            // The profile file now carries the dislike; keep the in-memory copy in sync.
            self.edit(None, |p| {
                if !p.disliked_videos.iter().any(|v| v == video_id) {
                    p.disliked_videos.push(video_id.to_string());
                }
            });
        }
    }

    pub fn discovery_set_assignment(
        &self,
        video_id: &str,
        key: Option<String>,
        track_id: Option<String>,
    ) {
        // Another track: the key, mood and loop mode follow it (unless the
        // key is chosen too).
        let replaced = match (&key, &track_id) {
            (None, Some(track)) => {
                let d = self.disco();
                let suggestion = d.all.iter().find(|s| s.video_id == video_id).cloned();
                let reserved: HashSet<String> = d
                    .visible()
                    .iter()
                    .filter(|s| s.video_id != video_id)
                    .filter_map(|s| d.enrich.get(&s.video_id).and_then(|e| e.key.clone()))
                    .collect();
                drop(d);
                suggestion.and_then(|s| {
                    self.with_profile(|p| {
                        p.map(|p| {
                            placement::place_all_reserving(
                                p,
                                &[hint_for(&s, Some(track.clone()))],
                                &reserved,
                            )
                            .remove(0)
                        })
                    })
                })
            }
            _ => None,
        };
        let mut d = self.disco();
        let e = d.enrich.entry(video_id.to_string()).or_default();
        if let Some(placed) = replaced {
            e.take_placement(placed);
            // Its kind may differ on that track: so may its start.
            if let Some(waveform) = e.waveform.clone() {
                e.take_waveform_start(&waveform);
            }
        }
        if key.is_some() {
            e.key = key;
            e.reason = Some("touche choisie".into());
            e.key_name = None;
        }
        if track_id.is_some() {
            e.track_id = track_id;
        }
        drop(d);
        self.discovery_changed();
    }

    pub fn discovery_set_momentum(&self, video_id: &str, momentum: f64) {
        let mut d = self.disco();
        let e = d.enrich.entry(video_id.to_string()).or_default();
        e.momentum = momentum.max(0.0);
        e.momentum_chosen = true;
        drop(d);
        self.discovery_changed();
    }

    pub fn discovery_set_preview_volume(&self, volume: f32) {
        let playing = {
            let mut d = self.disco();
            d.preview_volume = Some(volume.clamp(0.0, 1.0));
            d.previewing.clone()
        };
        if let (Some(id), Ok(engine)) = (playing, self.state.get_audio_engine()) {
            let _ = engine.set_sound_volume(PREVIEW_TRACK.to_string(), id, volume);
        }
    }

    /// Toggle the preview of a ready card, from `start` (defaults to its momentum).
    pub async fn discovery_preview(
        &self,
        video_id: &str,
        start: Option<f64>,
    ) -> Result<(), String> {
        let (path, momentum, volume, already) = {
            let d = self.disco();
            let e = d.enrich.get(video_id).cloned().unwrap_or_default();
            (
                e.cached_path,
                e.momentum,
                d.preview_volume.unwrap_or(0.5),
                d.previewing.as_deref() == Some(video_id),
            )
        };
        if already && start.is_none() {
            self.discovery_stop_preview();
            return Ok(());
        }
        let path = path.ok_or("Still downloading")?;
        self.disco().previewing = Some(video_id.to_string());
        self.discovery_changed();
        let result = service::playback::play_sound(
            &self.state,
            PREVIEW_TRACK.to_string(),
            video_id.to_string(),
            path,
            start.unwrap_or(momentum).max(0.0),
            volume,
            Some(1.0),
        )
        .await;
        if result.is_err() {
            self.disco().previewing = None;
            self.discovery_changed();
        }
        result
    }

    pub fn discovery_stop_preview(&self) {
        let was = self.disco().previewing.take();
        if was.is_some() {
            self.stop_preview();
            self.discovery_changed();
        }
    }

    /// Add a card to the profile on its assigned key and track (downloading first if needed).
    pub async fn discovery_add(&self, video_id: &str) -> Result<(), String> {
        self.discovery_stop_preview();
        let (suggestion, info) = {
            let d = self.disco();
            let s = d
                .all
                .iter()
                .find(|s| s.video_id == video_id)
                .cloned()
                .ok_or("Suggestion no longer available")?;
            (s, d.enrich.get(video_id).cloned().unwrap_or_default())
        };
        let (Some(key), Some(track_id)) = (info.key.clone(), info.track_id.clone()) else {
            return Err("Choose a key and a track first".into());
        };
        let track_exists =
            self.with_profile(|p| p.is_some_and(|p| p.tracks.iter().any(|t| t.id == track_id)));
        if !track_exists {
            return Err("Choose a key and a track first".into());
        }

        let (source, duration, name) = match (info.status, info.cached_path.clone()) {
            (DownloadStatus::Ready, Some(cached_path)) => (
                SoundSource::YouTube {
                    url: suggestion.url.clone(),
                    cached_path,
                },
                info.duration.unwrap_or(suggestion.duration),
                suggestion.title.clone(),
            ),
            _ => {
                self.toast(
                    ToastKind::Info,
                    format!("Téléchargement : {}", suggestion.title),
                );
                let sound = self
                    .download_youtube(suggestion.url.clone(), format!("disc_{video_id}"))
                    .await?;
                (sound.source, sound.duration, sound.name)
            }
        };

        // Even out its level with the profile's sounds.
        let volume = info
            .waveform
            .as_ref()
            .and_then(|w| w.loudness)
            .map_or(1.0, placement::volume_for_loudness);
        let added = self.edit(Some("Add from Discovery"), |p| {
            let id = uuid::Uuid::new_v4().to_string();
            editor::add_sound(
                p,
                crate::types::Sound {
                    id: id.clone(),
                    name: name.clone(),
                    source,
                    momentum: info.momentum,
                    volume,
                    duration,
                    resolved_video_id: None,
                },
            );
            match p
                .key_bindings
                .iter_mut()
                .find(|b| b.key_code == key && b.track_id == track_id)
            {
                Some(binding) => binding.sound_ids.push(id),
                None => {
                    // A new key gets the guessed mood (that of the sounds it
                    // was found from, or of its title) and loop mode.
                    let mood = info
                        .mood
                        .or_else(|| rules::seed_mood(p, &suggestion.source_seed_ids));
                    p.key_bindings.push(KeyBinding {
                        key_code: key.clone(),
                        track_id: track_id.clone(),
                        sound_ids: vec![id],
                        loop_mode: info.loop_mode.clone().unwrap_or(LoopMode::Off),
                        current_index: 0,
                        name: info.key_name.clone(),
                        mood,
                        mood_intensity: None,
                    })
                }
            }
        });
        if added.is_none() {
            return Err("No profile open".into());
        }
        if let Some(profile_id) = self.disco().profile_id.clone() {
            let _ =
                crate::discovery::cache::DiscoveryCache::remove_suggestion(&profile_id, video_id);
        }
        self.discovery_remove(video_id);
        self.toast(ToastKind::Success, format!("Ajouté : {name}"));
        Ok(())
    }
}
