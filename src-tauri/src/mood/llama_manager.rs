use std::path::PathBuf;

use crate::storage::config::get_app_data_dir;

#[cfg(target_os = "linux")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LinuxRuntimeMode {
    Vulkan,
    CpuExplicit,
    Unsupported,
}

/// Get the platform-specific llama-server binary name.
fn llama_server_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "llama-server.exe"
    } else {
        "llama-server"
    }
}

/// Get the directory where llama-server and its DLLs are stored.
fn get_llama_server_dir() -> PathBuf {
    get_app_data_dir().join("bin").join("llama-server")
}

/// Get the path where llama-server binary is stored.
pub fn get_llama_server_path() -> PathBuf {
    get_llama_server_dir().join(llama_server_binary_name())
}

/// Get the directory where GGUF models are stored.
pub fn get_models_dir() -> PathBuf {
    get_app_data_dir().join("models")
}

/// Get the path for the Qwen3-VL 2B LLM model.
pub fn get_model_path() -> PathBuf {
    get_models_dir().join("Qwen3VL-2B-Instruct-Q4_K_M.gguf")
}

/// Get the path for the Qwen3-VL 2B vision encoder (mmproj).
pub fn get_mmproj_path() -> PathBuf {
    get_models_dir().join("mmproj-Qwen3VL-2B-Instruct-F16.gguf")
}

/// Check if the managed llama-server binary exists locally.
pub fn find_llama_server() -> Option<PathBuf> {
    let local_path = get_llama_server_path();
    if local_path.exists() {
        if let Ok(meta) = std::fs::metadata(&local_path) {
            if meta.len() > 1_000_000 {
                return Some(local_path);
            }
            let _ = std::fs::remove_file(&local_path);
        }
    }

    // Check system PATH as fallback
    if let Ok(output) = std::process::Command::new("llama-server")
        .arg("--version")
        .output()
    {
        if output.status.success() {
            return Some(PathBuf::from(llama_server_binary_name()));
        }
    }

    None
}

/// Check if llama-server is installed.
pub fn is_llama_server_installed() -> bool {
    find_llama_server().is_some()
}

/// Check if both model GGUF files are downloaded (LLM + mmproj).
pub fn is_model_downloaded() -> bool {
    let llm_ok = if let Ok(meta) = std::fs::metadata(get_model_path()) {
        meta.len() > 500_000_000 // Q4_K_M ~1.1GB
    } else {
        false
    };

    let mmproj_ok = if let Ok(meta) = std::fs::metadata(get_mmproj_path()) {
        meta.len() > 100_000_000 // mmproj F16 ~780MB
    } else {
        false
    };

    llm_ok && mmproj_ok
}

#[cfg(target_os = "linux")]
fn backend_override() -> Option<String> {
    std::env::var("KEYTOMUSIC_LLAMA_BACKEND")
        .ok()
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty())
}

#[cfg(target_os = "linux")]
fn vulkan_available() -> bool {
    std::process::Command::new("vulkaninfo")
        .arg("--summary")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn resolve_linux_runtime_mode(
    backend_override: Option<&str>,
    has_vulkan: bool,
) -> LinuxRuntimeMode {
    match backend_override {
        Some("cpu") => LinuxRuntimeMode::CpuExplicit,
        Some("vulkan") => {
            if has_vulkan {
                LinuxRuntimeMode::Vulkan
            } else {
                LinuxRuntimeMode::Unsupported
            }
        }
        Some(_) => LinuxRuntimeMode::Unsupported,
        None => {
            if has_vulkan {
                LinuxRuntimeMode::Vulkan
            } else {
                LinuxRuntimeMode::Unsupported
            }
        }
    }
}

/// Search pattern to match the right asset in llama.cpp GitHub releases.
fn asset_search_patterns() -> Result<Vec<&'static str>, String> {
    if cfg!(target_os = "windows") {
        Ok(vec!["win-vulkan-x64"])
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            Ok(vec!["macos-arm64"])
        } else {
            Ok(vec!["macos-x64"])
        }
    } else {
        #[cfg(target_os = "linux")]
        {
            return match resolve_linux_runtime_mode(
                backend_override().as_deref(),
                vulkan_available(),
            ) {
                LinuxRuntimeMode::Vulkan => Ok(vec!["ubuntu-vulkan-x64"]),
                LinuxRuntimeMode::CpuExplicit => Ok(vec!["ubuntu-x64"]),
                LinuxRuntimeMode::Unsupported => Err(
                    "Manga mood requires GPU acceleration on Linux. No supported GPU backend detected. Install/enable Vulkan or set KEYTOMUSIC_LLAMA_BACKEND=cpu for unsupported debug mode."
                        .to_string(),
                ),
            };
        }

        #[cfg(not(target_os = "linux"))]
        {
            Ok(vec!["ubuntu-x64"])
        }
    }
}

