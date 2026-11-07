//! The chapter being read: where its pages come from, the pages received,
//! the moods decided so far, and what to download or analyze next.
//!
//! Pages come two ways: the extension lists their URLs and the app downloads
//! them (most readers), or the extension uploads the images (pages the app
//! could not download: `blob:` images, hosts refusing it).

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use super::analyzer::{BatchContext, PageLabel, BATCH_PAGES, CONTEXT_PAGES};
use super::image::PageImage;

/// While pages keep arriving, wait this long for a fuller batch.
pub const SETTLE: Duration = Duration::from_millis(1500);
const MAX_BACKOFF: Duration = Duration::from_secs(60);
/// A failed download is tried this many times before the extension is asked.
const DOWNLOAD_ATTEMPTS: u32 = 2;
const DOWNLOAD_RETRY: Duration = Duration::from_secs(3);

/// What became of a page download.
#[derive(Debug, Clone, Default)]
struct Download {
    attempts: u32,
    retry_at: Option<Instant>,
    error: Option<String>,
}

pub struct Chapter {
    /// Stable key from the extension (the reader URL).
    pub key: String,
    pub title: Option<String>,
    pub total_pages: Option<u32>,
    /// Page images (0-based).
    pub pages: BTreeMap<u32, PageImage>,
    /// URL of each page, when the extension listed them.
    pub sources: BTreeMap<u32, String>,
    /// Referer for the downloads (the reader page).
    pub referer: Option<String>,
    pub downloading: BTreeSet<u32>,
    downloads: BTreeMap<u32, Download>,
    pub labels: BTreeMap<u32, PageLabel>,
    pub in_flight: BTreeSet<u32>,
    /// Page being read.
    pub focus: u32,
    pub last_page_at: Option<Instant>,
    failures: u32,
    retry_at: Option<Instant>,
    pub last_error: Option<String>,
}

/// Pages to send in one request.
#[derive(Debug, Clone)]
pub struct BatchPlan {
    pub pages: Vec<(u32, PageImage)>,
    pub context: BatchContext,
}

/// What a worker should do now.
#[derive(Debug)]
pub enum Next {
    Run(BatchPlan),
    /// Nothing ready yet; check again after this delay (or when woken).
    Wait(Duration),
    /// Everything received is analyzed or in flight.
    Idle,
}

impl Chapter {
    pub fn new(key: String) -> Self {
        Self {
            key,
            title: None,
            total_pages: None,
            pages: BTreeMap::new(),
            sources: BTreeMap::new(),
            referer: None,
            downloading: BTreeSet::new(),
            downloads: BTreeMap::new(),
            labels: BTreeMap::new(),
            in_flight: BTreeSet::new(),
            focus: 0,
            last_page_at: None,
            failures: 0,
            retry_at: None,
            last_error: None,
        }
    }

    pub fn add_page(&mut self, page: u32, image: PageImage, now: Instant) {
        if let Some(total) = self.total_pages {
            if page >= total {
                self.total_pages = Some(page + 1);
            }
        }
        self.pages.insert(page, image);
        self.downloading.remove(&page);
        self.downloads.remove(&page);
        self.last_page_at = Some(now);
    }

    /// The page URLs the reader lists (in reading order). A new list for the
    /// same chapter replaces the old one; pages already received stay.
    pub fn set_sources(&mut self, urls: Vec<String>, referer: Option<String>) {
        if urls.is_empty() {
            return;
        }
        let total = urls.len() as u32;
        let changed: Vec<u32> = urls
            .iter()
            .enumerate()
            .filter(|(i, url)| self.sources.get(&(*i as u32)) != Some(*url))
            .map(|(i, _)| i as u32)
            .collect();
        for page in &changed {
            self.downloads.remove(page);
        }
        self.sources = urls
            .into_iter()
            .enumerate()
            .map(|(i, url)| (i as u32, url))
            .collect();
        self.referer = referer.or(self.referer.take());
        self.total_pages = Some(self.total_pages.map_or(total, |t| t.max(total)));
    }

