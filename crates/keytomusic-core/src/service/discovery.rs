use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::discovery::{self, engine::DiscoverySuggestion};
use crate::events::CoreEvent;
use crate::state::AppState;
use crate::storage;
use crate::types::{Profile, SoundSource};
use crate::youtube;

/// Seeds a run actually fetches Mixes for.
const MAX_SEEDS: usize = 15;

/// One seed per distinct video: the profile's YouTube sounds and the local
/// sounds already matched to a video. Also the ids to never suggest.
fn profile_seeds(profile: &Profile) -> Vec<discovery::engine::SeedInfo> {
    let mut seeds: Vec<discovery::engine::SeedInfo> = Vec::new();
    for sound in &profile.sounds {
        let video_id = match &sound.source {
            SoundSource::YouTube { url, .. } => youtube::downloader::extract_video_id(url),
            SoundSource::Local { .. } => sound.resolved_video_id.clone(),
        };
        if let Some(video_id) = video_id {
            if !seeds.iter().any(|s| s.video_id == video_id) {
                seeds.push(discovery::engine::SeedInfo {
                    video_id,
                    sound_name: sound.name.clone(),
                });
            }
        }
    }
    seeds
}

/// Hash of the profile's seeds; a cache built from other seeds is stale.
pub fn profile_seed_hash(profile: &Profile) -> String {
    let ids: Vec<String> = profile_seeds(profile)
        .into_iter()
        .map(|s| s.video_id)
        .collect();
    discovery::cache::DiscoveryCache::compute_seed_hash(&ids)
}

/// With more seeds than a run can fetch, keep the newest sounds (what the
/// user is into right now) and a random pick of the rest, so every sound
/// eventually feeds the suggestions.
fn pick_seeds(mut seeds: Vec<discovery::engine::SeedInfo>) -> Vec<discovery::engine::SeedInfo> {
    if seeds.len() <= MAX_SEEDS {
        return seeds;
    }
    let newest = seeds.split_off(seeds.len() - MAX_SEEDS / 2);
    // Fisher–Yates with a per-run random state; no need for a crypto RNG.
    let mut state = {
        use std::hash::{BuildHasher, Hasher};
        let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
        hasher.write_usize(seeds.len());
        hasher.finish() | 1
    };
    for i in (1..seeds.len()).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        seeds.swap(i, (state % (i as u64 + 1)) as usize);
    }
    seeds.truncate(MAX_SEEDS - newest.len());
    seeds.extend(newest);
    seeds
}

