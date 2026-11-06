//! The browser extension ships inside the app: the UIs export it to a folder
//! the user loads in their browser.

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

/// Where the extension goes by default: Downloads, else the home folder.
pub fn default_destination() -> PathBuf {
    dirs::download_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(std::env::temp_dir)
        .join(FOLDER_NAME)
}

/// Write the extension into `dir` (replacing an older copy). Returns `dir`.
pub fn export(dir: &Path) -> Result<PathBuf, String> {
    for (name, bytes) in FILES {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Impossible de créer {} : {e}", parent.display()))?;
        }
        std::fs::write(&path, bytes)
            .map_err(|e| format!("Impossible d'écrire {} : {e}", path.display()))?;
    }
    Ok(dir.to_path_buf())
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_exported_manifest_lists_existing_files() {
        let dir = tempfile::tempdir().unwrap();
        let out = super::export(dir.path()).unwrap();
        let manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(out.join("manifest.json")).unwrap()).unwrap();
        assert_eq!(manifest["manifest_version"], 3);
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
