use std::path::{Path, PathBuf};
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

/// Name prefix of the per-download temp dirs inside the cache directory.
pub(crate) const TEMP_DIR_PREFIX: &str = ".dl-";

/// Upper bound for one yt-dlp download run (the process is killed on expiry).
const DOWNLOAD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10 * 60);

/// Whether a download for this video ID currently holds its lock.
pub(crate) fn is_download_active(id: &str) -> bool {
    DOWNLOAD_LOCKS
        .get()
        .and_then(|locks| {
            locks
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(id)
                .map(|lock| lock.strong_count() > 0)
        })
        .unwrap_or(false)
}

/// yt-dlp only enables deno by default for YouTube's JS challenges; point it
/// at node when that is the only runtime installed.
pub(crate) fn add_js_runtime_args(cmd: &mut Command) {
    static USE_NODE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    let use_node = *USE_NODE.get_or_init(|| !is_on_path("deno") && is_on_path("node"));
    if use_node {
        cmd.arg("--js-runtimes").arg("node");
    }
}

/// `-x` needs ffprobe as well as ffmpeg; the managed install only ships ffmpeg.
/// Without it, only native m4a streams are accepted (symphonia cannot decode opus).
fn can_extract_audio() -> bool {
    let local_ffprobe =
        super::ffmpeg_manager::get_ffmpeg_path().with_file_name(if cfg!(target_os = "windows") {
            "ffprobe.exe"
        } else {
            "ffprobe"
        });
    super::ffmpeg_manager::is_installed() && (local_ffprobe.is_file() || is_on_path("ffprobe"))
}

fn is_on_path(program: &str) -> bool {
    let file = if cfg!(target_os = "windows") {
        format!("{}.exe", program)
    } else {
        program.to_string()
    };
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|dir| dir.join(&file).is_file()))
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

    let cache_dir = get_app_data_dir().join("cache");
    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("Failed to create cache directory: {}", e))?;

    // A complete file can outlive its index entry (lost or corrupt index):
    // re-index it instead of downloading it again.
    if let Some(existing) = existing_audio_file(&cache_dir, &video_id) {
        emit("Finalizing...", None);
        let title = fetch_title(&cache_url)
            .await
            .unwrap_or_else(|| video_id.clone());
        return finalize_download(&cache_url, &existing, &title, cache);
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

    emit("Downloading...", Some(0.0));

    // Retry loop for transient network errors
    let max_attempts = 3;
    let mut last_err = String::new();
    let mut did_update_ytdlp = false;
    let mut current_bin = yt_dlp_bin;

    for attempt in 1..=max_attempts {
        if attempt > 1 {
            emit("Retrying...", Some(0.0));
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }

        // Each attempt downloads into its own temp dir next to the cache (same
        // filesystem, so the final move is a rename). Partial files never touch
        // the cache and are removed when the dir drops.
        let temp_dir = tempfile::Builder::new()
            .prefix(TEMP_DIR_PREFIX)
            .tempdir_in(&cache_dir)
            .map_err(|e| format!("Failed to create download directory: {}", e))?;
        // Output template uses video ID only (predictable path)
        let output_template = temp_dir
            .path()
            .join("%(id)s.%(ext)s")
            .to_string_lossy()
            .to_string();

        // Build yt-dlp command with --write-info-json for title extraction
        let mut cmd = yt_dlp_command(&current_bin);
        if can_extract_audio() {
            // Take any audio stream, converting to m4a only when YouTube has no m4a one.
            cmd.args(["-f", "ba[ext=m4a]/ba", "-x", "--audio-format", "m4a"]);
        } else {
            cmd.args(["-f", "ba[ext=m4a]"]);
        }
        cmd.arg("-o")
            .arg(&output_template)
            .arg("--write-info-json")
            .arg("--no-playlist")
            // A live stream never ends: refuse it at once rather than time out.
            .arg("--match-filter")
            .arg("live_status!=is_live & live_status!=is_upcoming")
            .arg("--newline")
            .arg("--force-overwrite")
            .arg("--no-update")
            .arg("--socket-timeout")
            .arg("30");
        add_js_runtime_args(&mut cmd);

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
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to run yt-dlp: {}", e))?;

        // Errors go to stderr; collect them concurrently so neither pipe fills up.
        let stderr_task = child.stderr.take().map(|stderr| {
            tokio::spawn(async move {
                let mut errors = Vec::new();
                let mut reader = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    if line.contains("ERROR") || line.contains("error:") {
                        errors.push(line);
                    }
                }
                errors
            })
        });

        // Progress lines ([download] NN%, post-processors) go to stdout.
        let has_progress = on_progress.is_some();
        let stdout = child.stdout.take();
        let run = async {
            if let Some(stdout) = stdout {
                let mut reader = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    if !has_progress {
                        continue;
                    }
                    if let Some(pct) = parse_progress_line(&line) {
                        emit("Downloading...", Some(pct));
                    } else if line.contains("[FixupM4a]")
                        || line.contains("[Remux]")
                        || line.contains("[ExtractAudio]")
                    {
                        emit("Processing...", None);
                    }
                }
            }
            child.wait().await
        };
        let result = tokio::time::timeout(DOWNLOAD_TIMEOUT, run).await;
        let status = match result {
            Ok(status) => status.map_err(|e| format!("yt-dlp process error: {}", e))?,
            Err(_) => {
                let _ = child.kill().await;
                return Err("Download timed out".to_string());
            }
        };
        let last_error_lines = match stderr_task {
            Some(task) => task.await.unwrap_or_default(),
            None => Vec::new(),
        };

        if status.success() {
            // Download succeeded
            emit("Finalizing...", None);

            // Read title from the info.json file written by yt-dlp
            let title = read_title_from_info_json(temp_dir.path(), &video_id);

            // Move the finished file into the cache, replacing any stale copy.
            let downloaded = find_downloaded_file(temp_dir.path(), &video_id)?;
            let file_name = downloaded
                .file_name()
                .ok_or_else(|| "Downloaded file has no name".to_string())?;
            let actual_path = cache_dir.join(file_name);
            std::fs::rename(&downloaded, &actual_path)
                .map_err(|e| format!("Failed to move downloaded file: {}", e))?;
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

const AUDIO_EXTENSIONS: [&str; 8] = [
    ".m4a", ".mp3", ".opus", ".webm", ".ogg", ".wav", ".flac", ".aac",
];

/// A finished `{video_id}.{ext}` already in the cache. yt-dlp only gives the
/// final name to complete files (partials are `.part` / `.temp.*`).
fn existing_audio_file(cache_dir: &Path, video_id: &str) -> Option<PathBuf> {
    AUDIO_EXTENSIONS
        .iter()
        .map(|ext| cache_dir.join(format!("{}{}", video_id, ext)))
        .find(|path| std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.len() > 0))
}

