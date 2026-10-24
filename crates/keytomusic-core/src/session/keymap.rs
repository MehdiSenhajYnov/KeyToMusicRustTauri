//! Key codes and combos (web `KeyboardEvent.code` names, "Ctrl+Shift+KeyA").
//!
//! Bindings always store physical codes; labels for the user's layout are a
//! host concern. Combos are normalized as modifiers (Ctrl, Shift, Alt) then the
//! base keys sorted alphabetically, matching `keys::chord` and the web frontend.

use std::collections::HashSet;

use crate::types::{AppConfig, KeyBinding, MomentumModifier};

const MODIFIERS: [&str; 3] = ["Ctrl", "Shift", "Alt"];

fn is_modifier_part(part: &str) -> bool {
    MODIFIERS.contains(&part)
}

/// Normalize a combo string: modifiers first (Ctrl > Shift > Alt), then base keys sorted.
pub fn normalize_combo(combo: &str) -> String {
    let parts: Vec<&str> = combo.split('+').filter(|p| !p.is_empty()).collect();
    let mut base: Vec<&str> = parts
        .iter()
        .copied()
        .filter(|p| !is_modifier_part(p))
        .collect();
    base.sort_unstable();
    base.dedup();
    let mut out: Vec<&str> = MODIFIERS
        .iter()
        .copied()
        .filter(|m| parts.contains(m))
        .collect();
    out.extend(base);
    out.join("+")
}

/// Build a normalized combo from physically held key codes ("ControlLeft", "KeyA", ...).
/// Returns `None` when only modifiers are held.
pub fn combo_from_pressed<'a>(pressed: impl IntoIterator<Item = &'a str>) -> Option<String> {
    let mut ctrl = false;
    let mut shift = false;
    let mut alt = false;
    let mut base: Vec<&str> = Vec::new();
    for key in pressed {
        match key {
            "ControlLeft" | "ControlRight" => ctrl = true,
            "ShiftLeft" | "ShiftRight" => shift = true,
            "AltLeft" | "AltRight" => alt = true,
            other => base.push(other),
        }
    }
    if base.is_empty() {
        return None;
    }
    base.sort_unstable();
    base.dedup();
    let mut parts = Vec::new();
    if ctrl {
        parts.push("Ctrl");
    }
    if shift {
        parts.push("Shift");
    }
    if alt {
        parts.push("Alt");
    }
    parts.extend(base);
    Some(parts.join("+"))
}

/// Last part of a combo (the base key for single-key combos).
pub fn base_key(combo: &str) -> &str {
    combo.rsplit('+').next().unwrap_or(combo)
}

pub fn has_modifier(combo: &str, modifier: &MomentumModifier) -> bool {
    let wanted = match modifier {
        MomentumModifier::Shift => "Shift",
        MomentumModifier::Ctrl => "Ctrl",
        MomentumModifier::Alt => "Alt",
        MomentumModifier::None => return false,
    };
    combo.split('+').any(|p| p == wanted)
}

/// More than one non-modifier key, e.g. "KeyA+KeyZ".
pub fn is_multi_key_chord(combo: &str) -> bool {
    combo.split('+').filter(|p| !is_modifier_part(p)).count() > 1
}

/// Short, layout-independent fallback label for a physical code ("KeyA" → "A", "Numpad4" → "Num4").
pub fn fallback_label(code: &str) -> String {
    if let Some(letter) = code.strip_prefix("Key") {
        return letter.to_string();
    }
    if let Some(digit) = code.strip_prefix("Digit") {
        return digit.to_string();
    }
    let fixed = match code {
        "NumpadMultiply" => "Num*",
        "NumpadAdd" => "Num+",
        "NumpadSubtract" => "Num-",
        "NumpadDecimal" => "Num.",
        "NumpadDivide" => "Num/",
        "NumpadEnter" => "NumEnter",
        "ArrowUp" => "↑",
        "ArrowDown" => "↓",
        "ArrowLeft" => "←",
        "ArrowRight" => "→",
        "Escape" => "Esc",
        "Backspace" => "Bksp",
        "Delete" => "Del",
        "Insert" => "Ins",
        "PageUp" => "PgUp",
        "PageDown" => "PgDn",
        "CapsLock" => "Caps",
        "NumLock" => "NumLk",
        "ScrollLock" => "ScrLk",
        "PrintScreen" => "PrtSc",
        "ShiftLeft" => "LShift",
        "ShiftRight" => "RShift",
        "ControlLeft" => "LCtrl",
        "ControlRight" => "RCtrl",
        "AltLeft" => "LAlt",
        "AltRight" => "RAlt",
        "MetaLeft" => "LMeta",
        "MetaRight" => "RMeta",
        "Semicolon" => ";",
        "Comma" => ",",
        "Period" => ".",
        "Slash" => "/",
        "Backslash" => "\\",
        "Quote" => "'",
        "Backquote" => "`",
        "BracketLeft" => "[",
        "BracketRight" => "]",
        "Minus" => "-",
        "Equal" => "=",
        _ => "",
    };
    if !fixed.is_empty() {
        return fixed.to_string();
    }
    if let Some(n) = code.strip_prefix("Numpad") {
        return format!("Num{}", n);
    }
    code.to_string()
}

/// Display a combo, resolving each base key through `label` (e.g. the system layout).
pub fn display_combo(combo: &str, label: impl Fn(&str) -> Option<String>) -> String {
    combo
        .split('+')
        .map(|part| {
            if is_modifier_part(part) {
                part.to_string()
            } else {
                label(part).unwrap_or_else(|| fallback_label(part))
            }
        })
        .collect::<Vec<_>>()
        .join("+")
}

