use super::ExportMetadata;
use crate::audio::analysis::WaveformData;
use crate::storage;
use crate::types::{Profile, SoundSource};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

/// Progress callback: (current_file_index, total_files, filename)
pub type ProgressCallback = Box<dyn Fn(usize, usize, &str) + Send>;

static EXPORT_LOCK: Mutex<()> = Mutex::new(());

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

struct ExportTracking(PathBuf);
impl Drop for ExportTracking {
    fn drop(&mut self) {
        fs::remove_file(&self.0).ok();
    }
}

/// Export a profile as a .ktm file (ZIP archive containing profile.json, metadata.json, sounds/, and waveforms.json).
/// Writes to a temp file first, then renames to the final path on success.
/// If `waveforms` is provided, matching entries (keyed by absolute path) are included as waveforms.json.
pub fn export_profile(
    profile_id: &str,
    output_path: &str,
    waveforms: Option<HashMap<String, WaveformData>>,
    on_progress: Option<ProgressCallback>,
) -> Result<(), String> {
    let _export_guard = EXPORT_LOCK
        .try_lock()
        .map_err(|_| "An export is already in progress".to_string())?;
    // Reset cancellation flag
    EXPORT_CANCELLED.store(false, Ordering::Relaxed);

    // Load the profile
    let profile = storage::load_profile(profile_id.to_string())?;

    export_loaded_profile(
        &profile,
        output_path,
        waveforms,
        on_progress,
        export_tracking_path(),
    )
}

