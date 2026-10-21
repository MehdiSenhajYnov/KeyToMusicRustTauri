use super::{ExportMetadata, ImportResult};
use crate::audio::analysis::WaveformData;
use crate::storage;
use crate::types::{Profile, SoundSource};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Seek};
use std::path::Path;
use zip::ZipArchive;

const MAX_JSON_BYTES: u64 = 32 * 1024 * 1024;
const MAX_AUDIO_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 16 * 1024 * 1024 * 1024;

fn read_json<R: Read + Seek>(archive: &mut ZipArchive<R>, name: &str) -> Result<String, String> {
    let entry = archive
        .by_name(name)
        .map_err(|e| format!("Missing {name}: {e}"))?;
    if entry.size() > MAX_JSON_BYTES {
        return Err(format!("{name} is too large"));
    }
    let mut contents = String::new();
    entry
        .take(MAX_JSON_BYTES + 1)
        .read_to_string(&mut contents)
        .map_err(|e| e.to_string())?;
    if contents.len() as u64 > MAX_JSON_BYTES {
        return Err(format!("{name} is too large"));
    }
    Ok(contents)
}

/// Validate references before extracting anything. Unassigned sounds are valid library data.
fn validate_profile(profile: &Profile) -> Result<(), String> {
    if profile.name.trim().is_empty() {
        return Err("Profile name cannot be empty".into());
    }
    if profile.tracks.len() > 20 {
        return Err("Maximum 20 tracks allowed".into());
    }
    let sounds: HashSet<_> = profile.sounds.iter().map(|s| &s.id).collect();
    let tracks: HashSet<_> = profile.tracks.iter().map(|t| &t.id).collect();
    if sounds.len() != profile.sounds.len() || tracks.len() != profile.tracks.len() {
        return Err("Duplicate sound or track identifiers".into());
    }
    for sound in &profile.sounds {
        if sound.id.is_empty()
            || !sound.duration.is_finite()
            || sound.duration < 0.0
            || !sound.momentum.is_finite()
            || sound.momentum < 0.0
            || !sound.volume.is_finite()
            || !(0.0..=1.0).contains(&sound.volume)
        {
            return Err(format!("Invalid audio settings for '{}'", sound.name));
        }
    }
    for track in &profile.tracks {
        if track.id.is_empty() || !track.volume.is_finite() || !(0.0..=1.0).contains(&track.volume)
        {
            return Err(format!("Invalid track '{}'", track.name));
        }
    }
    let mut bindings = HashSet::new();
    for binding in &profile.key_bindings {
        if binding.key_code.is_empty()
            || !tracks.contains(&binding.track_id)
            || binding.sound_ids.iter().any(|id| !sounds.contains(id))
            || !bindings.insert((&binding.key_code, &binding.track_id))
        {
            return Err(format!("Invalid binding '{}'", binding.key_code));
        }
    }
    Ok(())
}

pub fn import_profile(ktm_path: &str) -> Result<ImportResult, String> {
    import_into(Path::new(ktm_path), &storage::get_app_data_dir())
}