#[cfg(target_os = "linux")]
pub fn ensure_mood_runtime_supported() -> Result<(), String> {
    match resolve_linux_runtime_mode(backend_override().as_deref(), vulkan_available()) {
        LinuxRuntimeMode::Vulkan | LinuxRuntimeMode::CpuExplicit => Ok(()),
        LinuxRuntimeMode::Unsupported => Err(
            "Manga mood requires GPU acceleration on Linux. No supported GPU backend detected. Install/enable Vulkan or set KEYTOMUSIC_LLAMA_BACKEND=cpu for unsupported debug mode."
                .to_string(),
        ),
    }
}

#[cfg(not(target_os = "linux"))]
pub fn ensure_mood_runtime_supported() -> Result<(), String> {
    Ok(())
}

/// Fetch the download URL for the latest llama-server release from GitHub API.
async fn fetch_llama_server_url() -> Result<String, String> {
    let api_url = "https://api.github.com/repos/ggml-org/llama.cpp/releases/latest";
    let client = reqwest::Client::builder()
        .user_agent("KeyToMusic")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    tracing::info!("Fetching latest llama.cpp release info...");

    let response: serde_json::Value = client
        .get(api_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch release info: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse release info: {}", e))?;

    let patterns = asset_search_patterns()?;
    tracing::info!(
        "Looking for llama.cpp assets matching patterns: {:?}",
        patterns
    );

    let assets = response["assets"]
        .as_array()
        .ok_or_else(|| "No assets in release".to_string())?;

    for pattern in &patterns {
        for asset in assets {
            let name = asset["name"].as_str().unwrap_or_default();
            if !name.contains(pattern) {
                continue;
            }

            let is_linux = cfg!(not(any(target_os = "windows", target_os = "macos")));
            if is_linux
                && *pattern == "ubuntu-x64"
                && (name.contains("rocm")
                    || name.contains("vulkan")
                    || name.contains("s390x")
                    || name.contains("opencl"))
            {
                continue;
            }

            let url = asset["browser_download_url"]
                .as_str()
                .ok_or_else(|| "No download URL for asset".to_string())?;
            tracing::info!("Found matching asset: {} -> {}", name, url);
            return Ok(url.to_string());
        }
    }

    Err(format!(
        "No llama-server release found for platform patterns {:?}",
        patterns
    ))
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::{resolve_linux_runtime_mode, LinuxRuntimeMode};

    #[test]
    fn linux_runtime_prefers_vulkan_when_available() {
        assert_eq!(
            resolve_linux_runtime_mode(None, true),
            LinuxRuntimeMode::Vulkan
        );
    }

    #[test]
    fn linux_runtime_requires_explicit_cpu_override() {
        assert_eq!(
            resolve_linux_runtime_mode(None, false),
            LinuxRuntimeMode::Unsupported
        );
        assert_eq!(
            resolve_linux_runtime_mode(Some("cpu"), false),
            LinuxRuntimeMode::CpuExplicit
        );
    }
}

/// Download llama-server binary and all its dependencies.
pub async fn download_llama_server() -> Result<PathBuf, String> {
    let target_dir = get_llama_server_dir();
    let target_path = get_llama_server_path();

    // Clean previous install if any
    if target_dir.exists() {
        std::fs::remove_dir_all(&target_dir)
            .map_err(|e| format!("Failed to clean old install: {}", e))?;
    }
    std::fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Failed to create llama-server directory: {}", e))?;

    let url = fetch_llama_server_url().await?;
    tracing::info!("Downloading llama-server from {}", url);

    let response = reqwest::get(&url)
        .await
        .map_err(|e| format!("Failed to download llama-server: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to download llama-server: HTTP {}",
            response.status()
        ));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read download: {}", e))?;

    tracing::info!(
        "Downloaded {} bytes, extracting to {:?}...",
        bytes.len(),
        target_dir
    );

    // Extract all files from archive into the llama-server directory
    if cfg!(target_os = "windows") {
        extract_zip_all(&bytes, &target_dir)?;
    } else {
        extract_tar_gz_all(&bytes, &target_dir)?;
    }

    #[cfg(target_os = "linux")]
    ensure_linux_runtime_links(&target_dir)?;

    if !target_path.exists() {
        return Err("llama-server binary not found after extraction".to_string());
    }

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&target_path, perms)
            .map_err(|e| format!("Failed to set permissions: {}", e))?;
    }

    tracing::info!("llama-server installed at {:?}", target_path);
    Ok(target_path)
}

