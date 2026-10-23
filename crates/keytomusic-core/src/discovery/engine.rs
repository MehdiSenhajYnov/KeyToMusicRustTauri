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

/// A video found in one or more seed Mixes.
struct Candidate {
    title: String,
    channel: String,
    duration: f64,
    url: String,
    seed_names: Vec<String>,
    seed_ids: Vec<String>,
    /// Sum over seeds of a weight that decays with the position in the Mix:
    /// YouTube puts the closest matches first.
    score: f64,
}

/// Suggestions kept per run.
const MAX_SUGGESTIONS: usize = 30;
/// Items backed by a single seed that one seed may contribute, so one Mix
/// can't fill the whole list.
const MAX_SINGLE_SEED_ITEMS: usize = 6;

/// Same song under another upload ("Official Video", "Lyrics", "- Topic"…).
fn title_key(title: &str) -> String {
    static NOISE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(
            r"(?i)[\[(][^\])]*[\])]|\b(official|music|video|audio|lyrics?|lyric|hd|hq|4k|visualizer|extended|version|full)\b|- topic",
        )
        .expect("invalid title noise regex")
    });
    NOISE
        .replace_all(title, " ")
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Rank candidates: seeds that agree first, then Mix position; ties broken by
/// id so the order is stable between runs and partial updates.
fn build_suggestions(
    candidates: &HashMap<String, Candidate>,
    excluded: &HashSet<String>,
) -> Vec<DiscoverySuggestion> {
    let mut ranked: Vec<(&String, &Candidate)> = candidates
        .iter()
        .filter(|(video_id, c)| {
            !excluded.contains(*video_id) && c.duration >= 30.0 && c.duration <= 900.0
            // 15 minutes
        })
        .collect();
    ranked.sort_by(|(a_id, a), (b_id, b)| {
        b.seed_ids
            .len()
            .cmp(&a.seed_ids.len())
            .then(b.score.total_cmp(&a.score))
            .then(a_id.cmp(b_id))
    });

    let mut titles = HashSet::new();
    let mut per_seed: HashMap<&str, usize> = HashMap::new();
    let mut suggestions = Vec::new();
    for (video_id, c) in ranked {
        if suggestions.len() >= MAX_SUGGESTIONS {
            break;
        }
        let key = title_key(&c.title);
        if !key.is_empty() && !titles.insert(key) {
            continue;
        }
        if let [only] = c.seed_ids.as_slice() {
            let count = per_seed.entry(only.as_str()).or_default();
            if *count >= MAX_SINGLE_SEED_ITEMS {
                continue;
            }
            *count += 1;
        }
        suggestions.push(DiscoverySuggestion {
            video_id: video_id.clone(),
            title: c.title.clone(),
            channel: c.channel.clone(),
            duration: c.duration,
            url: c.url.clone(),
            occurrence_count: c.seed_ids.len(),
            source_seed_names: c.seed_names.clone(),
            source_seed_ids: c.seed_ids.clone(),
        });
    }
    suggestions
}

pub struct DiscoveryEngine {
    pub cancel_flag: Arc<AtomicBool>,
}

