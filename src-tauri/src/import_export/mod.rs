pub mod export;
pub mod import;

pub use export::{export_profile, cleanup_interrupted_export, cancel_export};
pub use import::import_profile;

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::audio::analysis::WaveformData;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportMetadata {
    pub version: String,
    pub exported_at: String,
    pub app_version: String,
    pub platform: String,
}

/// Result of importing a .ktm file.
pub struct ImportResult {
    pub profile_id: String,
    pub waveforms: HashMap<String, WaveformData>,
}
