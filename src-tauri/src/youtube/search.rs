use std::process::Stdio;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};

use super::cache::YouTubeCache;
use super::downloader::{canonical_url, extract_video_id, get_yt_dlp_bin, yt_dlp_command};
use super::yt_dlp_manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YoutubeSearchResult {
    pub video_id: String,
    pub title: String,
    pub duration: f64,
    pub channel: String,
    pub thumbnail_url: String,
    pub url: String,
    pub already_downloaded: bool,
}

/// Search YouTube using yt-dlp and return matching results.
pub async fn search_youtube(
    query: &str,
    max_results: u32,
    cache: Arc<Mutex<YouTubeCache>>,
) -> Result<Vec<YoutubeSearchResult>, String> {
    let yt_dlp_bin = match get_yt_dlp_bin() {
        Ok(path) => path,
        Err(_) => yt_dlp_manager::download_yt_dlp().await?,
    };
    let max = max_results.min(20).max(1);

    let search_query = format!("ytsearch{}:{}", max, query);

    let mut cmd = yt_dlp_command(&yt_dlp_bin);
    cmd.arg(&search_query)
        .arg("--flat-playlist")
        .arg("--dump-json")
        .arg("--no-download")
        .arg("--no-warnings")
        .arg("--no-check-certificates")
        .arg("--socket-timeout")
        .arg("10")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to run yt-dlp search: {}", e))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture yt-dlp stdout".to_string())?;

    let mut results = Vec::new();
    let mut reader = BufReader::new(stdout).lines();

    while let Ok(Some(line)) = reader.next_line().await {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
            let video_id = json
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if video_id.is_empty() {
                continue;
            }

            let title = json
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();

            let duration = json
                .get("duration")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            let channel = json
                .get("channel")
                .or_else(|| json.get("uploader"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let thumbnail_url = json
                .get("thumbnail")
                .or_else(|| json.get("thumbnails").and_then(|t| t.as_array()).and_then(|a| a.last()))
                .and_then(|v| {
                    if v.is_string() {
                        v.as_str().map(|s| s.to_string())
                    } else {
                        v.get("url").and_then(|u| u.as_str()).map(|s| s.to_string())
                    }
                })
                .unwrap_or_default();

            let url = canonical_url(&video_id);

            let already_downloaded = {
                let mut cache_guard = cache.lock().unwrap();
                cache_guard.get(&url).is_some()
            };

            results.push(YoutubeSearchResult {
                video_id,
                title,
                duration,
                channel,
                thumbnail_url,
                url,
                already_downloaded,
            });
        }
    }

    // Wait for process to finish
    let _ = child.wait().await;

    Ok(results)
}

/// Playlist entry info from yt-dlp flat playlist dump.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YoutubePlaylist {
    pub title: String,
    pub entries: Vec<YoutubeSearchResult>,
    pub total_count: usize,
}

