use std::path::PathBuf;
use std::process::Stdio;

use crate::storage::config::get_app_data_dir;

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

/// Check if yt-dlp is available (either in app data or in PATH).
/// Returns the path to use for executing yt-dlp.
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

    // Check if it's in PATH
    if which_yt_dlp().is_some() {
        return Some(PathBuf::from("yt-dlp"));
    }

    None
}

/// Check if yt-dlp exists in system PATH.
fn which_yt_dlp() -> Option<PathBuf> {
    let name = if cfg!(target_os = "windows") {
        "yt-dlp.exe"
    } else {
        "yt-dlp"
    };

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

/// Check if yt-dlp is installed (locally or in PATH).
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
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

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
