use crate::state::AppState;
use crate::types::{AppConfig, MomentumModifier};

use super::mood;

fn parse_momentum_modifier(value: &str) -> Option<MomentumModifier> {
    match value {
        "Shift" => Some(MomentumModifier::Shift),
        "Ctrl" => Some(MomentumModifier::Ctrl),
        "Alt" => Some(MomentumModifier::Alt),
        "None" => Some(MomentumModifier::None),
        _ => None,
    }
}

/// Reject out-of-range values before anything is applied, so a bad request
/// leaves the config untouched instead of half-applied.
fn validate_config_updates(updates: &serde_json::Value) -> Result<(), String> {
    fn ranged(
        updates: &serde_json::Value,
        key: &str,
        range: std::ops::RangeInclusive<u64>,
        message: &str,
    ) -> Result<(), String> {
        match updates.get(key) {
            Some(v) if !v.as_u64().is_some_and(|v| range.contains(&v)) => Err(message.into()),
            _ => Ok(()),
        }
    }
    if let Some(v) = updates.get("masterVolume") {
        if !v.as_f64().is_some_and(|v| (0.0..=1.0).contains(&v)) {
            return Err("Master volume must be between 0 and 1".into());
        }
    }
    ranged(
        updates,
        "crossfadeDuration",
        100..=2000,
        "Crossfade must be between 100 and 2000ms",
    )?;
    ranged(
        updates,
        "stopFadeMs",
        0..=3000,
        "Stop fade must be at most 3000ms",
    )?;
    ranged(
        updates,
        "keyCooldown",
        0..=5000,
        "Cooldown must be at most 5000ms",
    )?;
    ranged(
        updates,
        "chordWindowMs",
        20..=100,
        "Chord window must be between 20 and 100ms",
    )?;
    if let Some(v) = updates.get("stopAllShortcut") {
        if !v.as_array().is_some_and(|keys| keys.len() >= 2) {
            return Err("Stop all shortcut must have at least 2 keys".into());
        }
    }
    if let Some(v) = updates.get("momentumModifier") {
        if v.as_str().and_then(parse_momentum_modifier).is_none() {
            return Err("Unknown momentum modifier".into());
        }
    }
    Ok(())
}

