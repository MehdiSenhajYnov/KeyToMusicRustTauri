use crate::storage;
use crate::types::{Profile, SoundSource};
use super::ExportMetadata;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

/// Progress callback: (current_file_index, total_files, filename)
pub type ProgressCallback = Box<dyn Fn(usize, usize, &str) + Send>;

static EXPORT_CANCELLED: AtomicBool = AtomicBool::new(false);

/// Request cancellation of the current export.
pub fn cancel_export() {
    EXPORT_CANCELLED.store(true, Ordering::Relaxed);
}

/// Path to the file that tracks an in-progress export temp file.
fn export_tracking_path() -> PathBuf {
    storage::get_app_data_dir().join("export_in_progress.txt")
}

/// Clean up any orphaned temp file from a previously interrupted export.
pub fn cleanup_interrupted_export() {
    let tracking = export_tracking_path();
    if tracking.exists() {
        if let Ok(temp_path) = fs::read_to_string(&tracking) {
            let temp = Path::new(temp_path.trim());
            if temp.exists() {
                fs::remove_file(temp).ok();
            }
        }
        fs::remove_file(&tracking).ok();
    }
}

/// Export a profile as a .ktm file (ZIP archive containing profile.json, metadata.json, and sounds/).
/// Writes to a temp file first, then renames to the final path on success.
pub fn export_profile(
    profile_id: &str,
    output_path: &str,
    on_progress: Option<ProgressCallback>,
) -> Result<(), String> {
    // Reset cancellation flag
    EXPORT_CANCELLED.store(false, Ordering::Relaxed);

    // Load the profile
    let profile = storage::load_profile(profile_id.to_string())?;

    // Collect all sound files and build the modified profile with relative paths
    let (modified_profile, sound_files) = collect_sound_files(&profile)?;

    let total_files = sound_files.len();

    // Serialize profile and metadata
    let profile_json = serde_json::to_string_pretty(&modified_profile)
        .map_err(|e| format!("Failed to serialize profile: {}", e))?;

    let metadata = ExportMetadata {
        version: "1.0.0".to_string(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        platform: std::env::consts::OS.to_string(),
    };
    let metadata_json = serde_json::to_string_pretty(&metadata)
        .map_err(|e| format!("Failed to serialize metadata: {}", e))?;

    // Write to a temp file first (same dir as output so rename works)
    let final_path = Path::new(output_path);
    let temp_path = final_path.with_extension("ktm.tmp");

    // Track the temp file so it can be cleaned up if the app is killed
    fs::write(export_tracking_path(), temp_path.to_string_lossy().as_bytes()).ok();

    let zip_file = fs::File::create(&temp_path)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;

    let mut zip = ZipWriter::new(zip_file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // Write profile.json
    zip.start_file("profile.json", options)
        .map_err(|e| format!("Failed to write profile.json to ZIP: {}", e))?;
    zip.write_all(profile_json.as_bytes())
        .map_err(|e| format!("Failed to write profile data: {}", e))?;

    // Write metadata.json
    zip.start_file("metadata.json", options)
        .map_err(|e| format!("Failed to write metadata.json to ZIP: {}", e))?;
    zip.write_all(metadata_json.as_bytes())
        .map_err(|e| format!("Failed to write metadata data: {}", e))?;

    // Write sound files with progress
    for (i, (relative_path, absolute_path)) in sound_files.iter().enumerate() {
        // Check for cancellation
        if EXPORT_CANCELLED.load(Ordering::Relaxed) {
            drop(zip);
            fs::remove_file(&temp_path).ok();
            fs::remove_file(export_tracking_path()).ok();
            return Err("Export cancelled".to_string());
        }

        if let Some(ref cb) = on_progress {
            cb(i, total_files, relative_path);
        }

        let file_data = fs::read(absolute_path)
            .map_err(|e| format!("Failed to read sound file '{}': {}", absolute_path.display(), e))?;

        let zip_entry_path = format!("sounds/{}", relative_path);
        zip.start_file(&zip_entry_path, options)
            .map_err(|e| format!("Failed to add '{}' to ZIP: {}", zip_entry_path, e))?;
        zip.write_all(&file_data)
            .map_err(|e| format!("Failed to write sound data: {}", e))?;
    }

    zip.finish()
        .map_err(|e| format!("Failed to finalize ZIP: {}", e))?;

    // Rename temp file to final path
    if final_path.exists() {
        fs::remove_file(final_path).ok();
    }
    fs::rename(&temp_path, final_path)
        .map_err(|e| format!("Failed to finalize export file: {}", e))?;

    // Remove the tracking file now that export succeeded
    fs::remove_file(export_tracking_path()).ok();

    // Signal completion
    if let Some(ref cb) = on_progress {
        cb(total_files, total_files, "");
    }

    Ok(())
}

/// Collect sound files from a profile, returning a modified profile with relative paths
/// and a list of (relative_filename, absolute_path) pairs (deduplicated by source path).
fn collect_sound_files(profile: &Profile) -> Result<(Profile, Vec<(String, PathBuf)>), String> {
    let mut modified_profile = profile.clone();
    let mut sound_files: Vec<(String, PathBuf)> = Vec::new();
    let mut used_filenames: std::collections::HashSet<String> = std::collections::HashSet::new();
    // Map from absolute source path -> relative filename in ZIP (for deduplication)
    let mut path_to_zip_name: std::collections::HashMap<PathBuf, String> =
        std::collections::HashMap::new();

    for sound in &mut modified_profile.sounds {
        let source_path = match &sound.source {
            SoundSource::Local { path } => PathBuf::from(path),
            SoundSource::YouTube { cached_path, .. } => PathBuf::from(cached_path),
        };

        if !source_path.exists() {
            return Err(format!(
                "Sound file not found: '{}' (sound: '{}')",
                source_path.display(),
                sound.name
            ));
        }

        // Check if we've already processed this source file
        let zip_filename = if let Some(existing) = path_to_zip_name.get(&source_path) {
            existing.clone()
        } else {
            // New file - generate a unique filename for the ZIP
            let original_filename = source_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            let unique_filename = make_unique_filename(&original_filename, &used_filenames);
            used_filenames.insert(unique_filename.clone());
            sound_files.push((unique_filename.clone(), source_path.clone()));
            path_to_zip_name.insert(source_path, unique_filename.clone());
            unique_filename
        };

        // Update the source to use relative path
        sound.source = SoundSource::Local {
            path: format!("sounds/{}", zip_filename),
        };
    }

    Ok((modified_profile, sound_files))
}

/// Generate a unique filename by appending a counter if needed.
fn make_unique_filename(
    filename: &str,
    used: &std::collections::HashSet<String>,
) -> String {
    if !used.contains(filename) {
        return filename.to_string();
    }

    let path = Path::new(filename);
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();

    let mut counter = 2;
    loop {
        let candidate = format!("{} ({}){}", stem, counter, ext);
        if !used.contains(&candidate) {
            return candidate;
        }
        counter += 1;
    }
}
