use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use futures::stream::{self, StreamExt};
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::mix_fetcher;
use crate::audio::analysis;
use crate::youtube::{cache::YouTubeCache, search};

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
            !existing_set.contains(*video_id) && *duration >= 30.0 && *duration <= 900.0
            // 15 minutes
        })
        .map(
            |(
                video_id,
                (title, channel, duration, url, source_seed_names, source_seed_ids, count),
            )| {
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

// Pre-compiled regexes for filename cleaning (compiled once, reused across calls)
static NOISE_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?i)\b(copy|final|v\d+|edit|\d{2,3}kbps|track\s?\d+)\b")
        .expect("invalid noise regex")
});
static BRACKET_RE: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r"[\[\(](.*?)[\]\)]").expect("invalid bracket regex"));
static USEFUL_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"(?i)\b(OST|Soundtrack|Original)\b").expect("invalid useful regex")
});
static LEADING_NUM_RE: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r"^\d+\s+").expect("invalid leading num regex"));
static SPACES_RE: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r"\s{2,}").expect("invalid spaces regex"));

/// Clean a filename into a search query by removing extensions, noise, and formatting.
pub fn clean_filename_for_search(filename: &str) -> String {
    // Remove extension
    let name = std::path::Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(filename);

    // Replace separators with spaces
    let result = name.replace(['_', '-', '.'], " ");

    // Remove parasitic patterns (case-insensitive)
    let result = NOISE_RE.replace_all(&result, " ");

    // Remove brackets/parentheses content EXCEPT useful keywords
    let mut preserved = Vec::new();
    for cap in BRACKET_RE.captures_iter(&result) {
        if let Some(inner) = cap.get(1) {
            let inner_str = inner.as_str();
            if USEFUL_RE.is_match(inner_str) {
                for m in USEFUL_RE.find_iter(inner_str) {
                    preserved.push(m.as_str().to_string());
                }
            }
        }
    }
    let result = BRACKET_RE.replace_all(&result, " ");
    let mut result = result.to_string();
    for kw in &preserved {
        result.push(' ');
        result.push_str(kw);
    }

    // Remove leading isolated numbers (track numbers)
    let result = LEADING_NUM_RE.replace(&result, "");

    // Collapse multiple spaces
    let result = SPACES_RE.replace_all(&result, " ");

    result.trim().to_string()
}

/// Result of resolving a local sound to a YouTube video ID.
pub struct ResolvedLocal {
    pub sound_id: String,
    pub video_id: String,
}

/// Resolve local sounds to YouTube video IDs by searching their metadata/filenames.
/// Returns resolved pairs (sound_id, video_id). Skips sounds with unusable queries.
pub async fn resolve_local_seeds(
    locals: Vec<(String, String, String)>, // (sound_id, file_path, sound_name)
    youtube_cache: Arc<Mutex<YouTubeCache>>,
) -> Vec<ResolvedLocal> {
    if locals.is_empty() {
        return Vec::new();
    }

    let results: Vec<Option<ResolvedLocal>> = stream::iter(locals.into_iter())
        .map(|(sound_id, file_path, sound_name)| {
            let cache = youtube_cache.clone();
            async move {
                // Build query via cascade: tags > title > filename
                let query = build_search_query(&file_path, &sound_name);

                if query.len() < 3 {
                    tracing::debug!("Skipping local sound '{}': query too short", sound_name);
                    return None;
                }

                tracing::info!(
                    "Resolving local sound '{}' with query: '{}'",
                    sound_name,
                    query
                );

                match search::search_youtube(&query, 1, cache).await {
                    Ok(results) if !results.is_empty() => {
                        let video_id = results[0].video_id.clone();
                        tracing::info!("Resolved '{}' → {}", sound_name, video_id);
                        Some(ResolvedLocal { sound_id, video_id })
                    }
                    Ok(_) => {
                        tracing::debug!("No YouTube results for '{}'", sound_name);
                        None
                    }
                    Err(e) => {
                        tracing::warn!("YouTube search failed for '{}': {}", sound_name, e);
                        None
                    }
                }
            }
        })
        .buffer_unordered(5) // Max 5 concurrent to avoid spamming yt-dlp
        .collect()
        .await;

    results.into_iter().flatten().collect()
}

/// Build the best search query for a local sound using the cascade:
/// 1. Tags (title + artist) — best quality
/// 2. Tag title alone
/// 3. Cleaned filename — fallback
fn build_search_query(file_path: &str, sound_name: &str) -> String {
    if let Some(tags) = analysis::read_audio_metadata_tags(file_path) {
        if let (Some(ref title), Some(ref artist)) = (&tags.title, &tags.artist) {
            return format!("{} {}", title, artist);
        }
        if let Some(ref title) = tags.title {
            return title.clone();
        }
    }

    // Fallback: clean filename
    clean_filename_for_search(sound_name)
}
