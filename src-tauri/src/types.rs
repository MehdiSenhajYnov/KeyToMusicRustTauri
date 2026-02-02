use serde::{Deserialize, Serialize};

pub type SoundId = String;
pub type TrackId = String;
pub type ProfileId = String;
pub type KeyCode = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SoundSource {
    #[serde(rename = "local")]
    Local { path: String },
    #[serde(rename = "youtube")]
    YouTube {
        url: String,
        #[serde(rename = "cachedPath")]
        cached_path: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LoopMode {
    Off,
    Random,
    Single,
    Sequential,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MomentumModifier {
    Shift,
    Ctrl,
    Alt,
    None,
}

impl Default for MomentumModifier {
    fn default() -> Self {
        MomentumModifier::Shift
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sound {
    pub id: SoundId,
    pub name: String,
    pub source: SoundSource,
    pub momentum: f64,
    pub volume: f32,
    pub duration: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_video_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyBinding {
    pub key_code: KeyCode,
    pub track_id: TrackId,
    pub sound_ids: Vec<SoundId>,
    pub loop_mode: LoopMode,
    pub current_index: usize,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: TrackId,
    pub name: String,
    #[serde(default = "default_volume")]
    pub volume: f32,
}

fn default_volume() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: ProfileId,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub sounds: Vec<Sound>,
    pub tracks: Vec<Track>,
    pub key_bindings: Vec<KeyBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub master_volume: f32,
    pub auto_momentum: bool,
    pub key_detection_enabled: bool,
    pub master_stop_shortcut: Vec<KeyCode>,
    #[serde(default)]
    pub auto_momentum_shortcut: Vec<KeyCode>,
    #[serde(default)]
    pub key_detection_shortcut: Vec<KeyCode>,
    pub crossfade_duration: u32,
    pub key_cooldown: u32,
    pub current_profile_id: Option<ProfileId>,
    #[serde(default)]
    pub audio_device: Option<String>,
    #[serde(default = "default_chord_window_ms")]
    pub chord_window_ms: u32,
    #[serde(default)]
    pub momentum_modifier: MomentumModifier,
    #[serde(default)]
    pub playlist_import_enabled: bool,
}

fn default_chord_window_ms() -> u32 {
    30
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            master_volume: 0.8,
            auto_momentum: false,
            key_detection_enabled: true,
            master_stop_shortcut: vec![
                "ControlLeft".into(),
                "ShiftLeft".into(),
                "KeyS".into(),
            ],
            auto_momentum_shortcut: vec![],
            key_detection_shortcut: vec![],
            crossfade_duration: 500,
            key_cooldown: 200,
            current_profile_id: None,
            audio_device: None,
            chord_window_ms: 30,
            momentum_modifier: MomentumModifier::default(),
            playlist_import_enabled: false,
        }
    }
}
