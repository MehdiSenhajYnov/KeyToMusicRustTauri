use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::youtube::downloader::{canonical_url, yt_dlp_command};
use crate::youtube::search::YoutubeSearchResult;

/// Fetch YouTube Mix (Radio) recommendations for a given video.
/// Returns an empty Vec on failure (some videos have no mix).
pub async fn fetch_mix(
    video_id: &str,
    yt_dlp_bin: &PathBuf,
) -> Vec<YoutubeSearchResult> {
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
        .arg("--socket-timeout")
        .arg("15")
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => return Vec::new(),
    };

    let mut results = Vec::new();
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

            if vid.is_empty() || vid == video_id {
                continue; // Skip the seed video itself
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
    results
}
