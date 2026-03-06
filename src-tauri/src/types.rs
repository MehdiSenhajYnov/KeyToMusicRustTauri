use serde::{Deserialize, Serialize};

pub type SoundId = String;
pub type TrackId = String;
pub type ProfileId = String;
pub type KeyCode = String;

/// Base mood axis — 8 values (reduced from 10, removed emotional_climax and chase_action).
/// emotional_climax → any mood at intensity 3
/// chase_action → tension intensity 3 or epic intensity 2
/// epic_battle → epic intensity 2-3
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BaseMood {
    Epic,
    Tension,
    Sadness,
    Comedy,
    Romance,
    Horror,
    Peaceful,
    Mystery,
}

impl BaseMood {
    pub const ALL: [BaseMood; 8] = [
        BaseMood::Epic,
        BaseMood::Tension,
        BaseMood::Sadness,
        BaseMood::Comedy,
        BaseMood::Romance,
        BaseMood::Horror,
        BaseMood::Peaceful,
        BaseMood::Mystery,
    ];

    pub fn index(&self) -> usize {
        match self {
            BaseMood::Epic => 0,
            BaseMood::Tension => 1,
            BaseMood::Sadness => 2,
            BaseMood::Comedy => 3,
            BaseMood::Romance => 4,
            BaseMood::Horror => 5,
            BaseMood::Peaceful => 6,
            BaseMood::Mystery => 7,
        }
    }

    pub fn from_index(i: usize) -> Self {
        if i < 8 {
            Self::ALL[i]
        } else {
            BaseMood::Peaceful
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            BaseMood::Epic => "epic",
            BaseMood::Tension => "tension",
            BaseMood::Sadness => "sadness",
            BaseMood::Comedy => "comedy",
            BaseMood::Romance => "romance",
            BaseMood::Horror => "horror",
            BaseMood::Peaceful => "peaceful",
            BaseMood::Mystery => "mystery",
        }
    }

    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s {
            "epic" => Some(BaseMood::Epic),
            "tension" => Some(BaseMood::Tension),
            "sadness" => Some(BaseMood::Sadness),
            "comedy" => Some(BaseMood::Comedy),
            "romance" => Some(BaseMood::Romance),
            "horror" => Some(BaseMood::Horror),
            "peaceful" => Some(BaseMood::Peaceful),
            "mystery" => Some(BaseMood::Mystery),
            // Legacy mappings for backward compatibility
            "epic_battle" => Some(BaseMood::Epic),
            "chase_action" => Some(BaseMood::Tension),
            "emotional_climax" => None, // ambiguous — dropped
            _ => None,
        }
    }
}

/// Intensity axis — 3 levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoodIntensity {
    Low = 1,
    Medium = 2,
    High = 3,
}

impl MoodIntensity {
    pub fn as_u8(&self) -> u8 {
        match self {
            MoodIntensity::Low => 1,
            MoodIntensity::Medium => 2,
            MoodIntensity::High => 3,
        }
    }

    pub fn from_u8(n: u8) -> Self {
        match n {
            0 | 1 => MoodIntensity::Low,
            2 => MoodIntensity::Medium,
            _ => MoodIntensity::High,
        }
    }

    /// Round a float (1.0-3.0) to the nearest MoodIntensity.
    pub fn from_f32(v: f32) -> Self {
        if v < 1.5 {
            MoodIntensity::Low
        } else if v < 2.5 {
            MoodIntensity::Medium
        } else {
            MoodIntensity::High
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            MoodIntensity::Low => "low",
            MoodIntensity::Medium => "medium",
            MoodIntensity::High => "high",
        }
    }
}

/// Combined mood + intensity tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MoodTag {
    pub mood: BaseMood,
    pub intensity: MoodIntensity,
}

/// Legacy alias — references throughout the codebase use MoodCategory.
/// Maps to BaseMood for backward compatibility.
pub type MoodCategory = BaseMood;

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
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_mood_compat"
    )]
    pub mood: Option<BaseMood>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mood_intensity: Option<MoodIntensity>,
}

/// Deserialize mood field with backward compatibility for old MoodCategory values.
fn deserialize_mood_compat<'de, D>(deserializer: D) -> Result<Option<BaseMood>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    Ok(opt.and_then(|s| BaseMood::from_str_opt(&s)))
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
    #[serde(default)]
    pub disliked_videos: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub master_volume: f32,
    pub auto_momentum: bool,
    pub key_detection_enabled: bool,
    pub stop_all_shortcut: Vec<KeyCode>,
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
    #[serde(default)]
    pub mood_ai_enabled: bool,
    #[serde(default = "default_mood_api_port")]
    pub mood_api_port: u16,
    #[serde(default = "default_mood_entry_threshold")]
    pub mood_entry_threshold: f32,
    #[serde(default = "default_mood_exit_threshold")]
    pub mood_exit_threshold: f32,
    #[serde(default = "default_mood_dwell_pages")]
    pub mood_dwell_pages: u32,
    #[serde(default = "default_mood_window_size")]
    pub mood_window_size: usize,
}

fn default_chord_window_ms() -> u32 {
    30
}

fn default_mood_api_port() -> u16 {
    8765
}

fn default_mood_entry_threshold() -> f32 {
    0.55
}

fn default_mood_exit_threshold() -> f32 {
    0.25
}

fn default_mood_dwell_pages() -> u32 {
    2
}

fn default_mood_window_size() -> usize {
    5
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            master_volume: 0.8,
            auto_momentum: false,
            key_detection_enabled: true,
            stop_all_shortcut: vec!["ControlLeft".into(), "ShiftLeft".into(), "KeyS".into()],
            auto_momentum_shortcut: vec![],
            key_detection_shortcut: vec![],
            crossfade_duration: 500,
            key_cooldown: 200,
            current_profile_id: None,
            audio_device: None,
            chord_window_ms: 30,
            momentum_modifier: MomentumModifier::default(),
            playlist_import_enabled: false,
            mood_ai_enabled: false,
            mood_api_port: 8765,
            mood_entry_threshold: 0.55,
            mood_exit_threshold: 0.25,
            mood_dwell_pages: 2,
            mood_window_size: 5,
        }
    }
}