    /// Next page to download: the one being read first, then ahead, then behind.
    pub fn next_download(&mut self, now: Instant) -> Option<(u32, String, Option<String>)> {
        let page = self.priority_order().into_iter().find(|p| {
            self.sources
                .get(p)
                .is_some_and(|url| super::fetch::downloadable(url))
                && !self.pages.contains_key(p)
                && !self.downloading.contains(p)
                && self.downloads.get(p).is_none_or(|d| {
                    d.attempts < DOWNLOAD_ATTEMPTS && d.retry_at.is_none_or(|at| at <= now)
                })
        })?;
        self.downloading.insert(page);
        Some((page, self.sources[&page].clone(), self.referer.clone()))
    }

    /// When the next retry is due, if a download waits for one.
    pub fn next_download_retry(&self, now: Instant) -> Option<Duration> {
        self.downloads
            .iter()
            .filter(|(p, d)| d.attempts < DOWNLOAD_ATTEMPTS && !self.pages.contains_key(p))
            .filter_map(|(_, d)| d.retry_at)
            .map(|at| at.saturating_duration_since(now))
            .min()
    }

    pub fn download_failed(&mut self, page: u32, error: String, now: Instant) {
        self.downloading.remove(&page);
        let entry = self.downloads.entry(page).or_default();
        entry.attempts += 1;
        entry.retry_at = Some(now + DOWNLOAD_RETRY);
        entry.error = Some(error);
    }

    /// Pages the app cannot get by itself: the extension should upload them.
    pub fn wanted_uploads(&self) -> Vec<u32> {
        let listed = self.known_pages();
        (0..listed)
            .filter(|p| !self.pages.contains_key(p) && !self.downloading.contains(p))
            .filter(|p| match self.sources.get(p) {
                None => true,
                Some(url) if !super::fetch::downloadable(url) => true,
                Some(_) => self
                    .downloads
                    .get(p)
                    .is_some_and(|d| d.attempts >= DOWNLOAD_ATTEMPTS),
            })
            .collect()
    }

    /// Why the last download of some page failed, if every attempt did.
    pub fn download_error(&self) -> Option<String> {
        self.downloads
            .values()
            .filter(|d| d.attempts >= DOWNLOAD_ATTEMPTS)
            .find_map(|d| d.error.clone())
    }

    fn open(&self, page: u32) -> bool {
        !self.labels.contains_key(&page) && !self.in_flight.contains(&page)
    }

    fn last_known_page(&self) -> Option<u32> {
        match self.total_pages {
            Some(total) => total.checked_sub(1),
            None => self.pages.keys().next_back().copied(),
        }
    }

    /// Pages in the order they matter: the one being read, then ahead, then behind.
    fn priority_order(&self) -> Vec<u32> {
        let Some(last) = self.last_known_page() else {
            return Vec::new();
        };
        let focus = self.focus.min(last);
        let mut order: Vec<u32> = (focus..=last).collect();
        order.extend((0..focus).rev());
        order
    }

    /// Choose the next batch, or say how long to wait.
    pub fn next_batch(&self, now: Instant) -> Next {
        if let Some(retry_at) = self.retry_at {
            if retry_at > now {
                return Next::Wait(retry_at - now);
            }
        }
        let Some(start) = self
            .priority_order()
            .into_iter()
            .find(|&p| self.open(p) && self.pages.contains_key(&p))
        else {
            return Next::Idle;
        };

        // A run of consecutive open pages with images, forward from `start`…
        let size = BATCH_PAGES;
        let mut run = vec![start];
        let mut next = start + 1;
        while run.len() < size && self.open(next) && self.pages.contains_key(&next) {
            run.push(next);
            next += 1;
        }
        // …stopping where the extension has not delivered yet: worth a short wait.
        let gap_ahead = self.open(next)
            && !self.pages.contains_key(&next)
            && self.last_known_page().is_some_and(|last| next <= last);
        let end_unknown = self.total_pages.is_none() && self.pages.keys().next_back() == run.last();
        let more_expected = run.len() < size && (gap_ahead || end_unknown);
        if more_expected {
            if let Some(at) = self.last_page_at {
                let quiet = now.saturating_duration_since(at);
                if quiet < SETTLE {
                    return Next::Wait(SETTLE - quiet);
                }
            }
        }
        // …and completed backward when that leaves room.
        let mut previous = start;
        while run.len() < size && previous > 0 {
            previous -= 1;
            if !(self.open(previous) && self.pages.contains_key(&previous)) {
                break;
            }
            run.insert(0, previous);
        }

        let first = run[0];
        let last = *run.last().unwrap_or(&first);
        let before = (first.saturating_sub(CONTEXT_PAGES as u32)..first)
            .filter_map(|p| {
                self.pages
                    .get(&p)
                    .map(|image| (p, image.clone(), self.labels.get(&p).cloned()))
            })
            .collect();
        let after = self
            .labels
            .get(&(last + 1))
            .map(|label| (last + 1, label.clone()));
        Next::Run(BatchPlan {
            pages: run.iter().map(|p| (*p, self.pages[p].clone())).collect(),
            context: BatchContext {
                before,
                after,
                total_pages: self.total_pages,
                title: self.title.clone(),
            },
        })
    }