fn import_into(ktm_path: &Path, app_dir: &Path) -> Result<ImportResult, String> {
    let file = fs::File::open(ktm_path).map_err(|e| format!("Failed to open .ktm file: {e}"))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("Invalid .ktm archive: {e}"))?;
    if archive.file_names().any(|name| name == "metadata.json") {
        let metadata: ExportMetadata =
            serde_json::from_str(&read_json(&mut archive, "metadata.json")?)
                .map_err(|e| format!("Invalid metadata.json: {e}"))?;
        if metadata.version.split('.').next() != Some("1") {
            return Err(format!("Unsupported archive version: {}", metadata.version));
        }
    }
    let mut profile: Profile = serde_json::from_str(&read_json(&mut archive, "profile.json")?)
        .map_err(|e| format!("Invalid profile.json: {e}"))?;
    validate_profile(&profile)?;

    let new_id = uuid::Uuid::new_v4().to_string();
    let sound_ids: HashMap<_, _> = profile
        .sounds
        .iter()
        .map(|s| (s.id.clone(), uuid::Uuid::new_v4().to_string()))
        .collect();
    let track_ids: HashMap<_, _> = profile
        .tracks
        .iter()
        .map(|t| (t.id.clone(), uuid::Uuid::new_v4().to_string()))
        .collect();
    profile.id = new_id.clone();
    profile.name = format!("{} (Imported)", profile.name);
    profile.created_at = chrono::Utc::now().to_rfc3339();
    profile.updated_at = profile.created_at.clone();

    let imports = app_dir.join("imported_sounds");
    fs::create_dir_all(&imports).map_err(|e| e.to_string())?;
    // TempDir removes partial extraction automatically on every error path.
    let staging = tempfile::tempdir_in(&imports).map_err(|e| e.to_string())?;
    let final_dir = imports.join(&new_id);
    let mut extracted: HashMap<String, String> = HashMap::new();
    let mut zip_to_abs = HashMap::new();
    let names: HashSet<String> = archive.file_names().map(str::to_owned).collect();
    let mut total_bytes = 0;
    for sound in &mut profile.sounds {
        sound.id = sound_ids[&sound.id].clone();
        let relative_path = match &sound.source {
            SoundSource::Local { path } => path.clone(),
            SoundSource::YouTube { cached_path, .. } => cached_path.clone(),
        };
        // Treat archive paths as identifiers only, never as destination paths.
        let normalized = relative_path.replace('\\', "/");
        let filename = normalized.rsplit('/').next().unwrap_or_default();
        let zip_path = if normalized.starts_with("sounds/") {
            normalized.clone()
        } else {
            format!("sounds/{normalized}")
        };
        let entry_name = if names.contains(&zip_path) {
            zip_path.clone()
        } else if names.contains(filename) {
            filename.to_string()
        } else {
            return Err(format!(
                "Sound file missing from archive: '{}' ({})",
                relative_path, sound.name
            ));
        };
        let abs_path = if let Some(path) = extracted.get(&entry_name) {
            path.clone()
        } else {
            let mut entry = archive.by_name(&entry_name).map_err(|e| e.to_string())?;
            if entry.is_dir()
                || entry.size() > MAX_AUDIO_BYTES
                || total_bytes + entry.size() > MAX_TOTAL_BYTES
            {
                return Err(format!("Invalid or oversized sound: {entry_name}"));
            }
            let extension = Path::new(filename)
                .extension()
                .and_then(|s| s.to_str())
                .filter(|s| s.len() <= 10 && s.bytes().all(|c| c.is_ascii_alphanumeric()))
                .unwrap_or("audio");
            let safe_name = format!("{}.{}", uuid::Uuid::new_v4(), extension);
            let mut destination =
                fs::File::create(staging.path().join(&safe_name)).map_err(|e| e.to_string())?;
            let copied = std::io::copy(
                &mut (&mut entry).take(MAX_AUDIO_BYTES + 1),
                &mut destination,
            )
            .map_err(|e| e.to_string())?;
            total_bytes += copied;
            if copied > MAX_AUDIO_BYTES || total_bytes > MAX_TOTAL_BYTES {
                return Err("Archive is too large".into());
            }
            let path = final_dir.join(safe_name).to_string_lossy().into_owned();
            extracted.insert(entry_name.clone(), path.clone());
            path
        };
        zip_to_abs.insert(zip_path, abs_path.clone());
        zip_to_abs.insert(entry_name, abs_path.clone());
        sound.source = SoundSource::Local { path: abs_path };
    }
    for track in &mut profile.tracks {
        track.id = track_ids[&track.id].clone();
    }
    for binding in &mut profile.key_bindings {
        binding.track_id = track_ids[&binding.track_id].clone();
        binding.sound_ids = binding
            .sound_ids
            .iter()
            .map(|id| sound_ids[id].clone())
            .collect();
        binding.current_index = 0;
    }
    let waveforms = read_json(&mut archive, "waveforms.json")
        .ok()
        .and_then(|json| serde_json::from_str::<HashMap<String, WaveformData>>(&json).ok())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(key, data)| zip_to_abs.get(&key).map(|path| (path.clone(), data)))
        .collect();

    let profile_path = app_dir.join("profiles").join(format!("{new_id}.json"));
    let json = serde_json::to_vec_pretty(&profile).map_err(|e| e.to_string())?;
    fs::rename(staging.path(), &final_dir).map_err(|e| e.to_string())?;
    if let Err(error) = storage::atomic_write(&profile_path, &json) {
        fs::remove_dir_all(&final_dir).ok();
        return Err(error);
    }
    Ok(ImportResult {
        profile_id: new_id,
        waveforms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;

    fn archive(dir: &Path, paths: &[&str], files: &[(&str, &[u8])]) -> std::path::PathBuf {
        let profile = json!({
            "id":"old", "name":"Library", "createdAt":"", "updatedAt":"",
            "tracks":[{"id":"track", "name":"OST", "volume":0.25}],
            "sounds": paths.iter().enumerate().map(|(i,path)| json!({"id":format!("s{i}"),"name":format!("Sound {i}"),"source":{"type":"local","path":path},"duration":10,"momentum":0,"volume":1})).collect::<Vec<_>>(),
            "keyBindings":[{"keyCode":"KeyA","trackId":"track","soundIds":(0..paths.len()).map(|i|format!("s{i}")).collect::<Vec<_>>(),"loopMode":"sequential","currentIndex":999}]
        });
        let path = dir.join("test.ktm");
        let mut zip = zip::ZipWriter::new(fs::File::create(&path).unwrap());
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("profile.json", options).unwrap();
        zip.write_all(profile.to_string().as_bytes()).unwrap();
        for (name, data) in files {
            zip.start_file(*name, options).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
        path
    }

    fn load_import(app: &Path, result: &ImportResult) -> Profile {
        serde_json::from_slice(
            &fs::read(
                app.join("profiles")
                    .join(format!("{}.json", result.profile_id)),
            )
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn same_basename_files_remain_distinct_and_references_are_remapped() {
        let dir = tempfile::tempdir().unwrap();
        let path = archive(
            dir.path(),
            &["sounds/a/same.wav", "sounds/b/same.wav"],
            &[
                ("sounds/a/same.wav", b"first"),
                ("sounds/b/same.wav", b"second"),
            ],
        );
        let app = dir.path().join("app");
        let result = import_into(&path, &app).unwrap();
        let profile = load_import(&app, &result);
        assert_ne!(profile.tracks[0].id, "track");
        assert_eq!(profile.tracks[0].volume, 0.25);
        assert_eq!(profile.key_bindings[0].track_id, profile.tracks[0].id);
        assert_eq!(profile.key_bindings[0].current_index, 0);
        for (i, expected) in [b"first".as_slice(), b"second".as_slice()]
            .iter()
            .enumerate()
        {
            let SoundSource::Local { path } = &profile.sounds[i].source else {
                panic!()
            };
            assert_eq!(&fs::read(path).unwrap(), expected);
            assert!(Path::new(path).starts_with(app.join("imported_sounds")));
            assert_eq!(profile.key_bindings[0].sound_ids[i], profile.sounds[i].id);
        }
    }

    #[test]
    fn shared_archive_file_is_extracted_once() {
        let dir = tempfile::tempdir().unwrap();
        let path = archive(
            dir.path(),
            &["sounds/shared.wav", "sounds/shared.wav"],
            &[("sounds/shared.wav", b"audio")],
        );
        let app = dir.path().join("app");
        let result = import_into(&path, &app).unwrap();
        assert_eq!(
            fs::read_dir(app.join("imported_sounds").join(&result.profile_id))
                .unwrap()
                .count(),
            1
        );
    }

    #[test]
    fn missing_file_rolls_back_partial_extraction() {
        let dir = tempfile::tempdir().unwrap();
        let path = archive(
            dir.path(),
            &["sounds/ok.wav", "sounds/missing.wav"],
            &[("sounds/ok.wav", b"audio")],
        );
        let app = dir.path().join("app");
        assert!(import_into(&path, &app).is_err());
        assert_eq!(
            fs::read_dir(app.join("imported_sounds")).unwrap().count(),
            0
        );
        assert!(!app.join("profiles").exists());
    }

    #[test]
    fn publication_failure_cleans_up_extracted_audio() {
        let dir = tempfile::tempdir().unwrap();
        let path = archive(
            dir.path(),
            &["sounds/ok.wav"],
            &[("sounds/ok.wav", b"audio")],
        );
        let app = dir.path().join("app");
        fs::create_dir_all(&app).unwrap();
        fs::write(app.join("profiles"), b"block directory creation").unwrap();
        assert!(import_into(&path, &app).is_err());
        assert_eq!(
            fs::read_dir(app.join("imported_sounds")).unwrap().count(),
            0
        );
    }
}
