use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::youtube::downloader::{canonical_url, yt_dlp_command};
use crate::youtube::search::YoutubeSearchResult;

/// Entries kept per Mix. YouTube now pages Mixes out to 500+ entries that
/// take ~20 s to list and drift far from the seed; the head is what matters
/// (and lists in ~3 s).
const MIX_ENTRIES: usize = 50;

/// Hard cap on one Mix listing, so a stalled yt-dlp never blocks discovery.
const MIX_TIMEOUT: Duration = Duration::from_secs(20);

/// Fetch YouTube Mix (Radio) recommendations for a given video.
/// Returns an empty Vec on failure (some videos have no mix).
pub async fn fetch_mix(video_id: &str, yt_dlp_bin: &PathBuf) -> Vec<YoutubeSearchResult> {
    let mut results = Vec::new();
    // Keep whatever was read if the listing times out.
    let _ = tokio::time::timeout(MIX_TIMEOUT, read_mix(video_id, yt_dlp_bin, &mut results)).await;
    results
}

async fn read_mix(video_id: &str, yt_dlp_bin: &PathBuf, results: &mut Vec<YoutubeSearchResult>) {
    let url = format!(
        "https://www.youtube.com/watch?v={}&list=RD{}",
        video_id, video_id
    );

    let mut cmd = yt_dlp_command(yt_dlp_bin);
    cmd.arg(&url)
        .arg("--flat-playlist")
        .arg("--dump-json")
        .arg("--no-download")
        .arg("--no-warnings")
        .arg("--playlist-end")
        .arg(MIX_ENTRIES.to_string())
        .arg("--socket-timeout")
        .arg("15")
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(_) => return,
    };

    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => return,
    };

    // Mixes repeat entries; count each video once per seed.
    let mut seen = HashSet::new();
    let mut reader = BufReader::new(stdout).lines();

    while let Ok(Some(line)) = reader.next_line().await {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
            let vid = json
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if vid.is_empty() || vid == video_id || !seen.insert(vid.clone()) {
                continue; // Skip the seed video itself and repeats
            }

            let title = json
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();

            let duration = json.get("duration").and_then(|v| v.as_f64()).unwrap_or(0.0);

            let channel = json
                .get("channel")
                .or_else(|| json.get("uploader"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let thumbnail_url = json
                .get("thumbnail")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let entry_url = canonical_url(&vid);

            results.push(YoutubeSearchResult {
                video_id: vid,
                title,
                duration,
                channel,
                thumbnail_url,
                url: entry_url,
                already_downloaded: false,
            });
        }
    }

    let _ = child.wait().await;
}