    pub fn start(&mut self, plan: &BatchPlan) {
        self.in_flight.extend(plan.pages.iter().map(|(p, _)| *p));
    }

    pub fn finish(&mut self, plan: &BatchPlan, labels: BTreeMap<u32, PageLabel>) {
        for (page, _) in &plan.pages {
            self.in_flight.remove(page);
        }
        self.labels.extend(labels);
        self.failures = 0;
        self.retry_at = None;
        self.last_error = None;
    }

    /// Give the pages back and back off before the next attempt.
    pub fn fail(&mut self, plan: &BatchPlan, error: &str, now: Instant) {
        for (page, _) in &plan.pages {
            self.in_flight.remove(page);
        }
        self.failures += 1;
        let delay = Duration::from_secs(2u64.saturating_pow(self.failures.min(6)));
        self.retry_at = Some(now + delay.min(MAX_BACKOFF));
        self.last_error = Some(error.to_string());
    }

    /// Hold every request until `until` (usage limit reached).
    pub fn pause_until(&mut self, until: Instant) {
        self.retry_at = Some(self.retry_at.map_or(until, |at| at.max(until)));
    }

    /// Forget a past failure (new settings): try again right away.
    pub fn clear_backoff(&mut self) {
        self.failures = 0;
        self.retry_at = None;
        self.last_error = None;
    }

