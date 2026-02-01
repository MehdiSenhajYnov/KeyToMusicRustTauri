use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};

use super::mix_fetcher;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedInfo {
    pub video_id: String,
    pub sound_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverySuggestion {
    pub video_id: String,
    pub title: String,
    pub channel: String,
    pub duration: f64,
    pub url: String,
    pub occurrence_count: usize,
    pub source_seed_names: Vec<String>,
    #[serde(default)]
    pub source_seed_ids: Vec<String>,
}

// video_id -> (title, channel, duration, url, source_seed_names, source_seed_ids, count)
type OccurrenceMap = HashMap<
    String,
    (
        String,      // title
        String,      // channel
        f64,         // duration
        String,      // url
        Vec<String>, // source_seed_names
        Vec<String>, // source_seed_ids
        usize,       // count
    ),
>;

/// Build sorted, filtered suggestions from the occurrence map.
/// Uses `.iter()` + clone so the map survives between iterations.
fn build_suggestions(
    occurrence_map: &OccurrenceMap,
    existing_set: &HashSet<String>,
) -> Vec<DiscoverySuggestion> {
    let mut suggestions: Vec<DiscoverySuggestion> = occurrence_map
        .iter()
        .filter(|(video_id, (_, _, duration, _, _, _, _))| {
            !existing_set.contains(*video_id)
                && *duration >= 30.0
                && *duration <= 900.0 // 15 minutes
        })
        .map(
            |(video_id, (title, channel, duration, url, source_seed_names, source_seed_ids, count))| {
                DiscoverySuggestion {
                    video_id: video_id.clone(),
                    title: title.clone(),
                    channel: channel.clone(),
                    duration: *duration,
                    url: url.clone(),
                    occurrence_count: *count,
                    source_seed_names: source_seed_names.clone(),
                    source_seed_ids: source_seed_ids.clone(),
                }
            },
        )
        .collect();

    // Sort by occurrence count descending
    suggestions.sort_by(|a, b| b.occurrence_count.cmp(&a.occurrence_count));

    // Return top 30
    suggestions.truncate(30);
    suggestions
}

pub struct DiscoveryEngine {
    pub cancel_flag: Arc<AtomicBool>,
}

impl DiscoveryEngine {
    pub fn new(cancel_flag: Arc<AtomicBool>) -> Self {
        Self { cancel_flag }
    }

    /// Generate discovery suggestions from seed videos.
    /// `seeds`: YouTube videos already in the profile.
    /// `existing_ids`: video IDs already in the profile (to filter out).
    /// `progress_callback`: called after each seed is processed (current, total, seed_name).
    /// `partial_callback`: called after each seed with the current partial suggestions.
    pub async fn generate_suggestions(
        &self,
        seeds: Vec<SeedInfo>,
        existing_ids: Vec<String>,
        yt_dlp_bin: PathBuf,
        progress_callback: impl Fn(usize, usize, &str) + Send + Sync,
        partial_callback: impl Fn(&[DiscoverySuggestion]) + Send + Sync,
    ) -> Vec<DiscoverySuggestion> {
        let seeds = if seeds.len() > 15 {
            seeds[..15].to_vec()
        } else {
            seeds
        };

        let total = seeds.len();
        let existing_set: HashSet<String> = existing_ids.into_iter().collect();

        let mut occurrence_map: OccurrenceMap = HashMap::new();

        let completed = Arc::new(AtomicUsize::new(0));
        let cancel = self.cancel_flag.clone();
        let progress_callback = Arc::new(progress_callback);

        let mut stream = stream::iter(seeds.into_iter())
            .map(|seed| {
                let bin = yt_dlp_bin.clone();
                let cancel = cancel.clone();
                let count = completed.clone();
                let cb = progress_callback.clone();
                async move {
                    if cancel.load(Ordering::Relaxed) {
                        return (seed, vec![]);
                    }
                    let mix = mix_fetcher::fetch_mix(&seed.video_id, &bin).await;
                    let done = count.fetch_add(1, Ordering::Relaxed) + 1;
                    cb(done, total, &seed.sound_name);
                    (seed, mix)
                }
            })
            .buffer_unordered(10);

        while let Some((seed, mix)) = stream.next().await {
            if self.cancel_flag.load(Ordering::Relaxed) {
                break;
            }

            for result in mix {
                let entry = occurrence_map
                    .entry(result.video_id.clone())
                    .or_insert_with(|| {
                        (
                            result.title.clone(),
                            result.channel.clone(),
                            result.duration,
                            result.url.clone(),
                            Vec::new(),
                            Vec::new(),
                            0,
                        )
                    });
                if !entry.4.contains(&seed.sound_name) {
                    entry.4.push(seed.sound_name.clone());
                }
                if !entry.5.contains(&seed.video_id) {
                    entry.5.push(seed.video_id.clone());
                }
                entry.6 += 1;
            }

            // Emit partial suggestions after each seed
            let partial = build_suggestions(&occurrence_map, &existing_set);
            partial_callback(&partial);
        }

        build_suggestions(&occurrence_map, &existing_set)
    }
}
