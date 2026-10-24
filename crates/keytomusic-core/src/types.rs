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
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "binding_intensity"
    )]
    pub mood_intensity: Option<MoodIntensity>,
}

/// A binding's minimum intensity travels as the number 1–3 (what the web UI
/// sends); older profiles stored it as "low"/"medium"/"high", which still loads.
mod binding_intensity {
    use super::MoodIntensity;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(
        value: &Option<MoodIntensity>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(intensity) => serializer.serialize_u8(intensity.as_u8()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<MoodIntensity>, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Number(u8),
            Text(String),
        }
        Ok(match Option::<Raw>::deserialize(deserializer)? {
            Some(Raw::Number(n)) => Some(MoodIntensity::from_u8(n)),
            Some(Raw::Text(text)) => match text.as_str() {
                "low" | "1" => Some(MoodIntensity::Low),
                "medium" | "2" => Some(MoodIntensity::Medium),
                "high" | "3" => Some(MoodIntensity::High),
                _ => None,
            },
            None => None,
        })
    }
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
    /// While this track plays, the other tracks are lowered (sound effects
    /// over the music).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ducks_others: bool,
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

// Struct-level default: a missing or newly added field falls back to its
// default instead of failing the whole config parse.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppConfig {
    pub master_volume: f32,
    pub auto_momentum: bool,
    pub key_detection_enabled: bool,
    pub stop_all_shortcut: Vec<KeyCode>,
    /// Pause or resume everything that plays.
    pub pause_shortcut: Vec<KeyCode>,
    #[serde(default)]
    pub auto_momentum_shortcut: Vec<KeyCode>,
    #[serde(default)]
    pub key_detection_shortcut: Vec<KeyCode>,
    pub crossfade_duration: u32,
    /// Fade-out when sounds are stopped (Stop all, a track's stop), in ms.
    pub stop_fade_ms: u32,
    /// Gain of the other tracks while a ducking track plays (0–1).
    pub duck_level: f32,
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
}

fn default_chord_window_ms() -> u32 {
    30
}

fn default_mood_api_port() -> u16 {
    8765
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            master_volume: 0.8,
            auto_momentum: false,
            key_detection_enabled: true,
            stop_all_shortcut: vec!["ControlLeft".into(), "ShiftLeft".into(), "KeyS".into()],
            pause_shortcut: vec!["ControlLeft".into(), "ShiftLeft".into(), "KeyP".into()],
            auto_momentum_shortcut: vec![],
            key_detection_shortcut: vec![],
            crossfade_duration: 500,
            stop_fade_ms: 400,
            duck_level: 0.3,
            key_cooldown: 200,
            current_profile_id: None,
            audio_device: None,
            chord_window_ms: 30,
            momentum_modifier: MomentumModifier::default(),
            playlist_import_enabled: false,
            mood_ai_enabled: false,
            mood_api_port: 8765,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binding_intensity_accepts_numbers_and_legacy_strings() {
        let base =
            r#"{"keyCode":"KeyA","trackId":"t","soundIds":[],"loopMode":"off","currentIndex":0"#;
        let from_number: KeyBinding =
            serde_json::from_str(&format!(r#"{base},"moodIntensity":2}}"#)).unwrap();
        assert_eq!(from_number.mood_intensity, Some(MoodIntensity::Medium));
        let from_text: KeyBinding =
            serde_json::from_str(&format!(r#"{base},"moodIntensity":"high"}}"#)).unwrap();
        assert_eq!(from_text.mood_intensity, Some(MoodIntensity::High));
        let none: KeyBinding = serde_json::from_str(&format!("{base}}}")).unwrap();
        assert_eq!(none.mood_intensity, None);
        let json = serde_json::to_value(&from_text).unwrap();
        assert_eq!(json["moodIntensity"], 3);
    }

    #[test]
    fn track_ducking_is_optional_on_the_wire() {
        let old: Track = serde_json::from_str(r#"{"id":"t","name":"OST","volume":1.0}"#).unwrap();
        assert!(!old.ducks_others);
        assert!(serde_json::to_value(&old).unwrap().get("ducksOthers").is_none());
        let ducking = Track {
            ducks_others: true,
            ..old
        };
        assert_eq!(serde_json::to_value(&ducking).unwrap()["ducksOthers"], true);
    }
}
