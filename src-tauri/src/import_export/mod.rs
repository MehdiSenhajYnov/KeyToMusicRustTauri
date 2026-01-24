pub mod export;
pub mod import;

pub use export::{export_profile, cleanup_interrupted_export, cancel_export};
pub use import::import_profile;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportMetadata {
    pub version: String,
    pub exported_at: String,
    pub app_version: String,
    pub platform: String,
}