fn export_loaded_profile(
    profile: &Profile,
    output_path: &str,
    waveforms: Option<HashMap<String, WaveformData>>,
    on_progress: Option<ProgressCallback>,
    tracking_path: PathBuf,
) -> Result<(), String> {
    // Collect all sound files and build the modified profile with relative paths
    let (modified_profile, sound_files) = collect_sound_files(profile)?;

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
    let parent = final_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let temp = tempfile::NamedTempFile::new_in(parent)
        .map_err(|e| format!("Failed to create export: {}", e))?;
    fs::write(&tracking_path, temp.path().to_string_lossy().as_bytes())
        .map_err(|e| e.to_string())?;
    let _tracking = ExportTracking(tracking_path);
    let mut zip = ZipWriter::new(temp);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    // Audio is already compressed: deflating it costs CPU for no size gain.
    let audio_options = options.compression_method(zip::CompressionMethod::Stored);

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
            return Err("Export cancelled".to_string());
        }

        if let Some(ref cb) = on_progress {
            cb(i, total_files, relative_path);
        }

        let mut source = fs::File::open(absolute_path)
            .map_err(|e| format!("Failed to read '{}': {}", absolute_path.display(), e))?;
        let zip_entry_path = format!("sounds/{}", relative_path);
        zip.start_file(&zip_entry_path, audio_options)
            .map_err(|e| format!("Failed to add '{}' to ZIP: {}", zip_entry_path, e))?;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            if EXPORT_CANCELLED.load(Ordering::Relaxed) {
                return Err("Export cancelled".into());
            }
            let count = source.read(&mut buffer).map_err(|e| e.to_string())?;
            if count == 0 {
                break;
            }
            zip.write_all(&buffer[..count]).map_err(|e| e.to_string())?;
        }
    }

    // Write waveforms.json (keyed by ZIP-relative paths like "sounds/filename.mp3")
    if let Some(waveform_map) = waveforms {
        let mut zip_waveforms: HashMap<String, WaveformData> = HashMap::new();
        for (relative_name, absolute_path) in &sound_files {
            let abs_str = absolute_path.to_string_lossy().to_string();
            if let Some(data) = waveform_map.get(&abs_str) {
                zip_waveforms.insert(format!("sounds/{}", relative_name), data.clone());
            }
        }
        if !zip_waveforms.is_empty() {
            let waveforms_json = serde_json::to_string(&zip_waveforms)
                .map_err(|e| format!("Failed to serialize waveforms: {}", e))?;
            zip.start_file("waveforms.json", options)
                .map_err(|e| format!("Failed to write waveforms.json to ZIP: {}", e))?;
            zip.write_all(waveforms_json.as_bytes())
                .map_err(|e| format!("Failed to write waveform data: {}", e))?;
        }
    }

    let temp = zip
        .finish()
        .map_err(|e| format!("Failed to finalize ZIP: {}", e))?;
    if EXPORT_CANCELLED.load(Ordering::Relaxed) {
        return Err("Export cancelled".into());
    }
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    // Atomic replacement: the old export stays intact if anything above fails.
    temp.persist(final_path)
        .map_err(|e| format!("Failed to publish export: {}", e))?;

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
fn make_unique_filename(filename: &str, used: &std::collections::HashSet<String>) -> String {
    if !used.contains(filename) {
        return filename.to_string();
    }

    let path = Path::new(filename);
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();

    let mut counter = 2;
    loop {
        let candidate = format!("{} ({}){}", stem, counter, ext);
        if !used.contains(&candidate) {
            return candidate;
        }
        counter += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn profile(path: &Path) -> Profile {
        serde_json::from_value(serde_json::json!({
            "id":"p", "name":"P", "createdAt":"", "updatedAt":"", "tracks":[], "keyBindings":[],
            "sounds":[{"id":"s", "name":"S", "source":{"type":"local", "path":path}, "momentum":0, "volume":1, "duration":1}]
        })).unwrap()
    }
    #[test]
    fn export_failure_preserves_previous_archive_and_removes_temp_files() {
        let _guard = EXPORT_LOCK.lock().unwrap();
        EXPORT_CANCELLED.store(false, Ordering::Relaxed);
        let dir = tempfile::tempdir().unwrap();
        let sound = dir.path().join("sound.wav");
        fs::write(&sound, b"audio").unwrap();
        let profile = profile(&sound);
        let output = dir.path().join("library.ktm");
        fs::write(&output, b"previous archive").unwrap();
        let result = export_loaded_profile(
            &profile,
            output.to_str().unwrap(),
            None,
            Some(Box::new(move |_, _, _| {
                fs::remove_file(&sound).unwrap();
            })),
            dir.path().join("tracking"),
        );
        assert!(result.is_err());
        assert_eq!(fs::read(&output).unwrap(), b"previous archive");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }
    #[test]
    fn cancellation_preserves_previous_archive() {
        let _guard = EXPORT_LOCK.lock().unwrap();
        EXPORT_CANCELLED.store(false, Ordering::Relaxed);
        let dir = tempfile::tempdir().unwrap();
        let sound = dir.path().join("sound.wav");
        fs::write(&sound, b"audio").unwrap();
        let output = dir.path().join("library.ktm");
        fs::write(&output, b"previous archive").unwrap();
        let result = export_loaded_profile(
            &profile(&sound),
            output.to_str().unwrap(),
            None,
            Some(Box::new(|_, _, _| cancel_export())),
            dir.path().join("tracking"),
        );
        assert_eq!(result.unwrap_err(), "Export cancelled");
        assert_eq!(fs::read(&output).unwrap(), b"previous archive");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
        EXPORT_CANCELLED.store(false, Ordering::Relaxed);
    }
    #[test]
    fn completed_archive_contains_sound_bytes_and_a_portable_profile() {
        let _guard = EXPORT_LOCK.lock().unwrap();
        EXPORT_CANCELLED.store(false, Ordering::Relaxed);
        let dir = tempfile::tempdir().unwrap();
        let sound = dir.path().join("sound.wav");
        fs::write(&sound, b"audio bytes").unwrap();
        let output = dir.path().join("library.ktm");
        export_loaded_profile(
            &profile(&sound),
            output.to_str().unwrap(),
            None,
            None,
            dir.path().join("tracking"),
        )
        .unwrap();
        let mut archive = zip::ZipArchive::new(fs::File::open(&output).unwrap()).unwrap();
        let mut bytes = Vec::new();
        archive
            .by_name("sounds/sound.wav")
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        assert_eq!(bytes, b"audio bytes");
        let mut json = String::new();
        archive
            .by_name("profile.json")
            .unwrap()
            .read_to_string(&mut json)
            .unwrap();
        assert!(json.contains("sounds/sound.wav"));
        assert!(!json.contains(dir.path().to_str().unwrap()));
        assert!(!dir.path().join("tracking").exists());
    }
}