/// Extract all files from ZIP archive into target directory (Windows).
fn extract_zip_all(data: &[u8], target_dir: &PathBuf) -> Result<(), String> {
    use std::io::Cursor;
    let reader = Cursor::new(data);
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|e| format!("Failed to open ZIP: {}", e))?;

    tracing::info!("Extracting {} files from ZIP...", archive.len());
    let mut extracted = 0;
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read ZIP entry: {}", e))?;
        let name = file.name().to_string();

        // Skip directories
        if name.ends_with('/') {
            continue;
        }

        // Extract just the filename (flatten any subdirectory structure)
        let file_name = std::path::Path::new(&name)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or(name.clone());

        // Only extract .exe and .dll files (skip docs, etc.)
        if !file_name.ends_with(".exe") && !file_name.ends_with(".dll") {
            continue;
        }

        let out_path = target_dir.join(&file_name);
        tracing::debug!("Extracting: {} -> {:?}", name, out_path);
        let mut out = std::fs::File::create(&out_path)
            .map_err(|e| format!("Failed to create {}: {}", file_name, e))?;
        std::io::copy(&mut file, &mut out)
            .map_err(|e| format!("Failed to extract {}: {}", file_name, e))?;
        extracted += 1;
    }

    tracing::info!("Extracted {} files", extracted);
    Ok(())
}

/// Extract all files from tar.gz archive into target directory (macOS/Linux).
#[cfg(not(target_os = "windows"))]
fn extract_tar_gz_all(data: &[u8], target_dir: &PathBuf) -> Result<(), String> {
    use std::io::Cursor;
    let decoder = flate2::read::GzDecoder::new(Cursor::new(data));
    let mut archive = tar::Archive::new(decoder);

    let mut extracted = 0;
    for entry in archive
        .entries()
        .map_err(|e| format!("Failed to read tar: {}", e))?
    {
        let mut entry = entry.map_err(|e| format!("Failed to read tar entry: {}", e))?;
        let path = entry
            .path()
            .map_err(|e| format!("Failed to get entry path: {}", e))?;

        let file_name = match path.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => continue,
        };

        // Skip directories and non-binary files
        if entry.header().entry_type().is_dir() {
            continue;
        }

        let out_path = target_dir.join(&file_name);
        tracing::debug!("Extracting: {:?} -> {:?}", path, out_path);
        let mut out = std::fs::File::create(&out_path)
            .map_err(|e| format!("Failed to create {}: {}", file_name, e))?;
        std::io::copy(&mut entry, &mut out)
            .map_err(|e| format!("Failed to extract {}: {}", file_name, e))?;
        extracted += 1;
    }

    tracing::info!("Extracted {} files", extracted);
    Ok(())
}

/// Stub for Windows (tar.gz not used).
#[cfg(target_os = "windows")]
fn extract_tar_gz_all(_data: &[u8], _target_dir: &PathBuf) -> Result<(), String> {
    Err("tar.gz extraction not supported on Windows".to_string())
}

