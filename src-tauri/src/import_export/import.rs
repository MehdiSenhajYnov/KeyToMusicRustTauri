use crate::audio::analysis::WaveformData;
use crate::storage;
use crate::types::{Profile, SoundSource};
use super::{ExportMetadata, ImportResult};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

/// Import a .ktm file, returning the new profile ID and any embedded waveform data.
pub fn import_profile(ktm_path: &str) -> Result<ImportResult, String> {
    let ktm_file = Path::new(ktm_path);
    if !ktm_file.exists() {
        return Err(format!("File not found: {}", ktm_path));
    }

    // Open and read the ZIP
    let file = fs::File::open(ktm_file)
        .map_err(|e| format!("Failed to open .ktm file: {}", e))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("Invalid .ktm file (not a valid ZIP): {}", e))?;

    // Read and parse metadata.json (optional - for version checking)
    if let Ok(mut entry) = archive.by_name("metadata.json") {
        let mut contents = String::new();
        entry.read_to_string(&mut contents)
            .map_err(|e| format!("Failed to read metadata.json: {}", e))?;
        let _metadata: ExportMetadata = serde_json::from_str(&contents)
            .map_err(|e| format!("Failed to parse metadata.json: {}", e))?;
        // Version compatibility could be checked here in the future
    }

    // Read and parse profile.json (required)
    let mut profile: Profile = {
        let mut entry = archive.by_name("profile.json")
            .map_err(|_| "Invalid .ktm file: missing profile.json".to_string())?;
        let mut contents = String::new();
        entry.read_to_string(&mut contents)
            .map_err(|e| format!("Failed to read profile.json: {}", e))?;
        serde_json::from_str(&contents)
            .map_err(|e| format!("Failed to parse profile.json: {}", e))?
    };

    // Generate new IDs
    let new_profile_id = uuid::Uuid::new_v4().to_string();
    let old_to_new_sound_ids: HashMap<String, String> = profile
        .sounds
        .iter()
        .map(|s| (s.id.clone(), uuid::Uuid::new_v4().to_string()))
        .collect();

    // Update profile ID and name
    profile.id = new_profile_id.clone();
    profile.name = format!("{} (Imported)", profile.name);

    // Update timestamps
    let now = chrono::Utc::now().to_rfc3339();
    profile.created_at = now.clone();
    profile.updated_at = now;

    // Create the destination directory for imported sounds
    let app_dir = storage::get_app_data_dir();
    let sounds_dir = app_dir.join("imported_sounds").join(&new_profile_id);
    fs::create_dir_all(&sounds_dir)
        .map_err(|e| format!("Failed to create imported sounds directory: {}", e))?;

    // Map from ZIP-relative path ("sounds/filename.mp3") to new absolute path
    let mut zip_to_abs: HashMap<String, String> = HashMap::new();

    // Extract sound files and update paths
    for sound in &mut profile.sounds {
        // Update sound ID
        let old_id = sound.id.clone();
        sound.id = old_to_new_sound_ids
            .get(&old_id)
            .cloned()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        // Extract the sound file from the ZIP
        let relative_path = match &sound.source {
            SoundSource::Local { path } => path.clone(),
            SoundSource::YouTube { cached_path, .. } => cached_path.clone(),
        };

        // The path in the ZIP is relative (e.g., "sounds/filename.mp3")
        let zip_entry_path = if relative_path.starts_with("sounds/") {
            relative_path.clone()
        } else {
            format!("sounds/{}", relative_path)
        };

        let filename = Path::new(&relative_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let dest_path = sounds_dir.join(&filename);

        // Find the correct entry name in the ZIP
        let available_names: Vec<String> = archive.file_names().map(|s| s.to_string()).collect();

        let entry_name = if available_names.contains(&zip_entry_path) {
            zip_entry_path.clone()
        } else if available_names.contains(&filename) {
            filename.clone()
        } else {
            return Err(format!(
                "Sound file '{}' not found in .ktm archive (sound: '{}')",
                zip_entry_path, sound.name
            ));
        };

        let mut entry = archive.by_name(&entry_name)
            .map_err(|e| format!("Failed to read '{}' from archive: {}", entry_name, e))?;
        let mut dest_file = fs::File::create(&dest_path)
            .map_err(|e| format!("Failed to create '{}': {}", dest_path.display(), e))?;
        std::io::copy(&mut entry, &mut dest_file)
            .map_err(|e| format!("Failed to extract '{}': {}", entry_name, e))?;

        // Update the source to use the absolute path
        let abs_path = dest_path.to_string_lossy().to_string();
        zip_to_abs.insert(zip_entry_path, abs_path.clone());
        sound.source = SoundSource::Local { path: abs_path };
    }

    // Update sound IDs in key bindings
    for binding in &mut profile.key_bindings {
        binding.sound_ids = binding
            .sound_ids
            .iter()
            .map(|old_id| {
                old_to_new_sound_ids
                    .get(old_id)
                    .cloned()
                    .unwrap_or_else(|| old_id.clone())
            })
            .collect();
    }

    // Save the new profile
    storage::save_profile(&profile)?;

    // Read waveforms.json (optional — old exports won't have it)
    let waveforms = if let Ok(mut entry) = archive.by_name("waveforms.json") {
        let mut contents = String::new();
        if entry.read_to_string(&mut contents).is_ok() {
            if let Ok(zip_waveforms) = serde_json::from_str::<HashMap<String, WaveformData>>(&contents) {
                // Remap keys from ZIP-relative paths to new absolute paths
                zip_waveforms
                    .into_iter()
                    .filter_map(|(zip_key, data)| {
                        zip_to_abs.get(&zip_key).map(|abs| (abs.clone(), data))
                    })
                    .collect()
            } else {
                HashMap::new()
            }
        } else {
            HashMap::new()
        }
    } else {
        HashMap::new()
    };

    Ok(ImportResult {
        profile_id: new_profile_id,
        waveforms,
    })
}
