use std::path::PathBuf;

use crate::storage::config::get_app_data_dir;

/// Get the platform-specific ffmpeg binary name.
fn ffmpeg_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    }
}

/// Get the download URL for ffmpeg based on the current platform.
fn ffmpeg_download_url() -> &'static str {
    if cfg!(target_os = "windows") {
        "https://github.com/yt-dlp/FFmpeg-Builds/releases/latest/download/ffmpeg-master-latest-win64-gpl.zip"
    } else if cfg!(target_os = "macos") {
        // yt-dlp/FFmpeg-Builds no longer provides macOS builds, use evermeet.cx instead
        "https://evermeet.cx/ffmpeg/getrelease/ffmpeg/zip"
    } else {
        "https://github.com/yt-dlp/FFmpeg-Builds/releases/latest/download/ffmpeg-master-latest-linux64-gpl.tar.xz"
    }
}

/// Get the path where ffmpeg should be stored (same dir as yt-dlp so it's found automatically).
pub fn get_ffmpeg_path() -> PathBuf {
    get_app_data_dir().join("bin").join(ffmpeg_binary_name())
}

/// Check if ffmpeg is available (in app bin dir or in PATH).
pub fn find_ffmpeg() -> Option<PathBuf> {
    let local_path = get_ffmpeg_path();
    if local_path.exists() {
        if let Ok(meta) = std::fs::metadata(&local_path) {
            if meta.len() > 1_000_000 {
                return Some(local_path);
            }
            let _ = std::fs::remove_file(&local_path);
        }
    }

    // Check PATH
    if which_ffmpeg().is_some() {
        return Some(PathBuf::from("ffmpeg"));
    }

    None
}

/// Check if ffmpeg exists in system PATH.
fn which_ffmpeg() -> Option<PathBuf> {
    let name = ffmpeg_binary_name();
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).find_map(|dir| {
            let full_path = dir.join(name);
            if full_path.is_file() {
                Some(full_path)
            } else {
                None
            }
        })
    })
}

/// Check if ffmpeg is installed.
pub fn is_installed() -> bool {
    find_ffmpeg().is_some()
}

/// Download ffmpeg to the app's bin directory.
/// On Windows/macOS: downloads ZIP and extracts ffmpeg binary.
/// On Linux: downloads tar.xz and extracts ffmpeg binary.
pub async fn download_ffmpeg() -> Result<PathBuf, String> {
    let target_path = get_ffmpeg_path();
    let bin_dir = target_path.parent().unwrap().to_path_buf();

    std::fs::create_dir_all(&bin_dir)
        .map_err(|e| format!("Failed to create bin directory: {}", e))?;

    let url = ffmpeg_download_url();

    let response = reqwest::get(url)
        .await
        .map_err(|e| format!("Failed to download ffmpeg: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Failed to download ffmpeg: HTTP {}", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read ffmpeg download: {}", e))?;

    // Extract ffmpeg binary from the archive
    if cfg!(target_os = "linux") {
        extract_from_tar_xz(&bytes, &target_path)?;
    } else {
        extract_from_zip(&bytes, &target_path)?;
    }

    // Set executable permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&target_path, perms)
            .map_err(|e| format!("Failed to set ffmpeg permissions: {}", e))?;
    }

    // Verify the file exists and has reasonable size
    let meta = std::fs::metadata(&target_path)
        .map_err(|e| format!("ffmpeg extraction failed: {}", e))?;
    if meta.len() < 1_000_000 {
        let _ = std::fs::remove_file(&target_path);
        return Err("ffmpeg extraction produced invalid file".to_string());
    }

    Ok(target_path)
}

/// Extract ffmpeg binary from a ZIP archive.
fn extract_from_zip(data: &[u8], target_path: &PathBuf) -> Result<(), String> {
    use std::io::Read;

    let cursor = std::io::Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| format!("Failed to open ffmpeg archive: {}", e))?;

    let ffmpeg_name = ffmpeg_binary_name();

    // Find the ffmpeg binary in the archive
    // Try two patterns:
    // 1. bin/ffmpeg (yt-dlp FFmpeg-Builds structure)
    // 2. ffmpeg directly at root (evermeet.cx structure)
    let mut found = false;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .map_err(|e| format!("Failed to read archive entry: {}", e))?;

        let name = file.name().to_string();

        // Match either: ends with bin/ffmpeg OR is exactly "ffmpeg" (or ffmpeg.exe on Windows)
        let is_bin_path = name.ends_with(ffmpeg_name) && (name.contains("/bin/") || name.contains("\\bin\\"));
        let is_root_path = name == ffmpeg_name;

        if is_bin_path || is_root_path {
            let mut contents = Vec::new();
            file.read_to_end(&mut contents)
                .map_err(|e| format!("Failed to extract ffmpeg: {}", e))?;

            std::fs::write(target_path, &contents)
                .map_err(|e| format!("Failed to write ffmpeg: {}", e))?;

            found = true;
            break;
        }
    }

    if !found {
        return Err("ffmpeg binary not found in archive".to_string());
    }

    Ok(())
}

/// Extract ffmpeg binary from a tar.xz archive (Linux).
fn extract_from_tar_xz(_data: &[u8], _target_path: &PathBuf) -> Result<(), String> {
    // On Linux, use system tar to extract
    // This is a fallback - Linux users typically have ffmpeg via package manager
    Err("On Linux, please install ffmpeg via your package manager (apt install ffmpeg, pacman -S ffmpeg, etc.)".to_string())
}
