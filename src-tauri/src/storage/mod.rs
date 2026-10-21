pub mod config;
pub mod profile;

pub use config::*;
pub use profile::*;

/// Write a complete replacement beside the destination, then atomically publish it.
/// Unique temporary names prevent concurrent writers from sharing a temp file.
pub fn atomic_write(path: &std::path::Path, contents: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let parent = path.parent().ok_or("Missing destination directory")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    temp.write_all(contents).map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(path)
        .map_err(|e| format!("Could not save {}: {}", path.display(), e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn atomic_replacement_preserves_the_destination_on_failure() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("profile.json");
        atomic_write(&file, b"before").unwrap();
        atomic_write(&file, b"after").unwrap();
        assert_eq!(std::fs::read(&file).unwrap(), b"after");
        let directory = dir.path().join("directory");
        std::fs::create_dir(&directory).unwrap();
        std::fs::write(directory.join("keep"), b"valuable").unwrap();
        assert!(atomic_write(&directory, b"invalid").is_err());
        assert_eq!(std::fs::read(directory.join("keep")).unwrap(), b"valuable");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
    }
    #[test]
    fn concurrent_writers_leave_one_complete_file_and_no_temp_files() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("profile.json");
        std::thread::scope(|scope| {
            for value in 0..8 {
                let file = &file;
                scope.spawn(move || atomic_write(file, &vec![value; 8192]).unwrap());
            }
        });
        let bytes = std::fs::read(file).unwrap();
        assert_eq!(bytes.len(), 8192);
        assert!(bytes.iter().all(|b| *b == bytes[0]));
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
