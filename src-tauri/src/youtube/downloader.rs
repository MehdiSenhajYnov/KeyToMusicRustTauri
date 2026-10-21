use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::storage::config::get_app_data_dir;

use super::cache::{CacheEntry, YouTubeCache};
use super::yt_dlp_manager;

/// Progress callback type for download status updates.
pub type ProgressCallback = Box<dyn Fn(&str, Option<f64>) + Send + Sync>;

/// Create a Command for yt-dlp with proper platform settings.
pub(crate) fn yt_dlp_command(bin: &PathBuf) -> Command {
    let mut cmd = Command::new(bin);
    cmd.stdin(Stdio::null());
    cmd.kill_on_drop(true);

    // Hide console window on Windows
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

    cmd
}

/// Extracts the video ID from a YouTube URL.
/// Supports youtube.com/watch?v=ID, youtu.be/ID, and youtube.com/shorts/ID formats.
pub fn extract_video_id(input: &str) -> Option<String> {
    let url = reqwest::Url::parse(input.trim()).ok()?;
    if !matches!(url.scheme(), "https" | "http")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    let id = match url.host_str()? {
        "youtu.be" => url.path().trim_start_matches('/').to_string(),
        "youtube.com" | "www.youtube.com" | "m.youtube.com" | "music.youtube.com" => {
            if url.path() == "/watch" {
                url.query_pairs()
                    .find(|(key, _)| key == "v")?
                    .1
                    .into_owned()
            } else {
                let mut segments = url.path_segments()?;
                if !matches!(segments.next()?, "shorts" | "live" | "embed") {
                    return None;
                }
                segments.next()?.to_string()
            }
        }
        _ => return None,
    };
    (id.len() == 11
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_'))
    .then_some(id)
}

pub fn is_valid_youtube_url(url: &str) -> bool {
    extract_video_id(url).is_some()
}

// A discovery predownload and a manual download may request the same file together.
// Serialize by video ID before checking the cache or removing partial files.
static DOWNLOAD_SLOTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(3);

static DOWNLOAD_LOCKS: std::sync::OnceLock<
    Mutex<std::collections::HashMap<String, std::sync::Weak<tokio::sync::Mutex<()>>>>,
> = std::sync::OnceLock::new();

fn download_lock(id: &str) -> Arc<tokio::sync::Mutex<()>> {
    let mut locks = DOWNLOAD_LOCKS
        .get_or_init(|| Mutex::new(std::collections::HashMap::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    locks.retain(|_, lock| lock.strong_count() > 0);
    if let Some(lock) = locks.get(id).and_then(std::sync::Weak::upgrade) {
        return lock;
    }
    let lock = Arc::new(tokio::sync::Mutex::new(()));
    locks.insert(id.to_string(), Arc::downgrade(&lock));
    lock
}

/// Get the yt-dlp binary path, or return an error message.
pub(crate) fn get_yt_dlp_bin() -> Result<PathBuf, String> {
    yt_dlp_manager::find_yt_dlp().ok_or_else(|| "yt-dlp is not installed".to_string())
}

/// Build a canonical YouTube URL from a video ID (for consistent cache lookups).
pub(crate) fn canonical_url(video_id: &str) -> String {
    format!("https://www.youtube.com/watch?v={}", video_id)
}

/// Download a YouTube video audio using yt-dlp.
/// Uses --print for title extraction and ensures ffmpeg is available for proper remuxing.
/// The progress callback receives (status_message, optional_progress_percentage).
pub async fn download_audio(
    url: &str,
    cache: Arc<Mutex<YouTubeCache>>,
    on_progress: Option<ProgressCallback>,
) -> Result<CacheEntry, String> {
    let emit = |msg: &str, pct: Option<f64>| {
        if let Some(ref cb) = on_progress {
            cb(msg, pct);
        }
    };

    // Validate URL
    if !is_valid_youtube_url(url) {
        return Err("Invalid YouTube URL".to_string());
    }

    let video_id =
        extract_video_id(url).ok_or_else(|| "Could not extract video ID from URL".to_string())?;

    let lock = download_lock(&video_id);
    let _guard = lock.lock().await;

    // Use canonical URL for cache lookups (strip list=, pp=, etc.)
    let cache_url = canonical_url(&video_id);

    // Check cache first (by canonical URL)
    {
        let mut cache_guard = cache.lock().unwrap();
        if let Some(entry) = cache_guard.get(&cache_url) {
            return Ok(entry.clone());
        }
    }

    let _slot = DOWNLOAD_SLOTS.acquire().await.map_err(|e| e.to_string())?;

    // Get yt-dlp binary path, auto-install if not found
    let yt_dlp_bin = match get_yt_dlp_bin() {
        Ok(path) => path,
        Err(_) => {
            emit("Installing yt-dlp...", None);
            yt_dlp_manager::download_yt_dlp().await?
        }
    };

    // Ensure ffmpeg is available for proper audio remuxing
    if !super::ffmpeg_manager::is_installed() {
        emit("Installing ffmpeg...", None);
        super::ffmpeg_manager::download_ffmpeg().await?;
    }

    let cache_dir = get_app_data_dir().join("cache");
    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("Failed to create cache directory: {}", e))?;

    // Clean any previous partial downloads for this video
    clean_video_files(&cache_dir, &video_id);

    emit("Downloading...", Some(0.0));

    // Output template uses video ID only (predictable path)
    let output_template = cache_dir
        .join("%(id)s.%(ext)s")
        .to_string_lossy()
        .to_string();

    // Retry loop for transient network errors
    let max_attempts = 3;
    let mut last_err = String::new();
    let mut did_update_ytdlp = false;
    let mut current_bin = yt_dlp_bin;

    for attempt in 1..=max_attempts {
        if attempt > 1 {
            // Clean partial files before retry
            clean_video_files(&cache_dir, &video_id);
            emit("Retrying...", Some(0.0));
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }

        // Build yt-dlp command with --write-info-json for title extraction
        let mut cmd = yt_dlp_command(&current_bin);
        cmd.arg("-f")
            .arg("bestaudio[ext=m4a]")
            .arg("-o")
            .arg(&output_template)
            .arg("--write-info-json")
            .arg("--no-playlist")
            .arg("--newline")
            .arg("--force-overwrite")
            .arg("--no-update")
            .arg("--socket-timeout")
            .arg("30");

        // Point yt-dlp to our ffmpeg location if it's in the app bin dir
        let ffmpeg_path = super::ffmpeg_manager::get_ffmpeg_path();
        if ffmpeg_path.exists() {
            if let Some(bin_dir) = ffmpeg_path.parent() {
                cmd.arg("--ffmpeg-location").arg(bin_dir);
            }
        }

        let mut child = cmd
            .arg("--")
            .arg(&cache_url)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to run yt-dlp: {}", e))?;

        // Read stderr for progress and errors
        let has_progress = on_progress.is_some();
        let mut last_error_lines: Vec<String> = Vec::new();
        let stderr = child.stderr.take();
        if let Some(stderr) = stderr {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                if let Some(pct) = parse_progress_line(&line) {
                    if has_progress {
                        emit("Downloading...", Some(pct));
                    }
                } else if has_progress && (line.contains("[FixupM4a]") || line.contains("[Remux]"))
                {
                    emit("Processing...", None);
                }
                if line.contains("ERROR") || line.contains("error:") {
                    last_error_lines.push(line);
                }
            }
        }

        let status = child
            .wait()
            .await
            .map_err(|e| format!("yt-dlp process error: {}", e))?;

        if status.success() {
            // Download succeeded
            emit("Finalizing...", None);

            // Read title from the info.json file written by yt-dlp
            let title = read_title_from_info_json(&cache_dir, &video_id);

            // Find the downloaded file
            let actual_path = find_downloaded_file(&cache_dir, &video_id)?;
            return finalize_download(&cache_url, &actual_path, &title, cache);
        }

        // Download failed
        let stderr_text = last_error_lines.join("\n");

        // If error looks like outdated yt-dlp and we haven't updated yet, try that
        if !did_update_ytdlp && is_likely_outdated_ytdlp(&stderr_text) {
            did_update_ytdlp = true;
            emit("Updating yt-dlp...", None);
            tracing::info!(
                "yt-dlp download failed (possibly outdated), attempting update. Error: {}",
                stderr_text
            );
            match yt_dlp_manager::download_yt_dlp().await {
                Ok(new_bin) => {
                    current_bin = new_bin;
                    // Don't count this as a retry — loop will increment and retry
                    last_err = stderr_text;
                    continue;
                }
                Err(update_err) => {
                    tracing::warn!("yt-dlp update failed: {}", update_err);
                    // Fall through to normal error handling
                }
            }
        }

        let is_network_error = is_retryable_error(&stderr_text);

        if !is_network_error || attempt == max_attempts {
            // Non-retryable error or final attempt
            if stderr_text.is_empty() {
                return Err("Download failed. Check the URL and try again.".to_string());
            }
            return Err(parse_yt_dlp_error(&stderr_text));
        }

        last_err = stderr_text;
    }

    // Should not reach here, but just in case
    Err(parse_yt_dlp_error(&last_err))
}

/// Parse a yt-dlp output line for download progress percentage.
fn parse_progress_line(line: &str) -> Option<f64> {
    // yt-dlp outputs lines like: [download]  45.2% of 5.43MiB at 1.23MiB/s
    if !line.contains("[download]") || !line.contains('%') {
        return None;
    }
    let pct_str = line.split('%').next()?;
    let num_str = pct_str.trim().rsplit_once(']')?.1.trim();
    num_str.parse::<f64>().ok()
}

/// Finalize a download by adding it to the cache.
fn finalize_download(
    url: &str,
    file_path: &PathBuf,
    title: &str,
    cache: Arc<Mutex<YouTubeCache>>,
) -> Result<CacheEntry, String> {
    let file_size = std::fs::metadata(file_path).map(|m| m.len()).unwrap_or(0);

    let cached_path = file_path.to_string_lossy().to_string();

    let mut cache_guard = cache.lock().unwrap();
    let entry = cache_guard.add_entry(url.to_string(), cached_path, title.to_string(), file_size);
    cache_guard.save_index().ok();

    Ok(entry)
}

/// Clean any previous files for a video ID in the cache directory.
fn clean_video_files(cache_dir: &PathBuf, video_id: &str) {
    if let Ok(entries) = std::fs::read_dir(cache_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            // Only match files that start with the exact video_id (avoid partial matches)
            if name.starts_with(video_id) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}

/// Try to find the downloaded file in cache_dir by video_id.
/// Prefers exact match (video_id.ext) over partial matches.
fn find_downloaded_file(cache_dir: &PathBuf, video_id: &str) -> Result<PathBuf, String> {
    let audio_extensions = [
        ".m4a", ".mp3", ".opus", ".webm", ".ogg", ".wav", ".flac", ".aac",
    ];

    // First try exact match: {video_id}.{ext}
    for ext in &audio_extensions {
        let exact_path = cache_dir.join(format!("{}{}", video_id, ext));
        if exact_path.exists() {
            return Ok(exact_path);
        }
    }

    // Fallback: scan directory for files starting with video_id
    if let Ok(entries) = std::fs::read_dir(cache_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(video_id) && audio_extensions.iter().any(|ext| name.ends_with(ext))
            {
                return Ok(entry.path());
            }
        }
    }
    Err(format!("Downloaded file not found for video {}", video_id))
}

/// Read the video title from the info.json file written by yt-dlp.
/// Returns the title or falls back to the video_id. Cleans up the JSON file afterwards.
fn read_title_from_info_json(cache_dir: &PathBuf, video_id: &str) -> String {
    let info_path = cache_dir.join(format!("{}.info.json", video_id));
    let title = if info_path.exists() {
        let content = std::fs::read_to_string(&info_path).unwrap_or_default();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap_or_default();
        let t = parsed
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        // Clean up the info.json file
        let _ = std::fs::remove_file(&info_path);
        t
    } else {
        String::new()
    };
    if title.is_empty() {
        video_id.to_string()
    } else {
        title
    }
}

/// Check if a yt-dlp error is a transient network issue worth retrying.
fn is_retryable_error(stderr: &str) -> bool {
    let lower = stderr.to_lowercase();

    // HTTP 403/429 are NOT transient network errors — they indicate
    // YouTube is blocking the request (usually outdated yt-dlp)
    if lower.contains("403") || lower.contains("forbidden") || lower.contains("429") {
        return false;
    }
    // Extraction failures are yt-dlp bugs, not network issues
    if lower.contains("unable to extract") || lower.contains("no video formats") {
        return false;
    }

    lower.contains("timed out") ||
    lower.contains("timeout") ||
    lower.contains("urlopen error") ||
    lower.contains("incomplete read") ||
    lower.contains("getaddrinfo failed") ||
    lower.contains("name or service not known") ||
    lower.contains("errno 22") ||
    // Only match actual connection resets/failures, not HTTP status errors
    lower.contains("connection reset") ||
    lower.contains("connection refused") ||
    lower.contains("connection aborted")
}

/// Check if an error looks like it could be fixed by updating yt-dlp.
fn is_likely_outdated_ytdlp(stderr: &str) -> bool {
    let lower = stderr.to_lowercase();
    lower.contains("403")
        || lower.contains("forbidden")
        || lower.contains("unable to extract")
        || lower.contains("no video formats")
        || lower.contains("unable to download webpage")
        || lower.contains("unsupported url")
        || lower.contains("sign in to confirm")
}

/// Parse yt-dlp error output into a user-friendly message.
fn parse_yt_dlp_error(stderr: &str) -> String {
    let lower = stderr.to_lowercase();

    if lower.contains("private video") || lower.contains("sign in") {
        return "This video is private or requires sign-in".to_string();
    }
    if lower.contains("not available") || lower.contains("unavailable") {
        return "This video is not available".to_string();
    }
    if lower.contains("not a valid url") {
        return "Invalid YouTube URL".to_string();
    }
    if lower.contains("geo") || lower.contains("country") {
        return "This video is not available in your region".to_string();
    }
    // Only true network errors (after the outdated-yt-dlp ones are filtered)
    if lower.contains("timed out")
        || lower.contains("timeout")
        || lower.contains("getaddrinfo")
        || lower.contains("name or service not known")
        || lower.contains("connection reset")
        || lower.contains("connection refused")
    {
        return "Network error. Check your internet connection".to_string();
    }

    // Fallback: return the first meaningful line
    let first_error = stderr
        .lines()
        .find(|l| l.contains("ERROR"))
        .unwrap_or(stderr.lines().next().unwrap_or("Unknown error"));

    format!("Download failed: {}", first_error.trim())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn youtube_urls_are_parsed_by_host_and_exact_ascii_id() {
        for url in [
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ&list=PL",
            "https://youtu.be/dQw4w9WgXcQ?t=20",
            "https://m.youtube.com/shorts/dQw4w9WgXcQ",
            "https://youtube.com/live/dQw4w9WgXcQ",
        ] {
            assert_eq!(extract_video_id(url).as_deref(), Some("dQw4w9WgXcQ"));
        }
        for url in [
            "https://evil.test/youtube.com/watch?v=dQw4w9WgXcQ",
            "https://youtube.com.evil.test/watch?v=dQw4w9WgXcQ",
            "https://youtube.com/watch?v=éééééééé",
            "https://youtu.be/short",
            "--exec=evil",
            "file:///youtube.com/watch?v=dQw4w9WgXcQ",
        ] {
            assert_eq!(extract_video_id(url), None, "{url}");
        }
    }
    #[test]
    fn concurrent_requests_share_the_same_video_lock() {
        let first = download_lock("same");
        let second = download_lock("same");
        assert!(Arc::ptr_eq(&first, &second));
        assert!(!Arc::ptr_eq(&first, &download_lock("other")));
    }
}