    pub fn known_pages(&self) -> u32 {
        self.total_pages
            .unwrap_or(0)
            .max(self.pages.keys().next_back().map_or(0, |p| p + 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{BaseMood, MoodIntensity};

    fn urls(n: u32) -> Vec<String> {
        (0..n).map(|i| format!("https://cdn/p{i}.jpg")).collect()
    }

    #[test]
    fn downloads_start_at_the_page_being_read_and_failures_go_to_the_extension() {
        let now = Instant::now();
        let mut c = Chapter::new("c".into());
        c.set_sources(urls(5), Some("https://reader/ch".into()));
        assert_eq!(c.total_pages, Some(5));
        c.focus = 3;
        let (page, url, referer) = c.next_download(now).unwrap();
        assert_eq!(
            (page, url.as_str(), referer.as_deref()),
            (3, "https://cdn/p3.jpg", Some("https://reader/ch"))
        );
        assert_eq!(c.next_download(now).unwrap().0, 4);
        assert_eq!(c.next_download(now).unwrap().0, 2);
        c.add_page(3, image(), now);
        c.download_failed(4, "HTTP 403".into(), now);
        // Retried once after a pause…
        assert!(c.next_download(now).is_some_and(|(p, ..)| p != 4));
        assert!(c.wanted_uploads().is_empty());
        let later = now + DOWNLOAD_RETRY;
        let (again, ..) = (0..3)
            .find_map(|_| c.next_download(later).filter(|(p, ..)| *p == 4))
            .unwrap();
        assert_eq!(again, 4);
        c.download_failed(4, "HTTP 403".into(), later);
        // …then asked of the extension.
        assert_eq!(c.wanted_uploads(), vec![4]);
        assert_eq!(c.download_error().as_deref(), Some("HTTP 403"));
        c.add_page(4, image(), later);
        assert!(!c.wanted_uploads().contains(&4));
    }

    #[test]
    fn local_page_urls_are_uploaded_by_the_extension() {
        let mut c = Chapter::new("c".into());
        c.set_sources(
            vec!["blob:https://reader/1".into(), "https://cdn/2.jpg".into()],
            None,
        );
        assert_eq!(c.next_download(Instant::now()).unwrap().0, 1);
        assert_eq!(c.wanted_uploads(), vec![0]);
    }

    fn image() -> PageImage {
        crate::mood::image::page(vec!["x".into()])
    }

    fn chapter(pages: impl IntoIterator<Item = u32>, total: Option<u32>, now: Instant) -> Chapter {
        let mut chapter = Chapter::new("c".into());
        chapter.total_pages = total;
        for page in pages {
            chapter.add_page(page, image(), now - SETTLE * 2);
        }
        chapter
    }

    fn label() -> PageLabel {
        PageLabel {
            mood: BaseMood::Tension,
            intensity: MoodIntensity::Medium,
            summary: String::new(),
        }
    }

    fn run(next: Next) -> BatchPlan {
        match next {
            Next::Run(plan) => plan,
            other => panic!("expected a batch, got {other:?}"),
        }
    }

    fn numbers(plan: &BatchPlan) -> Vec<u32> {
        plan.pages.iter().map(|(p, _)| *p).collect()
    }

    #[test]
    fn batches_start_at_the_page_being_read() {
        let now = Instant::now();
        let mut c = chapter(0..40, Some(40), now);
        c.focus = 12;
        let plan = run(c.next_batch(now));
        assert_eq!(numbers(&plan), (12..22).collect::<Vec<_>>());
        assert_eq!(
            plan.context
                .before
                .iter()
                .map(|(p, ..)| *p)
                .collect::<Vec<_>>(),
            vec![10, 11]
        );
        c.start(&plan);
        let second = run(c.next_batch(now));
        assert_eq!(numbers(&second), (22..32).collect::<Vec<_>>());
    }

    #[test]
    fn behind_pages_come_after_the_ones_ahead() {
        let now = Instant::now();
        let mut c = chapter(0..14, Some(14), now);
        c.focus = 8;
        let plan = run(c.next_batch(now));
        // 8..14 is only six pages: completed backward to ten.
        assert_eq!(numbers(&plan), (4..14).collect::<Vec<_>>());
        c.start(&plan);
        assert_eq!(numbers(&run(c.next_batch(now))), (0..4).collect::<Vec<_>>());
    }

    #[test]
    fn waits_briefly_for_pages_still_arriving() {
        let now = Instant::now();
        let mut c = Chapter::new("c".into());
        c.total_pages = Some(30);
        for page in 0..3 {
            c.add_page(page, image(), now);
        }
        assert!(matches!(c.next_batch(now), Next::Wait(_)));
        // Once the stream goes quiet, a short batch is better than nothing.
        assert_eq!(numbers(&run(c.next_batch(now + SETTLE))), vec![0, 1, 2]);
    }

    #[test]
    fn decided_pages_are_skipped_and_used_as_context() {
        let now = Instant::now();
        let mut c = chapter(0..20, Some(20), now);
        for page in 0..10 {
            c.labels.insert(page, label());
        }
        let plan = run(c.next_batch(now));
        assert_eq!(numbers(&plan), (10..20).collect::<Vec<_>>());
        assert!(plan.context.before.iter().all(|(_, _, l)| l.is_some()));
        c.start(&plan);
        assert!(matches!(c.next_batch(now), Next::Idle));
    }

    #[test]
    fn failures_back_off() {
        let now = Instant::now();
        let mut c = chapter(0..5, Some(5), now);
        let plan = run(c.next_batch(now));
        c.start(&plan);
        c.fail(&plan, "boom", now);
        assert!(matches!(c.next_batch(now), Next::Wait(_)));
        assert_eq!(c.last_error.as_deref(), Some("boom"));
        c.clear_backoff();
        assert!(matches!(c.next_batch(now), Next::Run(_)));
    }
}
