//! Mood AI engine: keeps the chapter being read, runs the model on it in the
//! background (two requests at a time), and tells the session which mood to
//! play when the page being read changes scene.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

use super::analyzer::{self, PageLabel};
use super::chapter::{BatchPlan, Chapter, Next};
use super::provider::ProviderSettings;
use crate::events::{CoreEvent, NoopSink, SharedSink};
use crate::types::{BaseMood, MoodIntensity};

/// Requests in flight at once.
pub const WORKERS: usize = 2;
/// Chapters whose moods are kept on disk (re-reading costs nothing).
const CACHED_CHAPTERS: usize = 200;

/// State of the chapter being read, for the UIs and the extension.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterSnapshot {
    pub key: String,
    pub title: Option<String>,
    pub total_pages: u32,
    pub received: u32,
    pub analyzed: u32,
    pub in_flight: u32,
    /// Page being read (0-based).
    pub focus: u32,
    pub error: Option<String>,
    /// Mood of the page being read, once known.
    pub current: Option<PageMood>,
    /// Every decided page (0-based), for the extension popup.
    pub pages: BTreeMap<u32, PageMood>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct PageMood {
    pub mood: BaseMood,
    pub intensity: u8,
    pub summary: String,
}

impl From<&PageLabel> for PageMood {
    fn from(label: &PageLabel) -> Self {
        Self {
            mood: label.mood,
            intensity: label.intensity.as_u8(),
            summary: label.summary.clone(),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct CachedLabel {
    page: u32,
    mood: BaseMood,
    intensity: u8,
    #[serde(default)]
    summary: String,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct DiskCache {
    /// Most recent last.
    chapters: Vec<(String, Vec<CachedLabel>)>,
}

struct State {
    chapter: Option<Chapter>,
    /// Bumped when the chapter changes: late answers for the old one are dropped.
    generation: u64,
    /// Mood last sent to the session.
    playing: Option<(BaseMood, MoodIntensity)>,
    settings: Option<ProviderSettings>,
    /// The settings were refused (bad key, unknown model): wait for new ones.
    blocked: Option<String>,
    cache: Option<DiskCache>,
}

pub struct MoodEngine {
    state: Mutex<State>,
    wake: Notify,
    events: Mutex<SharedSink>,
    cancel: Mutex<Option<CancellationToken>>,
    cache_path: Option<PathBuf>,
}

impl Default for MoodEngine {
    fn default() -> Self {
        Self::new(None)
    }
}

enum Step {
    Run(BatchPlan, u64, ProviderSettings),
    Wait(Duration),
    Idle,
}

impl MoodEngine {
    /// `cache_path`: where decided moods are kept between runs (None: memory only).
    pub fn new(cache_path: Option<PathBuf>) -> Self {
        Self {
            state: Mutex::new(State {
                chapter: None,
                generation: 0,
                playing: None,
                settings: None,
                blocked: None,
                cache: None,
            }),
            wake: Notify::new(),
            events: Mutex::new(Arc::new(NoopSink)),
            cancel: Mutex::new(None),
            cache_path,
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn emit(&self, event: CoreEvent) {
        let sink = self
            .events
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        sink.emit(event);
    }

    pub fn is_running(&self) -> bool {
        self.cancel
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .is_some_and(|c| !c.is_cancelled())
    }

    /// Start the workers on the current tokio runtime (restarts them if running).
    pub fn start(self: &Arc<Self>, sink: SharedSink) {
        *self.events.lock().unwrap_or_else(|e| e.into_inner()) = sink;
        let token = CancellationToken::new();
        if let Some(old) = self
            .cancel
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .replace(token.clone())
        {
            old.cancel();
        }
        for _ in 0..WORKERS {
            let engine = self.clone();
            let token = token.clone();
            tokio::spawn(async move { engine.worker(token).await });
        }
    }

    /// Stop the workers and forget the chapter and the playing mood.
    pub fn stop(&self) {
        if let Some(token) = self.cancel.lock().unwrap_or_else(|e| e.into_inner()).take() {
            token.cancel();
        }
        let mut state = self.lock();
        state.chapter = None;
        state.generation += 1;
        state.playing = None;
    }

    /// New provider settings: retry what failed with the old ones.
    pub fn configure(&self, settings: ProviderSettings) {
        {
            let mut state = self.lock();
            if state.settings.as_ref() == Some(&settings) {
                return;
            }
            state.settings = Some(settings);
            state.blocked = None;
            if let Some(chapter) = state.chapter.as_mut() {
                chapter.clear_backoff();
            }
        }
        self.wake.notify_waiters();
        self.emit_progress();
    }

    /// The reader opened (or re-announced) a chapter. Returns the pages
    /// already received, so the extension only sends the missing ones.
    pub fn open_chapter(
        &self,
        key: &str,
        title: Option<String>,
        total_pages: Option<u32>,
    ) -> Vec<u32> {
        let received = {
            let mut state = self.lock();
            self.ensure_chapter(&mut state, key);
            let chapter = state.chapter.as_mut().expect("chapter just ensured");
            if title.as_deref().is_some_and(|t| !t.trim().is_empty()) {
                chapter.title = title;
            }
            if let Some(total) = total_pages.filter(|t| *t > 0) {
                chapter.total_pages = Some(total);
            }
            chapter.pages.keys().copied().collect()
        };
        self.wake.notify_waiters();
        self.emit_progress();
        received
    }

    /// Store pages (base64 JPEG) of the chapter being read.
    pub fn add_pages(&self, key: &str, pages: Vec<(u32, Arc<String>)>) -> u32 {
        let received = {
            let mut state = self.lock();
            match state.chapter.as_ref() {
                // Late pages from a chapter the reader already left.
                Some(chapter) if chapter.key != key => return 0,
                Some(_) => {}
                None => self.ensure_chapter(&mut state, key),
            }
            let chapter = state.chapter.as_mut().expect("chapter checked");
            let now = Instant::now();
            for (page, image) in pages {
                chapter.add_page(page, image, now);
            }
            chapter.pages.len() as u32
        };
        self.wake.notify_waiters();
        self.emit_progress();
        received
    }

    /// The reader is on `page`: analyze around it first, and play its mood.
    pub fn set_position(&self, key: &str, page: u32, total_pages: Option<u32>) -> ChapterSnapshot {
        let (snapshot, events) = {
            let mut state = self.lock();
            self.ensure_chapter(&mut state, key);
            let chapter = state.chapter.as_mut().expect("chapter just ensured");
            chapter.focus = page;
            if let Some(total) = total_pages.filter(|t| *t > 0) {
                chapter.total_pages = Some(total);
            }
            let events = Self::play_focus(&mut state);
            (Self::snapshot_of(&state), events)
        };
        self.wake.notify_waiters();
        for event in events {
            self.emit(event);
        }
        snapshot.expect("chapter just ensured")
    }

    pub fn snapshot(&self) -> Option<ChapterSnapshot> {
        Self::snapshot_of(&self.lock())
    }

    /// Settings the analysis uses (None before the first `configure`).
    pub fn settings(&self) -> Option<ProviderSettings> {
        self.lock().settings.clone()
    }

    /// Error that stops the analysis (settings refused), if any.
    pub fn blocking_error(&self) -> Option<String> {
        self.lock().blocked.clone()
    }

    fn snapshot_of(state: &State) -> Option<ChapterSnapshot> {
        let chapter = state.chapter.as_ref()?;
        Some(ChapterSnapshot {
            key: chapter.key.clone(),
            title: chapter.title.clone(),
            total_pages: chapter.known_pages(),
            received: chapter.pages.len() as u32,
            analyzed: chapter.labels.len() as u32,
            in_flight: chapter.in_flight.len() as u32,
            focus: chapter.focus,
            error: state.blocked.clone().or_else(|| chapter.last_error.clone()),
            current: chapter.labels.get(&chapter.focus).map(PageMood::from),
            pages: chapter
                .labels
                .iter()
                .map(|(page, label)| (*page, PageMood::from(label)))
                .collect(),
        })
    }

    fn emit_progress(&self) {
        let event = {
            let state = self.lock();
            let Some(snapshot) = Self::snapshot_of(&state) else {
                return;
            };
            CoreEvent::MoodProgress {
                analyzed: snapshot.analyzed,
                total: snapshot.total_pages,
                in_flight: snapshot.in_flight,
                error: snapshot.error,
            }
        };
        self.emit(event);
    }

    fn ensure_chapter(&self, state: &mut State, key: &str) {
        if state.chapter.as_ref().is_some_and(|c| c.key == key) {
            return;
        }
        let mut chapter = Chapter::new(key.to_string());
        chapter.labels = self.cached_labels(state, key);
        state.chapter = Some(chapter);
        state.generation += 1;
    }

    /// Events making the session play the mood of the page being read, if it
    /// is known and differs from what plays.
    fn play_focus(state: &mut State) -> Vec<CoreEvent> {
        let Some(label) = state
            .chapter
            .as_ref()
            .and_then(|c| c.labels.get(&c.focus))
            .cloned()
        else {
            return Vec::new();
        };
        let wanted = (label.mood, label.intensity);
        if state.playing == Some(wanted) {
            return Vec::new();
        }
        let previous = state.playing.replace(wanted);
        vec![
            CoreEvent::MoodDetected {
                mood: label.mood.as_str().to_string(),
                intensity: label.intensity.as_u8(),
                source: "api".into(),
            },
            CoreEvent::MoodCommitted {
                mood: label.mood.as_str().to_string(),
                intensity: label.intensity.as_u8(),
                source: "api".into(),
                previous_mood: previous.map(|(m, _)| m.as_str().to_string()),
                summary: label.summary.clone(),
            },
        ]
    }

    fn next_step(&self) -> Step {
        let mut state = self.lock();
        let Some(settings) = state.settings.clone() else {
            return Step::Idle;
        };
        if state.blocked.is_some() || !settings.is_configured() {
            return Step::Idle;
        }
        let generation = state.generation;
        let Some(chapter) = state.chapter.as_mut() else {
            return Step::Idle;
        };
        match chapter.next_batch(Instant::now()) {
            Next::Run(plan) => {
                chapter.start(&plan);
                Step::Run(plan, generation, settings)
            }
            Next::Wait(delay) => Step::Wait(delay),
            Next::Idle => Step::Idle,
        }
    }

    async fn worker(self: Arc<Self>, cancel: CancellationToken) {
        loop {
            let notified = self.wake.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();

            match self.next_step() {
                Step::Run(plan, generation, settings) => {
                    self.emit_progress();
                    let started = Instant::now();
                    let result = tokio::select! {
                        result = analyzer::analyze(&settings, &plan.pages, &plan.context) => result,
                        _ = cancel.cancelled() => return,
                    };
                    tracing::info!(
                        "Mood AI: pages {}-{} in {:.1}s: {}",
                        plan.pages.first().map_or(0, |p| p.0 + 1),
                        plan.pages.last().map_or(0, |p| p.0 + 1),
                        started.elapsed().as_secs_f32(),
                        match &result {
                            Ok(_) => "ok".to_string(),
                            Err(e) => e.message.clone(),
                        }
                    );
                    let events = self.apply(plan, generation, result);
                    for event in events {
                        self.emit(event);
                    }
                    self.emit_progress();
                    self.wake.notify_waiters();
                }
                Step::Wait(delay) => {
                    tokio::select! {
                        _ = &mut notified => {}
                        _ = tokio::time::sleep(delay) => {}
                        _ = cancel.cancelled() => return,
                    }
                }
                Step::Idle => {
                    tokio::select! {
                        _ = &mut notified => {}
                        // Safety net for a missed wake-up.
                        _ = tokio::time::sleep(Duration::from_secs(5)) => {}
                        _ = cancel.cancelled() => return,
                    }
                }
            }
        }
    }

    fn apply(
        &self,
        plan: BatchPlan,
        generation: u64,
        result: Result<BTreeMap<u32, PageLabel>, super::provider::ProviderError>,
    ) -> Vec<CoreEvent> {
        let mut state = self.lock();
        if state.generation != generation {
            return Vec::new();
        }
        let Some(chapter) = state.chapter.as_mut() else {
            return Vec::new();
        };
        match result {
            Ok(labels) => {
                chapter.finish(&plan, labels);
                let key = chapter.key.clone();
                let labels = chapter.labels.clone();
                self.remember(&mut state, &key, &labels);
                Self::play_focus(&mut state)
            }
            Err(error) => {
                chapter.fail(&plan, &error.message, Instant::now());
                if error.settings {
                    state.blocked = Some(error.message.clone());
                }
                Vec::new()
            }
        }
    }

    // ── Disk cache ──

    fn load_cache(&self, state: &mut State) {
        if state.cache.is_some() {
            return;
        }
        let cache = self
            .cache_path
            .as_ref()
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice::<DiskCache>(&bytes).ok())
            .unwrap_or_default();
        state.cache = Some(cache);
    }

    fn cached_labels(&self, state: &mut State, key: &str) -> BTreeMap<u32, PageLabel> {
        self.load_cache(state);
        state
            .cache
            .as_ref()
            .and_then(|cache| cache.chapters.iter().find(|(k, _)| k == key))
            .map(|(_, labels)| {
                labels
                    .iter()
                    .map(|l| {
                        (
                            l.page,
                            PageLabel {
                                mood: l.mood,
                                intensity: MoodIntensity::from_u8(l.intensity),
                                summary: l.summary.clone(),
                            },
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn remember(&self, state: &mut State, key: &str, labels: &BTreeMap<u32, PageLabel>) {
        self.load_cache(state);
        let Some(cache) = state.cache.as_mut() else {
            return;
        };
        cache.chapters.retain(|(k, _)| k != key);
        cache.chapters.push((
            key.to_string(),
            labels
                .iter()
                .map(|(page, l)| CachedLabel {
                    page: *page,
                    mood: l.mood,
                    intensity: l.intensity.as_u8(),
                    summary: l.summary.clone(),
                })
                .collect(),
        ));
        let excess = cache.chapters.len().saturating_sub(CACHED_CHAPTERS);
        cache.chapters.drain(..excess);
        if let Some(path) = &self.cache_path {
            if let Ok(bytes) = serde_json::to_vec(cache) {
                let tmp = path.with_extension("json.tmp");
                if std::fs::write(&tmp, bytes).is_ok() {
                    let _ = std::fs::rename(&tmp, path);
                }
            }
        }
    }

    /// Forget every decided mood (memory and disk).
    pub fn clear_cache(&self) {
        let mut state = self.lock();
        state.cache = Some(DiskCache::default());
        if let Some(chapter) = state.chapter.as_mut() {
            chapter.labels.clear();
        }
        state.playing = None;
        if let Some(path) = &self.cache_path {
            let _ = std::fs::remove_file(path);
        }
        drop(state);
        self.wake.notify_waiters();
        self.emit_progress();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label(mood: BaseMood) -> PageLabel {
        PageLabel {
            mood,
            intensity: MoodIntensity::Medium,
            summary: "s".into(),
        }
    }

    #[test]
    fn the_mood_plays_once_per_scene() {
        let engine = MoodEngine::new(None);
        engine.set_position("c", 0, Some(10));
        {
            let mut state = engine.lock();
            let chapter = state.chapter.as_mut().unwrap();
            for page in 0..5 {
                chapter.labels.insert(page, label(BaseMood::Tension));
            }
            for page in 5..10 {
                chapter.labels.insert(page, label(BaseMood::Comedy));
            }
        }
        let play = |page: u32| {
            let mut state = engine.lock();
            state.chapter.as_mut().unwrap().focus = page;
            MoodEngine::play_focus(&mut state)
        };
        assert_eq!(play(0).len(), 2);
        assert!(play(3).is_empty(), "same scene: nothing new to play");
        let events = play(6);
        assert!(matches!(
            &events[1],
            CoreEvent::MoodCommitted { mood, previous_mood: Some(prev), .. }
                if mood == "comedy" && prev == "tension"
        ));
    }

    #[test]
    fn decided_moods_survive_a_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mood_cache.json");
        let engine = MoodEngine::new(Some(path.clone()));
        engine.open_chapter("https://reader/ch1", None, Some(3));
        {
            let mut state = engine.lock();
            let labels: BTreeMap<u32, PageLabel> =
                (0..3).map(|p| (p, label(BaseMood::Mystery))).collect();
            engine.remember(&mut state, "https://reader/ch1", &labels);
        }
        let again = MoodEngine::new(Some(path));
        again.open_chapter("https://reader/ch1", None, None);
        let snapshot = again.snapshot().unwrap();
        assert_eq!(snapshot.analyzed, 3);
        assert_eq!(snapshot.pages[&2].mood, BaseMood::Mystery);
    }

    #[test]
    fn late_pages_of_a_left_chapter_are_ignored() {
        let engine = MoodEngine::new(None);
        engine.open_chapter("ch2", None, None);
        assert_eq!(engine.add_pages("ch1", vec![(0, Arc::new("x".into()))]), 0);
        assert_eq!(engine.add_pages("ch2", vec![(0, Arc::new("x".into()))]), 1);
    }
}
