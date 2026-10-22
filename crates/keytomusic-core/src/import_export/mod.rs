pub mod export;
pub mod import;

pub use export::{cancel_export, cleanup_interrupted_export, export_profile};
pub use import::import_profile;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
