use std::collections::HashMap;
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
    pub async fn generate_suggestions(
        &self,
        seeds: Vec<SeedInfo>,
        existing_ids: Vec<String>,
        yt_dlp_bin: PathBuf,
        progress_callback: impl Fn(usize, usize, &str) + Send + Sync,
    ) -> Vec<DiscoverySuggestion> {
        let seeds = if seeds.len() > 15 {
            seeds[..15].to_vec()
        } else {
            seeds
        };

        let total = seeds.len();

        // video_id -> (info, set of source_seed_names, count)
        let mut occurrence_map: HashMap<
            String,
            (
                String,  // title
                String,  // channel
                f64,     // duration
                String,  // url
                Vec<String>, // source_seed_names
                usize,   // count
            ),
        > = HashMap::new();

        let completed = Arc::new(AtomicUsize::new(0));
        let cancel = self.cancel_flag.clone();
        let progress_callback = Arc::new(progress_callback);

        let results: Vec<_> = stream::iter(seeds.into_iter())
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
            .buffer_unordered(4)
            .collect()
            .await;

        for (seed, mix) in results {
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
                            0,
                        )
                    });
                if !entry.4.contains(&seed.sound_name) {
                    entry.4.push(seed.sound_name.clone());
                }
                entry.5 += 1;
            }
        }

        // Filter and sort
        let existing_set: std::collections::HashSet<String> =
            existing_ids.into_iter().collect();

        let mut suggestions: Vec<DiscoverySuggestion> = occurrence_map
            .into_iter()
            .filter(|(video_id, (_, _, duration, _, _, _))| {
                !existing_set.contains(video_id)
                    && *duration >= 30.0
                    && *duration <= 900.0 // 15 minutes
            })
            .map(
                |(video_id, (title, channel, duration, url, source_seed_names, count))| {
                    DiscoverySuggestion {
                        video_id,
                        title,
                        channel,
                        duration,
                        url,
                        occurrence_count: count,
                        source_seed_names,
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
}
