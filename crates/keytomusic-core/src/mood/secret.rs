//! The API key, kept out of `config.json` (which gets shared and exported) in
//! its own file readable only by the user.

use std::path::PathBuf;

fn key_path() -> PathBuf {
    crate::storage::get_app_data_dir().join("mood_api_key")
}

/// Stored key, else `KEYTOMUSIC_MOOD_API_KEY`, else `OPENAI_API_KEY` for OpenAI.
pub fn load(base_url: &str) -> Option<String> {
    let stored = std::fs::read_to_string(key_path())
        .ok()
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty());
    stored
        .or_else(|| std::env::var("KEYTOMUSIC_MOOD_API_KEY").ok())
        .or_else(|| {
            base_url
                .contains("api.openai.com")
                .then(|| std::env::var("OPENAI_API_KEY").ok())
                .flatten()
        })
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
}

/// Save (or with `None`, delete) the key.
pub fn save(key: Option<&str>) -> Result<(), String> {
    let path = key_path();
    match key.map(str::trim).filter(|k| !k.is_empty()) {
        None => match std::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                Err(format!("Impossible d'effacer la clé : {e}"))
            }
            _ => Ok(()),
        },
        Some(key) => {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            }
            write_private(&path, key).map_err(|e| format!("Impossible d'enregistrer la clé : {e}"))
        }
    }
}

#[cfg(unix)]
fn write_private(path: &std::path::Path, key: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    file.write_all(key.as_bytes())
}

#[cfg(not(unix))]
fn write_private(path: &std::path::Path, key: &str) -> std::io::Result<()> {
    std::fs::write(path, key)
}

/// `sk-…abcd`, for showing which key is saved.
pub fn masked(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    if chars.len() <= 8 {
        return "••••".into();
    }
    let head: String = chars[..3].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{head}…{tail}")
}

#[cfg(test)]
mod tests {
    #[test]
    fn keys_are_masked() {
        assert_eq!(super::masked("sk-proj-abcdefghijkl1234"), "sk-…1234");
        assert_eq!(super::masked("short"), "••••");
    }
}