fn apply_config_updates(config: &mut AppConfig, updates: &serde_json::Value) {
    if let Some(v) = updates.get("masterVolume").and_then(|v| v.as_f64()) {
        config.master_volume = (v as f32).clamp(0.0, 1.0);
    }
    if let Some(v) = updates.get("autoMomentum").and_then(|v| v.as_bool()) {
        config.auto_momentum = v;
    }
    if let Some(v) = updates.get("keyDetectionEnabled").and_then(|v| v.as_bool()) {
        config.key_detection_enabled = v;
    }
    if let Some(v) = updates.get("crossfadeDuration").and_then(|v| v.as_u64()) {
        config.crossfade_duration = u32::try_from(v).unwrap_or(u32::MAX).clamp(100, 2000);
    }
    if let Some(v) = updates.get("stopFadeMs").and_then(|v| v.as_u64()) {
        config.stop_fade_ms = u32::try_from(v).unwrap_or(u32::MAX).min(3000);
    }
    if let Some(v) = updates.get("keyCooldown").and_then(|v| v.as_u64()) {
        config.key_cooldown = u32::try_from(v).unwrap_or(u32::MAX).min(5000);
    }
    if let Some(v) = updates.get("stopAllShortcut").and_then(|v| v.as_array()) {
        let keys: Vec<String> = v
            .iter()
            .filter_map(|k| k.as_str().map(|s| s.to_string()))
            .collect();
        if keys.len() >= 2 {
            config.stop_all_shortcut = keys;
        }
    }
    if let Some(v) = updates.get("pauseShortcut").and_then(|v| v.as_array()) {
        config.pause_shortcut = v
            .iter()
            .filter_map(|k| k.as_str().map(|s| s.to_string()))
            .collect();
    }
    if let Some(v) = updates
        .get("autoMomentumShortcut")
        .and_then(|v| v.as_array())
    {
        config.auto_momentum_shortcut = v
            .iter()
            .filter_map(|k| k.as_str().map(|s| s.to_string()))
            .collect();
    }
    if let Some(v) = updates
        .get("keyDetectionShortcut")
        .and_then(|v| v.as_array())
    {
        config.key_detection_shortcut = v
            .iter()
            .filter_map(|k| k.as_str().map(|s| s.to_string()))
            .collect();
    }
    if updates.get("currentProfileId").is_some() {
        config.current_profile_id = updates
            .get("currentProfileId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
    }
    if updates.get("audioDevice").is_some() {
        config.audio_device = updates.get("audioDevice").and_then(|v| {
            if v.is_null() {
                None
            } else {
                v.as_str().map(|s| s.to_string())
            }
        });
    }
    if let Some(v) = updates.get("chordWindowMs").and_then(|v| v.as_u64()) {
        config.chord_window_ms = u32::try_from(v).unwrap_or(u32::MAX).clamp(20, 100);
    }
    if let Some(v) = updates
        .get("momentumModifier")
        .and_then(|v| v.as_str())
        .and_then(parse_momentum_modifier)
    {
        config.momentum_modifier = v;
    }
    if let Some(v) = updates
        .get("playlistImportEnabled")
        .and_then(|v| v.as_bool())
    {
        config.playlist_import_enabled = v;
    }
    if let Some(v) = updates.get("moodAiEnabled").and_then(|v| v.as_bool()) {
        config.mood_ai_enabled = v;
    }
    if let Some(v) = updates
        .get("moodApiPort")
        .and_then(|v| v.as_u64())
        .and_then(|v| u16::try_from(v).ok())
    {
        config.mood_api_port = v;
    }
}

fn restore_config_fields(
    config: &mut AppConfig,
    previous: &AppConfig,
    updates: &serde_json::Value,
) {
    // Restore only this request's fields; unrelated concurrent changes must survive.
    if let (Ok(serde_json::Value::Object(mut fields)), Some(updates)) =
        (serde_json::to_value(previous), updates.as_object())
    {
        fields.retain(|key, _| updates.contains_key(key));
        apply_config_updates(config, &serde_json::Value::Object(fields));
    }
}

fn sync_runtime_dependent_config(state: &AppState, config: &AppConfig) {
    if let Ok(engine) = state.get_audio_engine() {
        let _ = engine.set_audio_device(config.audio_device.clone());
        let _ = engine.set_master_volume(config.master_volume);
        engine.set_stop_fade(config.stop_fade_ms);
    }

    state.key_detector.set_enabled(config.key_detection_enabled);
    state.key_detector.set_cooldown(config.key_cooldown);
    state
        .key_detector
        .set_stop_all_shortcut(config.stop_all_shortcut.clone());
    state
        .key_detector
        .set_auto_momentum_shortcut(config.auto_momentum_shortcut.clone());
    state
        .key_detector
        .set_pause_shortcut(config.pause_shortcut.clone());
    state
        .key_detector
        .set_key_detection_shortcut(config.key_detection_shortcut.clone());
    state.key_detector.set_chord_window(config.chord_window_ms);
}

/// Apply a partial config update (camelCase JSON fields), keep runtime
/// services in sync, persist it, and roll back the touched fields on failure.
pub async fn update_config(state: &AppState, updates: serde_json::Value) -> Result<(), String> {
    let touches_mood_service =
        updates.get("moodAiEnabled").is_some() || updates.get("moodApiPort").is_some();
    let _lifecycle = if touches_mood_service {
        Some(state.mood_lifecycle.lock().await)
    } else {
        None
    };
    if let Some(port) = updates.get("moodApiPort") {
        if !port
            .as_u64()
            .is_some_and(|port| (1024..=65535).contains(&port))
        {
            return Err("API port must be between 1024 and 65535".into());
        }
    }
    validate_config_updates(&updates)?;
    let previous = state.get_config();

    let config = state.update_config(|config| {
        apply_config_updates(config, &updates);
    });
    sync_runtime_dependent_config(state, &config);

    if touches_mood_service {
        if let Err(err) = mood::sync_service_config(state, &previous, &config).await {
            let restored = state.update_config(|config| {
                restore_config_fields(config, &previous, &updates);
            });
            sync_runtime_dependent_config(state, &restored);
            let _ = mood::sync_service_config(state, &config, &restored).await;
            return Err(err);
        }
    } else {
        mood::emit_api_status(state);
    }

    state.schedule_config_save();
    if let Err(error) = state.flush_config() {
        let restored = state.update_config(|current| {
            restore_config_fields(current, &previous, &updates);
        });
        sync_runtime_dependent_config(state, &restored);
        if touches_mood_service {
            let _ = mood::sync_service_config(state, &config, &restored).await;
        }
        state.schedule_config_save();
        return Err(error);
    }
    Ok(())
}

pub fn set_master_volume(state: &AppState, volume: f32) {
    if !volume.is_finite() {
        return;
    }
    let volume = volume.clamp(0.0, 1.0);
    state.update_config(|config| {
        config.master_volume = volume;
    });
    state.schedule_config_save();

    // Graceful if the engine is not ready yet: it picks the value up from config on init.
    if let Ok(engine) = state.get_audio_engine() {
        let _ = engine.set_master_volume(volume);
    }
}

pub fn set_key_detection(state: &AppState, enabled: bool) {
    state.key_detector.set_enabled(enabled);
    state.update_config(|config| {
        config.key_detection_enabled = enabled;
    });
    state.schedule_config_save();
}

pub fn set_stop_all_shortcut(state: &AppState, keys: Vec<String>) -> Result<(), String> {
    if keys.len() < 2 {
        return Err("Stop all shortcut must have at least 2 keys".to_string());
    }
    state.key_detector.set_stop_all_shortcut(keys.clone());
    state.update_config(|config| {
        config.stop_all_shortcut = keys;
    });
    state.schedule_config_save();
    Ok(())
}

pub fn set_key_cooldown(state: &AppState, cooldown_ms: u32) -> Result<(), String> {
    if cooldown_ms > 5000 {
        return Err("Cooldown must be at most 5000ms".to_string());
    }
    state.key_detector.set_cooldown(cooldown_ms);
    state.update_config(|config| {
        config.key_cooldown = cooldown_ms;
    });
    state.schedule_config_save();
    Ok(())
}

pub fn set_audio_device(state: &AppState, device: Option<String>) -> Result<(), String> {
    state.get_audio_engine()?.set_audio_device(device.clone())?;
    state.update_config(|config| {
        config.audio_device = device;
    });
    state.schedule_config_save();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollback_restores_only_requested_fields() {
        let previous = AppConfig::default();
        let mut current = previous.clone();
        current.mood_api_port = 9999;
        current.master_volume = 0.2;
        restore_config_fields(
            &mut current,
            &previous,
            &serde_json::json!({"moodApiPort": 9999}),
        );
        assert_eq!(current.mood_api_port, previous.mood_api_port);
        assert_eq!(current.master_volume, 0.2);
    }

    #[test]
    fn invalid_updates_are_rejected() {
        for bad in [
            serde_json::json!({"masterVolume": 1.5}),
            serde_json::json!({"crossfadeDuration": 50}),
            serde_json::json!({"keyCooldown": 4294967296u64}),
            serde_json::json!({"chordWindowMs": 500}),
            serde_json::json!({"stopAllShortcut": ["KeyS"]}),
            serde_json::json!({"momentumModifier": "Meta"}),
        ] {
            assert!(validate_config_updates(&bad).is_err(), "{bad}");
        }
        assert!(validate_config_updates(&serde_json::json!({
            "masterVolume": 0.5,
            "crossfadeDuration": 500,
            "keyCooldown": 0,
            "chordWindowMs": 30,
            "stopAllShortcut": ["ControlLeft", "KeyS"],
            "momentumModifier": "Alt"
        }))
        .is_ok());
    }
}