/// Preferred order when a key must be picked automatically: the letter rows
/// from the top left, as they sit under the fingers (A Z E R T Y… on AZERTY,
/// Q W E R T Y… on QWERTY), then digits, function keys and the keypad.
pub const AUTO_KEY_ORDER: &[&str] = &[
    "KeyQ", "KeyW", "KeyE", "KeyR", "KeyT", "KeyY", "KeyU", "KeyI", "KeyO", "KeyP", "KeyA", "KeyS",
    "KeyD", "KeyF", "KeyG", "KeyH", "KeyJ", "KeyK", "KeyL", "KeyZ", "KeyX", "KeyC", "KeyV", "KeyB",
    "KeyN", "KeyM", "Digit1", "Digit2", "Digit3", "Digit4", "Digit5", "Digit6", "Digit7", "Digit8",
    "Digit9", "Digit0", "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12",
    "Numpad0", "Numpad1", "Numpad2", "Numpad3", "Numpad4", "Numpad5", "Numpad6", "Numpad7",
    "Numpad8", "Numpad9",
];

/// First key of [`AUTO_KEY_ORDER`] not in `used` nor `reserved`.
pub fn next_available_key(used: &HashSet<String>, reserved: &HashSet<String>) -> Option<String> {
    AUTO_KEY_ORDER
        .iter()
        .find(|k| !used.contains(**k) && !reserved.contains(**k))
        .map(|k| k.to_string())
}

/// A global shortcut that would collide with "momentum modifier + bound key".
#[derive(Debug, Clone, PartialEq)]
pub struct MomentumConflict {
    pub shortcut_name: &'static str,
    pub shortcut_keys: Vec<String>,
    pub bound_key: String,
}

fn shortcut_has_modifier(keys: &[String], modifier: &MomentumModifier) -> bool {
    let (left, right) = match modifier {
        MomentumModifier::Shift => ("ShiftLeft", "ShiftRight"),
        MomentumModifier::Ctrl => ("ControlLeft", "ControlRight"),
        MomentumModifier::Alt => ("AltLeft", "AltRight"),
        MomentumModifier::None => return false,
    };
    keys.iter().any(|k| k == left || k == right)
}

fn shortcut_base_key(keys: &[String]) -> Option<&str> {
    keys.iter()
        .map(String::as_str)
        .find(|k| !k.contains("Shift") && !k.contains("Control") && !k.contains("Alt"))
}

pub fn global_shortcuts(config: &AppConfig) -> [(&'static str, &Vec<String>); 4] {
    [
        ("Stop All", &config.stop_all_shortcut),
        ("Pause", &config.pause_shortcut),
        ("Auto-Momentum", &config.auto_momentum_shortcut),
        ("Key Detection", &config.key_detection_shortcut),
    ]
}

/// Conflict affecting one bound key, if any.
pub fn key_momentum_conflict(key_code: &str, config: &AppConfig) -> Option<MomentumConflict> {
    if config.momentum_modifier == MomentumModifier::None {
        return None;
    }
    let base = base_key(key_code);
    global_shortcuts(config)
        .into_iter()
        .find(|(_, keys)| {
            !keys.is_empty()
                && shortcut_base_key(keys) == Some(base)
                && shortcut_has_modifier(keys, &config.momentum_modifier)
        })
        .map(|(name, keys)| MomentumConflict {
            shortcut_name: name,
            shortcut_keys: keys.clone(),
            bound_key: base.to_string(),
        })
}

/// All shortcuts conflicting with the profile's bound keys.
pub fn momentum_conflicts(config: &AppConfig, bindings: &[KeyBinding]) -> Vec<MomentumConflict> {
    let bound: HashSet<&str> = bindings.iter().map(|b| base_key(&b.key_code)).collect();
    let mut seen = HashSet::new();
    bindings
        .iter()
        .filter_map(|b| key_momentum_conflict(&b.key_code, config))
        .filter(|c| bound.contains(c.bound_key.as_str()) && seen.insert(c.shortcut_name))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_modifiers_then_sorted_keys() {
        assert_eq!(normalize_combo("KeyZ+Alt+KeyA+Ctrl"), "Ctrl+Alt+KeyA+KeyZ");
        assert_eq!(normalize_combo("KeyA"), "KeyA");
    }

    #[test]
    fn builds_combo_from_physical_keys() {
        assert_eq!(
            combo_from_pressed(["ShiftLeft", "KeyZ", "ControlRight", "KeyA"]).as_deref(),
            Some("Ctrl+Shift+KeyA+KeyZ")
        );
        assert_eq!(combo_from_pressed(["ShiftLeft"]), None);
    }

    #[test]
    fn labels_and_chords() {
        assert_eq!(fallback_label("Numpad7"), "Num7");
        assert_eq!(
            display_combo("Ctrl+KeyQ", |c| (c == "KeyQ").then(|| "A".into())),
            "Ctrl+A"
        );
        assert!(is_multi_key_chord("Shift+KeyA+KeyZ"));
        assert!(!is_multi_key_chord("Shift+KeyA"));
    }

    #[test]
    fn detects_momentum_conflicts() {
        let config = AppConfig {
            stop_all_shortcut: vec!["ShiftLeft".into(), "KeyS".into()],
            ..AppConfig::default()
        };
        let conflict = key_momentum_conflict("KeyS", &config).unwrap();
        assert_eq!(conflict.shortcut_name, "Stop All");
        assert!(key_momentum_conflict("KeyA", &config).is_none());
    }

    #[test]
    fn next_key_skips_used_and_reserved() {
        let used: HashSet<String> = ["KeyQ".to_string()].into();
        let reserved: HashSet<String> = ["KeyW".to_string()].into();
        assert_eq!(
            next_available_key(&used, &reserved).as_deref(),
            Some("KeyE")
        );
    }
}