/// Generate suggestions from the profile's YouTube and (resolved) local sounds.
/// Foreground runs stream `Discovery*` events and replace the cache; background
/// runs are silent and append to it. Starting a run cancels the previous one.
pub async fn start_discovery(
    state: &AppState,
    profile_id: String,
    exclude_ids: Vec<String>,
    background: bool,
) -> Result<Vec<DiscoverySuggestion>, String> {
    let cancel_flag = {
        let mut active = state
            .discovery_cancel
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        active.store(true, Ordering::Relaxed);
        let next = Arc::new(AtomicBool::new(false));
        *active = next.clone();
        next
    };

    let profile = storage::load_profile(profile_id.clone())?;

    let mut seeds = profile_seeds(&profile);
    let mut existing_ids: Vec<String> = seeds.iter().map(|s| s.video_id.clone()).collect();
    let unresolved_locals: Vec<(String, String, String)> = profile
        .sounds
        .iter()
        .filter(|s| s.resolved_video_id.is_none())
        .filter_map(|s| match &s.source {
            SoundSource::Local { path } => Some((s.id.clone(), path.clone(), s.name.clone())),
            SoundSource::YouTube { .. } => None,
        })
        .collect(); // (sound_id, path, name)

    // Resolve unresolved local sounds via YouTube search
    if !unresolved_locals.is_empty() {
        let count = unresolved_locals.len();
        if !background {
            state.emit(CoreEvent::DiscoveryResolving { count });
        }
        tracing::info!("Resolving {} local sounds for discovery seeds", count);

        let youtube_cache = state.youtube_cache.clone();
        let resolved = tokio::select! {
            resolved = discovery::engine::resolve_local_seeds(unresolved_locals, youtube_cache) => resolved,
            _ = discovery::engine::cancelled(&cancel_flag) => return Err("Discovery cancelled".into()),
        };

        let mut resolved_updates: Vec<(String, String)> = Vec::new();
        for r in &resolved {
            if let Some(sound) = profile.sounds.iter().find(|s| s.id == r.sound_id) {
                if !seeds.iter().any(|s| s.video_id == r.video_id) {
                    seeds.push(discovery::engine::SeedInfo {
                        video_id: r.video_id.clone(),
                        sound_name: sound.name.clone(),
                    });
                }
                existing_ids.push(r.video_id.clone());
                resolved_updates.push((r.sound_id.clone(), r.video_id.clone()));
            }
        }

        // Targeted update avoids overwriting concurrent profile changes
        if !resolved_updates.is_empty() {
            if let Err(e) = storage::update_resolved_video_ids(&profile_id, &resolved_updates) {
                tracing::warn!("Failed to save resolved video IDs: {}", e);
            }
        }
    }

    if seeds.is_empty() {
        return Err(
            "No seeds found in profile (no YouTube sounds and no resolvable local sounds)"
                .to_string(),
        );
    }

    let seed_ids: Vec<String> = seeds.iter().map(|s| s.video_id.clone()).collect();
    let seed_hash = discovery::cache::DiscoveryCache::compute_seed_hash(&seed_ids);

    // Dismissed and already-seen suggestions from previous runs don't come back.
    let previous = discovery::cache::DiscoveryCache::load(&profile_id);
    let mut all_excluded = existing_ids;
    if let Some(previous) = &previous {
        all_excluded.extend(previous.dismissed_ids.iter().cloned());
    }
    all_excluded.extend(profile.disliked_videos.iter().cloned());
    all_excluded.extend(exclude_ids);
    let seen_ids = previous.map(|d| d.seen_ids).unwrap_or_default();

    let yt_dlp_bin = match youtube::downloader::get_yt_dlp_bin() {
        Ok(path) => path,
        Err(_) => youtube::download_yt_dlp().await?,
    };
    if cancel_flag.load(Ordering::Relaxed) {
        return Err("Discovery cancelled".into());
    }

    if !background {
        state.emit(CoreEvent::DiscoveryStarted);
    }

    let engine = discovery::engine::DiscoveryEngine::new(cancel_flag.clone());

    let progress_sink = state.event_sink();
    let partial_sink = state.event_sink();
    let progress_cancel = cancel_flag.clone();
    let partial_cancel = cancel_flag.clone();
    let suggestions = engine
        .generate_suggestions(
            pick_seeds(seeds),
            all_excluded,
            seen_ids,
            yt_dlp_bin,
            move |current, total, seed_name| {
                if !background && !progress_cancel.load(Ordering::Relaxed) {
                    progress_sink.emit(CoreEvent::DiscoveryProgress {
                        current,
                        total,
                        seed_name: seed_name.to_string(),
                    });
                }
            },
            move |partial_suggestions| {
                if !background && !partial_cancel.load(Ordering::Relaxed) {
                    partial_sink.emit(CoreEvent::DiscoveryPartial(partial_suggestions.to_vec()));
                }
            },
        )
        .await;

    if cancel_flag.load(Ordering::Relaxed) {
        return Err("Discovery cancelled".to_string());
    }

    if background {
        discovery::cache::DiscoveryCache::append_suggestions(
            &profile_id,
            suggestions.clone(),
            &seed_hash,
        )
        .ok();
    } else {
        discovery::cache::DiscoveryCache::replace_suggestions(
            &profile_id,
            suggestions.clone(),
            &seed_hash,
        )
        .ok();

        state.emit(CoreEvent::DiscoveryComplete {
            count: suggestions.len(),
        });
    }

    Ok(suggestions)
}

