//! The chapter being read: its pages as the extension sends them, the moods
//! decided so far, and which pages to send to the model next.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::analyzer::{BatchContext, PageLabel, BATCH_PAGES, CONTEXT_PAGES};

/// While pages keep arriving, wait this long for a fuller batch.
pub const SETTLE: Duration = Duration::from_millis(1500);
const MAX_BACKOFF: Duration = Duration::from_secs(60);

pub struct Chapter {
    /// Stable key from the extension (the reader URL).
    pub key: String,
    pub title: Option<String>,
    pub total_pages: Option<u32>,
    /// Base64 JPEG per page (0-based).
    pub pages: BTreeMap<u32, Arc<String>>,
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
    pub pages: Vec<(u32, Arc<String>)>,
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
            labels: BTreeMap::new(),
            in_flight: BTreeSet::new(),
            focus: 0,
            last_page_at: None,
            failures: 0,
            retry_at: None,
            last_error: None,
        }
    }

    pub fn add_page(&mut self, page: u32, image: Arc<String>, now: Instant) {
        if let Some(total) = self.total_pages {
            if page >= total {
                self.total_pages = Some(page + 1);
            }
        }
        self.pages.insert(page, image);
        self.last_page_at = Some(now);
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
        let mut run = vec![start];
        let mut next = start + 1;
        while run.len() < BATCH_PAGES && self.open(next) && self.pages.contains_key(&next) {
            run.push(next);
            next += 1;
        }
        // …stopping where the extension has not delivered yet: worth a short wait.
        let gap_ahead = self.open(next)
            && !self.pages.contains_key(&next)
            && self.last_known_page().is_some_and(|last| next <= last);
        let end_unknown = self.total_pages.is_none() && self.pages.keys().next_back() == run.last();
        let more_expected = run.len() < BATCH_PAGES && (gap_ahead || end_unknown);
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
        while run.len() < BATCH_PAGES && previous > 0 {
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

    fn image() -> Arc<String> {
        Arc::new("x".into())
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
