#[cfg(not(any(target_os = "macos", target_os = "windows")))]
use rdev::Key;

/// Events emitted by the key detection system.
#[derive(Debug, Clone)]
pub enum KeyEvent {
    KeyPressed { key_code: String, with_shift: bool },
    MasterStop,
    ToggleKeyDetection,
    ToggleAutoMomentum,
}

/// Convert an rdev::Key to a string key code matching Web KeyboardEvent.code format.
/// Only used on Linux (Windows and macOS have their own implementations).
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn key_to_code(key: Key) -> String {
    match key {
        // Letters A-Z
        Key::KeyA => "KeyA".to_string(),
        Key::KeyB => "KeyB".to_string(),
        Key::KeyC => "KeyC".to_string(),
        Key::KeyD => "KeyD".to_string(),
        Key::KeyE => "KeyE".to_string(),
        Key::KeyF => "KeyF".to_string(),
        Key::KeyG => "KeyG".to_string(),
        Key::KeyH => "KeyH".to_string(),
        Key::KeyI => "KeyI".to_string(),
        Key::KeyJ => "KeyJ".to_string(),
        Key::KeyK => "KeyK".to_string(),
        Key::KeyL => "KeyL".to_string(),
        Key::KeyM => "KeyM".to_string(),
        Key::KeyN => "KeyN".to_string(),
        Key::KeyO => "KeyO".to_string(),
        Key::KeyP => "KeyP".to_string(),
        Key::KeyQ => "KeyQ".to_string(),
        Key::KeyR => "KeyR".to_string(),
        Key::KeyS => "KeyS".to_string(),
        Key::KeyT => "KeyT".to_string(),
        Key::KeyU => "KeyU".to_string(),
        Key::KeyV => "KeyV".to_string(),
        Key::KeyW => "KeyW".to_string(),
        Key::KeyX => "KeyX".to_string(),
        Key::KeyY => "KeyY".to_string(),
        Key::KeyZ => "KeyZ".to_string(),

        // Digits 0-9
        Key::Num0 => "Digit0".to_string(),
        Key::Num1 => "Digit1".to_string(),
        Key::Num2 => "Digit2".to_string(),
        Key::Num3 => "Digit3".to_string(),
        Key::Num4 => "Digit4".to_string(),
        Key::Num5 => "Digit5".to_string(),
        Key::Num6 => "Digit6".to_string(),
        Key::Num7 => "Digit7".to_string(),
        Key::Num8 => "Digit8".to_string(),
        Key::Num9 => "Digit9".to_string(),

        // Numpad
        Key::Kp0 => "Numpad0".to_string(),
        Key::Kp1 => "Numpad1".to_string(),
        Key::Kp2 => "Numpad2".to_string(),
        Key::Kp3 => "Numpad3".to_string(),
        Key::Kp4 => "Numpad4".to_string(),
        Key::Kp5 => "Numpad5".to_string(),
        Key::Kp6 => "Numpad6".to_string(),
        Key::Kp7 => "Numpad7".to_string(),
        Key::Kp8 => "Numpad8".to_string(),
        Key::Kp9 => "Numpad9".to_string(),
        Key::KpMultiply => "NumpadMultiply".to_string(),
        Key::KpPlus => "NumpadAdd".to_string(),
        Key::KpMinus => "NumpadSubtract".to_string(),
        Key::KpDecimal => "NumpadDecimal".to_string(),
        Key::KpDivide => "NumpadDivide".to_string(),
        Key::KpReturn => "NumpadEnter".to_string(),

        // Function keys
        Key::F1 => "F1".to_string(),
        Key::F2 => "F2".to_string(),
        Key::F3 => "F3".to_string(),
        Key::F4 => "F4".to_string(),
        Key::F5 => "F5".to_string(),
        Key::F6 => "F6".to_string(),
        Key::F7 => "F7".to_string(),
        Key::F8 => "F8".to_string(),
        Key::F9 => "F9".to_string(),
        Key::F10 => "F10".to_string(),
        Key::F11 => "F11".to_string(),
        Key::F12 => "F12".to_string(),

        // Arrow keys
        Key::UpArrow => "ArrowUp".to_string(),
        Key::DownArrow => "ArrowDown".to_string(),
        Key::LeftArrow => "ArrowLeft".to_string(),
        Key::RightArrow => "ArrowRight".to_string(),

        // Special keys
        Key::Space => "Space".to_string(),
        Key::Return => "Enter".to_string(),
        Key::Tab => "Tab".to_string(),
        Key::Escape => "Escape".to_string(),
        Key::Backspace => "Backspace".to_string(),
        Key::Delete => "Delete".to_string(),
        Key::Insert => "Insert".to_string(),
        Key::Home => "Home".to_string(),
        Key::End => "End".to_string(),
        Key::PageUp => "PageUp".to_string(),
        Key::PageDown => "PageDown".to_string(),
        Key::CapsLock => "CapsLock".to_string(),
        Key::NumLock => "NumLock".to_string(),
        Key::ScrollLock => "ScrollLock".to_string(),
        Key::PrintScreen => "PrintScreen".to_string(),
        Key::Pause => "Pause".to_string(),

        // Modifiers
        Key::ShiftLeft => "ShiftLeft".to_string(),
        Key::ShiftRight => "ShiftRight".to_string(),
        Key::ControlLeft => "ControlLeft".to_string(),
        Key::ControlRight => "ControlRight".to_string(),
        Key::Alt => "AltLeft".to_string(),
        Key::AltGr => "AltRight".to_string(),
        Key::MetaLeft => "MetaLeft".to_string(),
        Key::MetaRight => "MetaRight".to_string(),

        // Punctuation and symbols
        Key::SemiColon => "Semicolon".to_string(),
        Key::Comma => "Comma".to_string(),
        Key::Dot => "Period".to_string(),
        Key::Slash => "Slash".to_string(),
        Key::BackSlash => "Backslash".to_string(),
        Key::Quote => "Quote".to_string(),
        Key::BackQuote => "Backquote".to_string(),
        Key::LeftBracket => "BracketLeft".to_string(),
        Key::RightBracket => "BracketRight".to_string(),
        Key::Minus => "Minus".to_string(),
        Key::Equal => "Equal".to_string(),
        Key::IntlBackslash => "IntlBackslash".to_string(),

        // Unknown keys: use debug format
        Key::Unknown(code) => format!("Unknown({})", code),
        _ => format!("{:?}", key),
    }
}