pub fn cancel_discovery(state: &AppState) {
    state
        .discovery_cancel
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .store(true, Ordering::Relaxed);
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryCacheResponse {
    pub suggestions: Vec<DiscoverySuggestion>,
    pub cursor_index: usize,
    pub revealed_count: usize,
    pub visited_index: i32,
    /// Built from other seeds than the profile has now: refresh soon.
    pub stale: bool,
}

/// The saved carousel, without what the profile gained or disliked since
/// (a suggestion added to the profile must not come back).
pub fn cached_suggestions(profile_id: &str) -> Option<DiscoveryCacheResponse> {
    let data = discovery::cache::DiscoveryCache::load(profile_id)?;
    let profile = storage::load_profile(profile_id.to_string()).ok();
    let (owned, stale): (std::collections::HashSet<String>, bool) = match &profile {
        Some(profile) => (
            profile_seeds(profile)
                .into_iter()
                .map(|s| s.video_id)
                .chain(profile.disliked_videos.iter().cloned())
                .collect(),
            profile_seed_hash(profile) != data.seed_hash,
        ),
        None => (Default::default(), false),
    };
    // Cursor fields index the saved list; shift them past removed entries.
    let mut cursor_index = data.cursor_index;
    let mut visited_index = data.visited_index;
    let mut suggestions = Vec::with_capacity(data.suggestions.len());
    for (index, suggestion) in data.suggestions.into_iter().enumerate() {
        if owned.contains(&suggestion.video_id) {
            if index < data.cursor_index {
                cursor_index = cursor_index.saturating_sub(1);
            }
            if (index as i32) <= data.visited_index {
                visited_index -= 1;
            }
            continue;
        }
        suggestions.push(suggestion);
    }
    Some(DiscoveryCacheResponse {
        suggestions,
        cursor_index,
        revealed_count: data.revealed_count,
        visited_index: visited_index.max(-1),
        stale,
    })
}

fn remove_cached_audio_in_background(state: &AppState, video_id: String) {
    let cache = state.youtube_cache.clone();
    std::thread::spawn(move || {
        if let Ok(mut cache) = cache.lock() {
            cache.ensure_loaded();
            cache.remove_entry_by_video_id(&video_id);
        }
    });
}

/// Hide a suggestion for this discovery cache only.
pub fn dismiss(state: &AppState, profile_id: &str, video_id: String) -> Result<(), String> {
    discovery::cache::DiscoveryCache::dismiss(profile_id, &video_id)?;
    remove_cached_audio_in_background(state, video_id);
    Ok(())
}

/// Never suggest this video again for the profile.
pub fn dislike(state: &AppState, profile_id: &str, video_id: String) -> Result<(), String> {
    storage::update_disliked_videos(profile_id, &video_id, true)?;
    let _ = discovery::cache::DiscoveryCache::dismiss(profile_id, &video_id);
    remove_cached_audio_in_background(state, video_id);
    Ok(())
}

pub fn undislike(profile_id: &str, video_id: &str) -> Result<(), String> {
    storage::update_disliked_videos(profile_id, video_id, false)
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DislikedVideoInfo {
    pub video_id: String,
    pub title: String,
    pub channel: String,
    pub duration: f64,
    pub url: String,
}

/// Disliked videos of a profile with their titles, fetched through yt-dlp (5 at a time).
pub async fn list_disliked_videos(profile_id: String) -> Result<Vec<DislikedVideoInfo>, String> {
    use futures::stream::{self, StreamExt};

    let profile = storage::load_profile(profile_id)?;

    if profile.disliked_videos.is_empty() {
        return Ok(Vec::new());
    }

    let yt_dlp_bin = match youtube::downloader::get_yt_dlp_bin() {
        Ok(path) => path,
        Err(_) => youtube::download_yt_dlp().await?,
    };

    let results: Vec<DislikedVideoInfo> = stream::iter(profile.disliked_videos.iter().cloned())
        .map(|video_id| {
            let bin = yt_dlp_bin.clone();
            async move {
                let url = youtube::downloader::canonical_url(&video_id);

                let mut cmd = youtube::downloader::yt_dlp_command(&bin);
                cmd.arg(&url)
                    .arg("--dump-json")
                    .arg("--no-download")
                    .arg("--no-warnings")
                    .arg("--no-check-certificates")
                    .arg("--no-playlist")
                    .arg("--socket-timeout")
                    .arg("10")
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped());

                if let Ok(child) = cmd.spawn() {
                    if let Ok(Ok(out)) = tokio::time::timeout(
                        std::time::Duration::from_secs(10),
                        child.wait_with_output(),
                    )
                    .await
                    {
                        if out.status.success() {
                            let stdout = String::from_utf8_lossy(&out.stdout);
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                                let title = json
                                    .get("title")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Unknown")
                                    .to_string();
                                let channel = json
                                    .get("channel")
                                    .or_else(|| json.get("uploader"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let duration =
                                    json.get("duration").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                return DislikedVideoInfo {
                                    video_id,
                                    title,
                                    channel,
                                    duration,
                                    url,
                                };
                            }
                        }
                    }
                }

                DislikedVideoInfo {
                    video_id: video_id.clone(),
                    title: format!("Video {}", video_id),
                    channel: String::new(),
                    duration: 0.0,
                    url,
                }
            }
        })
        .buffer_unordered(5)
        .collect()
        .await;

    Ok(results)
}
