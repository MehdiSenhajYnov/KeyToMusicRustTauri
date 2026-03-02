use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, SystemTime};

use crate::storage::config::get_app_data_dir;

/// Maximum age before re-downloading yt-dlp (7 days).
const UPDATE_INTERVAL: Duration = Duration::from_secs(7 * 24 * 3600);

/// Get the platform-specific yt-dlp binary name.
fn yt_dlp_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "yt-dlp.exe"
    } else {
        "yt-dlp"
    }
}

/// Get the download URL for yt-dlp based on the current platform.
fn yt_dlp_download_url() -> &'static str {
    if cfg!(target_os = "windows") {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
    } else if cfg!(target_os = "macos") {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos"
    } else {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux"
    }
}

/// Get the path where yt-dlp should be stored in the app's data directory.
pub fn get_yt_dlp_path() -> PathBuf {
    get_app_data_dir().join("bin").join(yt_dlp_binary_name())
}

/// Check if the managed yt-dlp binary exists in `data/bin/`.
/// Does NOT fall back to system PATH — the system version (e.g. pip) may be
/// outdated and cause extraction errors. All callers use the auto-install
/// fallback pattern (`download_yt_dlp()`) when this returns `None`.
pub fn find_yt_dlp() -> Option<PathBuf> {
    let local_path = get_yt_dlp_path();
    if local_path.exists() {
        // Verify the file is at least 1MB (yt-dlp is ~20MB, corrupt files are smaller)
        if let Ok(meta) = std::fs::metadata(&local_path) {
            if meta.len() > 1_000_000 {
                return Some(local_path);
            }
            // Corrupt/incomplete file, remove it
            let _ = std::fs::remove_file(&local_path);
        }
    }

    None
}

/// Check if the managed yt-dlp binary is installed locally.
pub async fn is_installed() -> bool {
    find_yt_dlp().is_some()
}

/// Download yt-dlp to the app's data directory.
pub async fn download_yt_dlp() -> Result<PathBuf, String> {
    let target_path = get_yt_dlp_path();

    // Create bin directory
    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create bin directory: {}", e))?;
    }

    let url = yt_dlp_download_url();

    // Download the file
    let response = reqwest::get(url)
        .await
        .map_err(|e| format!("Failed to download yt-dlp: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to download yt-dlp: HTTP {}",
            response.status()
        ));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read download: {}", e))?;

    // Write to file using std::fs to ensure handle is released synchronously
    std::fs::write(&target_path, &bytes)
        .map_err(|e| format!("Failed to write yt-dlp: {}", e))?;

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&target_path, perms)
            .map_err(|e| format!("Failed to set permissions: {}", e))?;
    }

    // Verify it works
    let mut cmd = tokio::process::Command::new(&target_path);
    cmd.arg("--version").stdin(Stdio::null());

    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

    let output = match cmd.output().await {
        Ok(output) => output,
        Err(e) => {
            let _ = std::fs::remove_file(&target_path);
            return Err(format!("yt-dlp downloaded but failed to execute: {}", e));
        }
    };

    if !output.status.success() {
        let _ = std::fs::remove_file(&target_path);
        return Err("yt-dlp downloaded but failed version check".to_string());
    }

    Ok(target_path)
}

/// Ensure the local yt-dlp binary is present and up to date.
/// Downloads if missing; re-downloads if older than 7 days.
/// Silently ignores network errors (best-effort background update).
pub async fn ensure_yt_dlp_up_to_date() {
    let local_path = get_yt_dlp_path();

    let needs_download = if local_path.exists() {
        // Check file age via mtime
        match std::fs::metadata(&local_path).and_then(|m| m.modified()) {
            Ok(mtime) => {
                let age = SystemTime::now()
                    .duration_since(mtime)
                    .unwrap_or(Duration::ZERO);
                if age > UPDATE_INTERVAL {
                    tracing::info!(
                        "yt-dlp binary is {} days old, updating",
                        age.as_secs() / 86400
                    );
                    true
                } else {
                    false
                }
            }
            Err(_) => false, // Can't read mtime, skip update
        }
    } else {
        tracing::info!("yt-dlp binary not found, downloading");
        true
    };

    if needs_download {
        match download_yt_dlp().await {
            Ok(path) => tracing::info!("yt-dlp updated successfully: {:?}", path),
            Err(e) => tracing::warn!("Failed to update yt-dlp (will retry next launch): {}", e),
        }
    }
}