/// Resolve when the flag is raised (checked every 100 ms).
pub(crate) async fn cancelled(flag: &AtomicBool) {
    while !flag.load(Ordering::Relaxed) {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

impl DiscoveryEngine {
    pub fn new(cancel_flag: Arc<AtomicBool>) -> Self {
        Self { cancel_flag }
    }

    /// Generate discovery suggestions from seed videos.
    /// `seeds`: YouTube videos already in the profile (at most 15 are used).
    /// `existing_ids`: video IDs never to suggest (profile, dismissed, disliked).
    /// `seen_ids`: suggestions shown before; skipped unless nothing else is left.
    /// `progress_callback`: called after each seed is processed (current, total, seed_name).
    /// `partial_callback`: called after each seed with the current partial suggestions.
    pub async fn generate_suggestions(
        &self,
        seeds: Vec<SeedInfo>,
        existing_ids: Vec<String>,
        seen_ids: Vec<String>,
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
        let mut excluded = existing_set.clone();
        excluded.extend(seen_ids);

        let mut candidates: HashMap<String, Candidate> = HashMap::new();

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

        loop {
            let next = tokio::select! {
                next = stream.next() => next,
                _ = cancelled(&self.cancel_flag) => None,
            };
            let Some((seed, mix)) = next else { break };
            if self.cancel_flag.load(Ordering::Relaxed) {
                break;
            }

            for (position, result) in mix.into_iter().enumerate() {
                let entry = candidates
                    .entry(result.video_id.clone())
                    .or_insert_with(|| Candidate {
                        title: result.title.clone(),
                        channel: result.channel.clone(),
                        duration: result.duration,
                        url: result.url.clone(),
                        seed_names: Vec::new(),
                        seed_ids: Vec::new(),
                        score: 0.0,
                    });
                if entry.seed_ids.contains(&seed.video_id) {
                    continue;
                }
                entry.seed_ids.push(seed.video_id.clone());
                if !entry.seed_names.contains(&seed.sound_name) {
                    entry.seed_names.push(seed.sound_name.clone());
                }
                entry.score += 1.0 / (position as f64 + 3.0);
            }

            // Emit partial suggestions after each seed
            let partial = build_suggestions(&candidates, &excluded);
            partial_callback(&partial);
        }

        let suggestions = build_suggestions(&candidates, &excluded);
        if suggestions.is_empty() && excluded.len() > existing_set.len() {
            // Everything new was already shown once: show it again rather than nothing.
            return build_suggestions(&candidates, &existing_set);
        }
        suggestions
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
/// Local sounds YouTube had nothing for, skipped for the rest of the run of
/// the app instead of searching them again on every refill.
static UNRESOLVABLE: std::sync::LazyLock<Mutex<HashSet<String>>> =
    std::sync::LazyLock::new(Mutex::default);

pub async fn resolve_local_seeds(
    locals: Vec<(String, String, String)>, // (sound_id, file_path, sound_name)
    youtube_cache: Arc<Mutex<YouTubeCache>>,
) -> Vec<ResolvedLocal> {
    let locals: Vec<_> = {
        let skip = UNRESOLVABLE.lock().unwrap_or_else(|e| e.into_inner());
        locals
            .into_iter()
            .filter(|(sound_id, _, _)| !skip.contains(sound_id))
            .collect()
    };
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
                    UNRESOLVABLE
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .insert(sound_id);
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
                        UNRESOLVABLE
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .insert(sound_id);
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

#[cfg(test)]
mod ranking_tests {
    use super::*;

    fn candidate(title: &str, seeds: &[&str], score: f64) -> Candidate {
        Candidate {
            title: title.to_string(),
            channel: String::new(),
            duration: 200.0,
            url: String::new(),
            seed_names: seeds.iter().map(|s| s.to_string()).collect(),
            seed_ids: seeds.iter().map(|s| s.to_string()).collect(),
            score,
        }
    }

    #[test]
    fn agreement_beats_position_and_order_is_stable() {
        let mut map = HashMap::new();
        map.insert("a".to_string(), candidate("Alpha", &["s1"], 0.30));
        map.insert("b".to_string(), candidate("Beta", &["s1", "s2"], 0.05));
        map.insert("c".to_string(), candidate("Gamma", &["s2"], 0.30));
        let ids: Vec<String> = build_suggestions(&map, &HashSet::new())
            .into_iter()
            .map(|s| s.video_id)
            .collect();
        assert_eq!(ids, ["b", "a", "c"]);
    }

    #[test]
    fn one_seed_cannot_fill_the_list() {
        let mut map = HashMap::new();
        for i in 0..20 {
            map.insert(
                format!("x{i:02}"),
                candidate(&format!("Song {i}"), &["s1"], 1.0),
            );
        }
        map.insert("y".to_string(), candidate("Other", &["s2"], 0.01));
        let out = build_suggestions(&map, &HashSet::new());
        assert_eq!(out.len(), MAX_SINGLE_SEED_ITEMS + 1);
        assert!(out.iter().any(|s| s.video_id == "y"));
    }

    #[test]
    fn same_song_from_another_upload_is_merged() {
        let mut map = HashMap::new();
        map.insert(
            "a".to_string(),
            candidate("Unravel (Official Video)", &["s1", "s2"], 1.0),
        );
        map.insert("b".to_string(), candidate("Unravel [Lyrics]", &["s1"], 0.5));
        map.insert("c".to_string(), candidate("Unravel - Topic", &["s3"], 0.5));
        assert_eq!(build_suggestions(&map, &HashSet::new()).len(), 1);
    }

    #[test]
    fn excluded_and_out_of_range_are_skipped() {
        let mut map = HashMap::new();
        map.insert("a".to_string(), candidate("Owned", &["s1"], 1.0));
        let mut short = candidate("Jingle", &["s1"], 1.0);
        short.duration = 10.0;
        map.insert("b".to_string(), short);
        let excluded: HashSet<String> = ["a".to_string()].into();
        assert!(build_suggestions(&map, &excluded).is_empty());
    }
}
