//! The browser extension ships inside the app: the UIs export it to a folder
//! the user loads in their browser ("Load unpacked").
//!
//! Updates need no action: when the app starts, it rewrites the copies it
//! exported if they are older, and the extension reloads itself when the app
//! reports a newer version (see `background.js`).

use std::path::{Path, PathBuf};

macro_rules! file {
    ($path:literal) => {
        (
            $path,
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/extension/", $path)) as &[u8],
        )
    };
}

const FILES: &[(&str, &[u8])] = &[
    file!("manifest.json"),
    file!("background.js"),
    file!("content.js"),
    file!("popup.html"),
    file!("popup.css"),
    file!("popup.js"),
    file!("README.md"),
    file!("icons/icon16.png"),
    file!("icons/icon32.png"),
    file!("icons/icon48.png"),
    file!("icons/icon128.png"),
];

pub const FOLDER_NAME: &str = "KeyToMusic-Mood-AI-extension";
/// Version in `manifest.json` (checked by a test).
pub const VERSION: &str = "3.0.0";
const NAME: &str = "KeyToMusic Mood AI";

/// Where the extension goes by default: Downloads, else the home folder.
pub fn default_destination() -> PathBuf {
    dirs::download_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(std::env::temp_dir)
        .join(FOLDER_NAME)
}

/// Folders the extension was exported to (to update them later).
fn registry() -> PathBuf {
    crate::storage::get_app_data_dir().join("mood_extension_dirs.json")
}

fn remembered() -> Vec<PathBuf> {
    std::fs::read(registry())
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn remember(dir: &Path) {
    let mut dirs = remembered();
    if !dirs.iter().any(|d| d == dir) {
        dirs.push(dir.to_path_buf());
        if let Ok(bytes) = serde_json::to_vec(&dirs) {
            let _ = std::fs::write(registry(), bytes);
        }
    }
}

/// Version of the extension copy in `dir`, if it is ours.
pub fn installed_version(dir: &Path) -> Option<String> {
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("manifest.json")).ok()?).ok()?;
    (manifest.get("name")?.as_str()? == NAME)
        .then(|| manifest.get("version")?.as_str().map(str::to_string))
        .flatten()
}

/// The folder holding an installed copy (the last exported, else the
/// default one), if any.
pub fn installed_dir() -> Option<PathBuf> {
    let mut dirs = remembered();
    dirs.reverse();
    dirs.push(default_destination());
    dirs.into_iter()
        .find(|dir| installed_version(dir).is_some())
}

/// Bring every exported copy (and the default folder's) up to date.
/// Returns the folders rewritten.
pub fn refresh_installed() -> Vec<PathBuf> {
    let mut dirs = remembered();
    let default = default_destination();
    if !dirs.contains(&default) {
        dirs.push(default);
    }
    dirs.into_iter()
        .filter(|dir| installed_version(dir).is_some_and(|v| v != VERSION))
        .filter(|dir| match write_files(dir) {
            Ok(()) => true,
            Err(error) => {
                tracing::warn!(
                    "Mood AI: extension in {} not updated: {error}",
                    dir.display()
                );
                false
            }
        })
        .collect()
}

/// Write the extension into `dir` (replacing an older copy). Returns `dir`.
pub fn export(dir: &Path) -> Result<PathBuf, String> {
    write_files(dir)?;
    remember(dir);
    Ok(dir.to_path_buf())
}

fn write_files(dir: &Path) -> Result<(), String> {
    for (name, bytes) in FILES {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Impossible de créer {} : {e}", parent.display()))?;
        }
        std::fs::write(&path, bytes)
            .map_err(|e| format!("Impossible d'écrire {} : {e}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_exported_manifest_lists_existing_files() {
        let dir = tempfile::tempdir().unwrap();
        super::write_files(dir.path()).unwrap();
        let out = dir.path();
        let manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(out.join("manifest.json")).unwrap()).unwrap();
        assert_eq!(manifest["manifest_version"], 3);
        assert_eq!(manifest["version"], super::VERSION);
        assert_eq!(manifest["name"], super::NAME);
        assert_eq!(
            super::installed_version(out).as_deref(),
            Some(super::VERSION)
        );
        for script in [
            "background.js",
            "content.js",
            "popup.html",
            "icons/icon128.png",
        ] {
            assert!(out.join(script).exists(), "{script} missing");
        }
        assert_eq!(manifest["content_scripts"][0]["js"][0], "content.js");
    }
}
