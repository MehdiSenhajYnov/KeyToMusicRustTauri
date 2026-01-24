use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    // Audio errors
    #[error("Sound file not found: {path}")]
    SoundFileNotFound { sound_id: String, path: String },

    #[error("Unsupported audio format: {format}")]
    UnsupportedFormat { format: String },

    #[error("Audio playback failed: {reason}")]
    PlaybackFailed { reason: String },

    // YouTube errors
    #[error("Invalid YouTube URL: {url}")]
    InvalidYouTubeUrl { url: String },

    #[error("YouTube download failed: {reason}")]
    YouTubeDownloadFailed { reason: String },

    #[error("yt-dlp not found")]
    YtDlpNotFound,

    // Storage errors
    #[error("Profile not found: {id}")]
    ProfileNotFound { id: String },

    #[error("Failed to save: {reason}")]
    SaveFailed { reason: String },

    #[error("Failed to load: {reason}")]
    LoadFailed { reason: String },

    // Import/Export errors
    #[error("Invalid export file: {reason}")]
    InvalidExportFile { reason: String },

    #[error("Export failed: {reason}")]
    ExportFailed { reason: String },

    // Key errors
    #[error("Key already assigned to Master Stop: {key_code}")]
    KeyAlreadyAssigned { key_code: String },

    #[error("Invalid key combination for master stop")]
    InvalidMasterStopShortcut,
}

impl From<AppError> for String {
    fn from(error: AppError) -> Self {
        error.to_string()
    }
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