/// Best-effort title lookup for a file re-indexed without its info.json.
async fn fetch_title(url: &str) -> Option<String> {
    let bin = get_yt_dlp_bin().ok()?;
    let mut cmd = yt_dlp_command(&bin);
    cmd.args(["--print", "title", "--skip-download", "--no-playlist"])
        .args(["--no-warnings", "--socket-timeout", "10"]);
    add_js_runtime_args(&mut cmd);
    cmd.arg("--")
        .arg(url)
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let output = tokio::time::timeout(std::time::Duration::from_secs(30), cmd.output())
        .await
        .ok()?
        .ok()?;
    let title = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (output.status.success() && !title.is_empty()).then_some(title)
}

/// Try to find the downloaded file in cache_dir by video_id.
/// Prefers exact match (video_id.ext) over partial matches.
fn find_downloaded_file(cache_dir: &Path, video_id: &str) -> Result<PathBuf, String> {
    let audio_extensions = AUDIO_EXTENSIONS;

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
fn read_title_from_info_json(cache_dir: &Path, video_id: &str) -> String {
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

    // HTTP 403/429 and bot checks are NOT transient network errors — they
    // indicate YouTube is blocking the request (often an outdated yt-dlp)
    if is_http_blocked(&lower) || lower.contains("sign in to confirm") {
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

/// `HTTP Error 403` / `HTTP Error 429` from yt-dlp (expects lowercased text).
fn is_http_blocked(lower: &str) -> bool {
    lower.contains("http error 403") || lower.contains("http error 429")
}

/// Check if an error looks like it could be fixed by updating yt-dlp.
/// Bot checks ("Sign in to confirm you're not a bot") are rate limiting, not
/// an outdated extractor, so they never trigger an update.
fn is_likely_outdated_ytdlp(stderr: &str) -> bool {
    let lower = stderr.to_lowercase();
    if lower.contains("sign in to confirm") {
        return false;
    }
    is_http_blocked(&lower)
        || lower.contains("unable to extract")
        || lower.contains("no video formats")
        || lower.contains("unable to download webpage")
        || lower.contains("unsupported url")
}

/// Parse yt-dlp error output into a user-friendly message.
fn parse_yt_dlp_error(stderr: &str) -> String {
    let lower = stderr.to_lowercase();

    if lower.contains("not a bot") {
        return "YouTube is temporarily blocking downloads. Try again later".to_string();
    }
    if lower.contains("confirm your age") {
        return "This video is age-restricted".to_string();
    }
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
        assert!(is_download_active("same"));
        drop((first, second));
        assert!(!is_download_active("same"));
    }
    #[test]
    fn only_precise_blocking_errors_trigger_an_update() {
        assert!(is_likely_outdated_ytdlp(
            "ERROR: unable to download video data: HTTP Error 403: Forbidden"
        ));
        assert!(!is_likely_outdated_ytdlp("ERROR: [youtube] abc: 403 views"));
        let bot = "ERROR: [youtube] abc: Sign in to confirm you're not a bot";
        assert!(!is_likely_outdated_ytdlp(bot));
        assert!(!is_retryable_error(bot));
        assert!(!parse_yt_dlp_error(bot).contains("private"));
    }
    #[test]
    fn progress_is_parsed_from_stdout_lines() {
        assert_eq!(
            parse_progress_line("[download]  45.2% of    5.43MiB at    1.23MiB/s ETA 00:03"),
            Some(45.2)
        );
        assert_eq!(parse_progress_line("[download] Destination: a.m4a"), None);
    }
}