/// Fetch playlist metadata and entries using yt-dlp --flat-playlist.
pub async fn fetch_playlist(
    url: &str,
    cache: Arc<Mutex<YouTubeCache>>,
) -> Result<YoutubePlaylist, String> {
    let yt_dlp_bin = match get_yt_dlp_bin() {
        Ok(path) => path,
        Err(_) => yt_dlp_manager::download_yt_dlp().await?,
    };

    let mut cmd = yt_dlp_command(&yt_dlp_bin);
    cmd.arg(url)
        .arg("--flat-playlist")
        .arg("--dump-json")
        .arg("--no-download")
        .arg("--no-warnings")
        .arg("--socket-timeout")
        .arg("15")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to run yt-dlp playlist fetch: {}", e))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture yt-dlp stdout".to_string())?;

    let mut entries = Vec::new();
    let mut playlist_title = String::new();
    let mut reader = BufReader::new(stdout).lines();

    while let Ok(Some(line)) = reader.next_line().await {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
            let video_id = json
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if video_id.is_empty() {
                continue;
            }

            // Try to get playlist title from first entry
            if playlist_title.is_empty() {
                playlist_title = json
                    .get("playlist_title")
                    .or_else(|| json.get("playlist"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Playlist")
                    .to_string();
            }

            let title = json
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();

            // Skip deleted, private, or unavailable videos
            let title_lower = title.to_lowercase();
            if title_lower.contains("[deleted video]")
                || title_lower.contains("[private video]")
                || title_lower.contains("[unavailable]")
            {
                continue;
            }

            let duration = json
                .get("duration")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            let channel = json
                .get("channel")
                .or_else(|| json.get("uploader"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let thumbnail_url = json
                .get("thumbnail")
                .or_else(|| json.get("thumbnails").and_then(|t| t.as_array()).and_then(|a| a.last()))
                .and_then(|v| {
                    if v.is_string() {
                        v.as_str().map(|s| s.to_string())
                    } else {
                        v.get("url").and_then(|u| u.as_str()).map(|s| s.to_string())
                    }
                })
                .unwrap_or_default();

            let entry_url = canonical_url(&video_id);

            let already_downloaded = {
                let mut cache_guard = cache.lock().unwrap();
                cache_guard.get(&entry_url).is_some()
            };

            entries.push(YoutubeSearchResult {
                video_id,
                title,
                duration,
                channel,
                thumbnail_url,
                url: entry_url,
                already_downloaded,
            });
        }
    }

    let _ = child.wait().await;

    let total_count = entries.len();

    Ok(YoutubePlaylist {
        title: playlist_title,
        entries,
        total_count,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamUrlResult {
    pub url: String,
    pub duration: f64,
    pub format: String,
}

/// In-memory cache for stream URLs (valid ~6h on YouTube CDN, we use 4h TTL).
fn stream_cache() -> &'static std::sync::Mutex<std::collections::HashMap<String, (StreamUrlResult, std::time::Instant)>> {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, (StreamUrlResult, std::time::Instant)>>> = std::sync::OnceLock::new();
    CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

const STREAM_URL_TTL_SECS: u64 = 4 * 3600; // 4 hours

/// Extract a direct audio stream URL for a YouTube video using yt-dlp.
/// The returned URL can be played directly via HTML5 `<audio>` (expires after ~6h).
/// Results are cached in memory for 4h to enable instant replay and pre-fetch.
/// Uses `--print` instead of `--dump-json` for faster output (skips full JSON serialization).
/// Prefers M4A (AAC) for maximum HTML5 `<audio>` compatibility.
pub async fn get_stream_url(video_id: &str) -> Result<StreamUrlResult, String> {
    // Check cache first
    {
        let cache = stream_cache().lock().unwrap();
        if let Some((result, created_at)) = cache.get(video_id) {
            if created_at.elapsed() < std::time::Duration::from_secs(STREAM_URL_TTL_SECS) {
                return Ok(result.clone());
            }
        }
    }

    let yt_dlp_bin = match get_yt_dlp_bin() {
        Ok(path) => path,
        Err(_) => yt_dlp_manager::download_yt_dlp().await?,
    };

    let video_url = canonical_url(video_id);

    let mut cmd = yt_dlp_command(&yt_dlp_bin);
    cmd.arg(&video_url)
        .arg("-f")
        .arg("ba[ext=m4a]/ba")
        .arg("--print")
        .arg("url")
        .arg("--print")
        .arg("duration")
        .arg("--print")
        .arg("ext")
        .arg("--no-download")
        .arg("--no-warnings")
        .arg("--no-playlist")
        .arg("--no-check-certificates")
        .arg("--socket-timeout")
        .arg("10")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to run yt-dlp stream extraction: {}", e))?;

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        child.wait_with_output(),
    )
    .await
    .map_err(|_| "yt-dlp stream extraction timed out".to_string())?
    .map_err(|e| format!("yt-dlp stream extraction failed: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("yt-dlp stream extraction failed: {}", stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.trim().lines().collect();

    if lines.is_empty() {
        return Err("No output from yt-dlp stream extraction".to_string());
    }

    let url = lines
        .first()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "No stream URL in yt-dlp output".to_string())?
        .to_string();

    let duration = lines
        .get(1)
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);

    let format = lines
        .get(2)
        .unwrap_or(&"m4a")
        .to_string();

    let result = StreamUrlResult {
        url,
        duration,
        format,
    };

    // Store in cache (evict expired entries opportunistically)
    {
        let mut cache = stream_cache().lock().unwrap();
        let ttl = std::time::Duration::from_secs(STREAM_URL_TTL_SECS);
        cache.retain(|_, (_, t)| t.elapsed() < ttl);
        cache.insert(video_id.to_string(), (result.clone(), std::time::Instant::now()));
    }

    Ok(result)
}

/// Strip the `list=` parameter from a YouTube URL, returning just the video URL.
#[allow(dead_code)]
pub fn strip_playlist_param(url: &str) -> String {
    if let Some(video_id) = extract_video_id(url) {
        canonical_url(&video_id)
    } else {
        url.to_string()
    }
}

/// Check if a URL contains a playlist parameter.
#[allow(dead_code)]
pub fn has_playlist_param(url: &str) -> bool {
    url.contains("list=")
}

/// Check if a URL is a pure playlist (has list= but no v=).
#[allow(dead_code)]
pub fn is_pure_playlist(url: &str) -> bool {
    has_playlist_param(url) && extract_video_id(url).is_none()
}