#[cfg(target_os = "linux")]
fn ensure_linux_runtime_links(target_dir: &PathBuf) -> Result<(), String> {
    use std::os::unix::fs::symlink;

    let required_links = [
        ("libmtmd.so.0", "libmtmd.so.0."),
        ("libllama.so.0", "libllama.so.0."),
        ("libggml.so.0", "libggml.so.0."),
        ("libggml-base.so.0", "libggml-base.so.0."),
    ];

    for (link_name, versioned_prefix) in required_links {
        let link_path = target_dir.join(link_name);

        let existing_ok = std::fs::symlink_metadata(&link_path)
            .map(|meta| meta.file_type().is_symlink() || meta.len() > 1_000_000)
            .unwrap_or(false);
        if existing_ok {
            continue;
        }

        if link_path.exists() {
            std::fs::remove_file(&link_path)
                .map_err(|e| format!("Failed to remove invalid {}: {}", link_name, e))?;
        }

        let mut candidates: Vec<String> = std::fs::read_dir(target_dir)
            .map_err(|e| format!("Failed to scan {:?}: {}", target_dir, e))?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| name.starts_with(versioned_prefix))
            .collect();
        candidates.sort();

        let target_name = candidates
            .last()
            .cloned()
            .ok_or_else(|| format!("Missing shared library matching {}", versioned_prefix))?;

        symlink(&target_name, &link_path).map_err(|e| {
            format!(
                "Failed to create symlink {} -> {}: {}",
                link_name, target_name, e
            )
        })?;
    }

    Ok(())
}

/// Download a single file with streaming progress.
async fn download_file_with_progress(
    url: &str,
    target_path: &PathBuf,
    base_downloaded: u64,
    base_total: u64,
    progress_cb: &(dyn Fn(u64, u64) + Send + Sync),
) -> Result<u64, String> {
    tracing::info!("Downloading {} -> {:?}", url, target_path);

    let response: reqwest::Response = reqwest::get(url)
        .await
        .map_err(|e| format!("Failed to download: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }

    let file_size = response.content_length().unwrap_or(0);
    let total = base_total.max(base_downloaded + file_size);
    let tmp_path = target_path.with_extension("tmp");

    let mut file = std::fs::File::create(&tmp_path)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;

    use futures::StreamExt;
    let mut stream = response.bytes_stream();
    let mut file_downloaded: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download error: {}", e))?;
        use std::io::Write;
        file.write_all(&chunk)
            .map_err(|e| format!("Failed to write: {}", e))?;
        file_downloaded += chunk.len() as u64;
        progress_cb(base_downloaded + file_downloaded, total);
    }

    drop(file);

    // Atomic rename
    let _ = std::fs::remove_file(target_path);
    std::fs::rename(&tmp_path, target_path).map_err(|e| format!("Failed to rename file: {}", e))?;

    tracing::info!("Downloaded {:?} ({} bytes)", target_path, file_downloaded);
    Ok(file_downloaded)
}

/// Download both model GGUF files (LLM + mmproj vision encoder) with progress.
pub async fn download_model<F>(progress_cb: F) -> Result<PathBuf, String>
where
    F: Fn(u64, u64) + Send + Sync + 'static,
{
    let model_path = get_model_path();
    let mmproj_path = get_mmproj_path();

    if let Some(parent) = model_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create models directory: {}", e))?;
    }

    // Estimated total: LLM Q4_K_M ~1.1GB + mmproj F16 ~780MB = ~1.88GB
    let estimated_total: u64 = 1_100_000_000 + 780_000_000;

    // Download LLM model
    let llm_url = "https://huggingface.co/Qwen/Qwen3-VL-2B-Instruct-GGUF/resolve/main/Qwen3VL-2B-Instruct-Q4_K_M.gguf";
    tracing::info!("Downloading LLM model from {}", llm_url);
    let llm_size =
        download_file_with_progress(llm_url, &model_path, 0, estimated_total, &progress_cb)
            .await
            .map_err(|e| format!("Failed to download model: {}", e))?;

    // Download mmproj vision encoder
    let mmproj_url = "https://huggingface.co/Qwen/Qwen3-VL-2B-Instruct-GGUF/resolve/main/mmproj-Qwen3VL-2B-Instruct-F16.gguf";
    tracing::info!("Downloading mmproj vision encoder from {}", mmproj_url);
    download_file_with_progress(
        mmproj_url,
        &mmproj_path,
        llm_size,
        estimated_total,
        &progress_cb,
    )
    .await
    .map_err(|e| format!("Failed to download vision encoder: {}", e))?;

    tracing::info!("All model files downloaded successfully");
    Ok(model_path)
}
