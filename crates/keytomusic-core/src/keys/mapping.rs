#[cfg(not(any(target_os = "macos", target_os = "windows")))]
use rdev::Key;

/// Events emitted by the key detection system.
#[derive(Debug, Clone)]
pub enum KeyEvent {
    KeyPressed { key_code: String, with_shift: bool },
    StopAll,
    ToggleKeyDetection,
    ToggleAutoMomentum,
    BackendWarning { message: String },
}

/// Convert an rdev::Key to a string key code matching Web KeyboardEvent.code format.
/// Only used on Linux (Windows and macOS have their own implementations).
/// Returns &'static str for known keys (zero allocation) or owned String for unknown keys.
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn key_to_code(key: Key) -> String {
    let static_code: Option<&'static str> = match key {
        // Letters A-Z
        Key::KeyA => Some("KeyA"),
        Key::KeyB => Some("KeyB"),
        Key::KeyC => Some("KeyC"),
        Key::KeyD => Some("KeyD"),
        Key::KeyE => Some("KeyE"),
        Key::KeyF => Some("KeyF"),
        Key::KeyG => Some("KeyG"),
        Key::KeyH => Some("KeyH"),
        Key::KeyI => Some("KeyI"),
        Key::KeyJ => Some("KeyJ"),
        Key::KeyK => Some("KeyK"),
        Key::KeyL => Some("KeyL"),
        Key::KeyM => Some("KeyM"),
        Key::KeyN => Some("KeyN"),
        Key::KeyO => Some("KeyO"),
        Key::KeyP => Some("KeyP"),
        Key::KeyQ => Some("KeyQ"),
        Key::KeyR => Some("KeyR"),
        Key::KeyS => Some("KeyS"),
        Key::KeyT => Some("KeyT"),
        Key::KeyU => Some("KeyU"),
        Key::KeyV => Some("KeyV"),
        Key::KeyW => Some("KeyW"),
        Key::KeyX => Some("KeyX"),
        Key::KeyY => Some("KeyY"),
        Key::KeyZ => Some("KeyZ"),

        // Digits 0-9
        Key::Num0 => Some("Digit0"),
        Key::Num1 => Some("Digit1"),
        Key::Num2 => Some("Digit2"),
        Key::Num3 => Some("Digit3"),
        Key::Num4 => Some("Digit4"),
        Key::Num5 => Some("Digit5"),
        Key::Num6 => Some("Digit6"),
        Key::Num7 => Some("Digit7"),
        Key::Num8 => Some("Digit8"),
        Key::Num9 => Some("Digit9"),

        // Numpad
        Key::Kp0 => Some("Numpad0"),
        Key::Kp1 => Some("Numpad1"),
        Key::Kp2 => Some("Numpad2"),
        Key::Kp3 => Some("Numpad3"),
        Key::Kp4 => Some("Numpad4"),
        Key::Kp5 => Some("Numpad5"),
        Key::Kp6 => Some("Numpad6"),
        Key::Kp7 => Some("Numpad7"),
        Key::Kp8 => Some("Numpad8"),
        Key::Kp9 => Some("Numpad9"),
        Key::KpMultiply => Some("NumpadMultiply"),
        Key::KpPlus => Some("NumpadAdd"),
        Key::KpMinus => Some("NumpadSubtract"),
        Key::KpDecimal => Some("NumpadDecimal"),
        Key::KpDivide => Some("NumpadDivide"),
        Key::KpReturn => Some("NumpadEnter"),

        // Function keys
        Key::F1 => Some("F1"),
        Key::F2 => Some("F2"),
        Key::F3 => Some("F3"),
        Key::F4 => Some("F4"),
        Key::F5 => Some("F5"),
        Key::F6 => Some("F6"),
        Key::F7 => Some("F7"),
        Key::F8 => Some("F8"),
        Key::F9 => Some("F9"),
        Key::F10 => Some("F10"),
        Key::F11 => Some("F11"),
        Key::F12 => Some("F12"),

        // Arrow keys
        Key::UpArrow => Some("ArrowUp"),
        Key::DownArrow => Some("ArrowDown"),
        Key::LeftArrow => Some("ArrowLeft"),
        Key::RightArrow => Some("ArrowRight"),

        // Special keys
        Key::Space => Some("Space"),
        Key::Return => Some("Enter"),
        Key::Tab => Some("Tab"),
        Key::Escape => Some("Escape"),
        Key::Backspace => Some("Backspace"),
        Key::Delete => Some("Delete"),
        Key::Insert => Some("Insert"),
        Key::Home => Some("Home"),
        Key::End => Some("End"),
        Key::PageUp => Some("PageUp"),
        Key::PageDown => Some("PageDown"),
        Key::CapsLock => Some("CapsLock"),
        Key::NumLock => Some("NumLock"),
        Key::ScrollLock => Some("ScrollLock"),
        Key::PrintScreen => Some("PrintScreen"),
        Key::Pause => Some("Pause"),

        // Modifiers
        Key::ShiftLeft => Some("ShiftLeft"),
        Key::ShiftRight => Some("ShiftRight"),
        Key::ControlLeft => Some("ControlLeft"),
        Key::ControlRight => Some("ControlRight"),
        Key::Alt => Some("AltLeft"),
        Key::AltGr => Some("AltRight"),
        Key::MetaLeft => Some("MetaLeft"),
        Key::MetaRight => Some("MetaRight"),

        // Punctuation and symbols
        Key::SemiColon => Some("Semicolon"),
        Key::Comma => Some("Comma"),
        Key::Dot => Some("Period"),
        Key::Slash => Some("Slash"),
        Key::BackSlash => Some("Backslash"),
        Key::Quote => Some("Quote"),
        Key::BackQuote => Some("Backquote"),
        Key::LeftBracket => Some("BracketLeft"),
        Key::RightBracket => Some("BracketRight"),
        Key::Minus => Some("Minus"),
        Key::Equal => Some("Equal"),
        Key::IntlBackslash => Some("IntlBackslash"),

        _ => None,
    };

    match static_code {
        Some(s) => s.to_string(),
        None => match key {
            Key::Unknown(code) => format!("Unknown({})", code),
            _ => format!("{:?}", key),
        },
    }
}
