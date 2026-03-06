use evdev::{enumerate, Device, EventSummary, KeyCode};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinuxInputAccessStatus {
    pub supported: bool,
    pub session_type: String,
    pub background_detection_available: bool,
    pub can_auto_fix: bool,
    pub relogin_recommended: bool,
    pub accessible_keyboard_devices: Vec<String>,
    pub keyboard_candidates: Vec<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub enum LinuxKeyEvent {
    Press(String),
    Release(String),
}

fn is_keyboard_device(device: &Device) -> bool {
    let Some(keys) = device.supported_keys() else {
        return false;
    };

    keys.contains(KeyCode::KEY_A)
        && keys.contains(KeyCode::KEY_Z)
        && keys.contains(KeyCode::KEY_SPACE)
        && keys.contains(KeyCode::KEY_ENTER)
}

fn keyboard_candidates_from_by_path() -> Vec<PathBuf> {
    let by_path = std::path::Path::new("/dev/input/by-path");
    let Ok(entries) = std::fs::read_dir(by_path) else {
        return Vec::new();
    };

    let mut paths = HashSet::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        if !name.ends_with("-event-kbd") {
            continue;
        }

        match std::fs::canonicalize(&path) {
            Ok(target) => {
                paths.insert(target);
            }
            Err(_) => {
                paths.insert(path);
            }
        }
    }

    let mut paths: Vec<PathBuf> = paths.into_iter().collect();
    paths.sort();
    paths
}

fn accessible_keyboard_devices() -> Vec<(PathBuf, Device)> {
    enumerate()
        .filter(|(_, device)| is_keyboard_device(device))
        .collect()
}

pub fn get_linux_input_access_status() -> LinuxInputAccessStatus {
    let accessible_keyboards = accessible_keyboard_devices();
    let keyboard_candidates = keyboard_candidates_from_by_path();

    let accessible_keyboard_devices = accessible_keyboards
        .iter()
        .map(|(path, _)| path.display().to_string())
        .collect::<Vec<_>>();

    let keyboard_candidates = keyboard_candidates
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();

    if accessible_keyboard_devices.is_empty() {
        LinuxInputAccessStatus {
            supported: true,
            session_type: "wayland".to_string(),
            background_detection_available: false,
            can_auto_fix: true,
            relogin_recommended: true,
            accessible_keyboard_devices,
            keyboard_candidates: keyboard_candidates.clone(),
            message: Some(if keyboard_candidates.is_empty() {
                "Background detection is blocked because this session cannot read keyboard devices yet.".to_string()
            } else {
                format!(
                    "Background detection is blocked because this session cannot read keyboard devices yet. Keyboards found: {}.",
                    keyboard_candidates.join(", ")
                )
            }),
        }
    } else {
        LinuxInputAccessStatus {
            supported: true,
            session_type: "wayland".to_string(),
            background_detection_available: !accessible_keyboard_devices.is_empty(),
            can_auto_fix: false,
            relogin_recommended: false,
            accessible_keyboard_devices,
            keyboard_candidates,
            message: None,
        }
    }
}

fn keycode_to_code(key: KeyCode) -> Option<&'static str> {
    match key {
        KeyCode::KEY_A => Some("KeyA"),
        KeyCode::KEY_B => Some("KeyB"),
        KeyCode::KEY_C => Some("KeyC"),
        KeyCode::KEY_D => Some("KeyD"),
        KeyCode::KEY_E => Some("KeyE"),
        KeyCode::KEY_F => Some("KeyF"),
        KeyCode::KEY_G => Some("KeyG"),
        KeyCode::KEY_H => Some("KeyH"),
        KeyCode::KEY_I => Some("KeyI"),
        KeyCode::KEY_J => Some("KeyJ"),
        KeyCode::KEY_K => Some("KeyK"),
        KeyCode::KEY_L => Some("KeyL"),
        KeyCode::KEY_M => Some("KeyM"),
        KeyCode::KEY_N => Some("KeyN"),
        KeyCode::KEY_O => Some("KeyO"),
        KeyCode::KEY_P => Some("KeyP"),
        KeyCode::KEY_Q => Some("KeyQ"),
        KeyCode::KEY_R => Some("KeyR"),
        KeyCode::KEY_S => Some("KeyS"),
        KeyCode::KEY_T => Some("KeyT"),
        KeyCode::KEY_U => Some("KeyU"),
        KeyCode::KEY_V => Some("KeyV"),
        KeyCode::KEY_W => Some("KeyW"),
        KeyCode::KEY_X => Some("KeyX"),
        KeyCode::KEY_Y => Some("KeyY"),
        KeyCode::KEY_Z => Some("KeyZ"),

        KeyCode::KEY_0 => Some("Digit0"),
        KeyCode::KEY_1 => Some("Digit1"),
        KeyCode::KEY_2 => Some("Digit2"),
        KeyCode::KEY_3 => Some("Digit3"),
        KeyCode::KEY_4 => Some("Digit4"),
        KeyCode::KEY_5 => Some("Digit5"),
        KeyCode::KEY_6 => Some("Digit6"),
        KeyCode::KEY_7 => Some("Digit7"),
        KeyCode::KEY_8 => Some("Digit8"),
        KeyCode::KEY_9 => Some("Digit9"),

        KeyCode::KEY_KP0 => Some("Numpad0"),
        KeyCode::KEY_KP1 => Some("Numpad1"),
        KeyCode::KEY_KP2 => Some("Numpad2"),
        KeyCode::KEY_KP3 => Some("Numpad3"),
        KeyCode::KEY_KP4 => Some("Numpad4"),
        KeyCode::KEY_KP5 => Some("Numpad5"),
        KeyCode::KEY_KP6 => Some("Numpad6"),
        KeyCode::KEY_KP7 => Some("Numpad7"),
        KeyCode::KEY_KP8 => Some("Numpad8"),
        KeyCode::KEY_KP9 => Some("Numpad9"),
        KeyCode::KEY_KPASTERISK => Some("NumpadMultiply"),
        KeyCode::KEY_KPPLUS => Some("NumpadAdd"),
        KeyCode::KEY_KPMINUS => Some("NumpadSubtract"),
        KeyCode::KEY_KPDOT => Some("NumpadDecimal"),
        KeyCode::KEY_KPSLASH => Some("NumpadDivide"),
        KeyCode::KEY_KPENTER => Some("NumpadEnter"),

        KeyCode::KEY_F1 => Some("F1"),
        KeyCode::KEY_F2 => Some("F2"),
        KeyCode::KEY_F3 => Some("F3"),
        KeyCode::KEY_F4 => Some("F4"),
        KeyCode::KEY_F5 => Some("F5"),
        KeyCode::KEY_F6 => Some("F6"),
        KeyCode::KEY_F7 => Some("F7"),
        KeyCode::KEY_F8 => Some("F8"),
        KeyCode::KEY_F9 => Some("F9"),
        KeyCode::KEY_F10 => Some("F10"),
        KeyCode::KEY_F11 => Some("F11"),
        KeyCode::KEY_F12 => Some("F12"),

        KeyCode::KEY_UP => Some("ArrowUp"),
        KeyCode::KEY_DOWN => Some("ArrowDown"),
        KeyCode::KEY_LEFT => Some("ArrowLeft"),
        KeyCode::KEY_RIGHT => Some("ArrowRight"),

        KeyCode::KEY_SPACE => Some("Space"),
        KeyCode::KEY_ENTER => Some("Enter"),
        KeyCode::KEY_TAB => Some("Tab"),
        KeyCode::KEY_ESC => Some("Escape"),
        KeyCode::KEY_BACKSPACE => Some("Backspace"),
        KeyCode::KEY_DELETE => Some("Delete"),
        KeyCode::KEY_INSERT => Some("Insert"),
        KeyCode::KEY_HOME => Some("Home"),
        KeyCode::KEY_END => Some("End"),
        KeyCode::KEY_PAGEUP => Some("PageUp"),
        KeyCode::KEY_PAGEDOWN => Some("PageDown"),
        KeyCode::KEY_CAPSLOCK => Some("CapsLock"),
        KeyCode::KEY_NUMLOCK => Some("NumLock"),
        KeyCode::KEY_SCROLLLOCK => Some("ScrollLock"),
        KeyCode::KEY_SYSRQ | KeyCode::KEY_PRINT => Some("PrintScreen"),
        KeyCode::KEY_PAUSE => Some("Pause"),

        KeyCode::KEY_LEFTSHIFT => Some("ShiftLeft"),
        KeyCode::KEY_RIGHTSHIFT => Some("ShiftRight"),
        KeyCode::KEY_LEFTCTRL => Some("ControlLeft"),
        KeyCode::KEY_RIGHTCTRL => Some("ControlRight"),
        KeyCode::KEY_LEFTALT => Some("AltLeft"),
        KeyCode::KEY_RIGHTALT => Some("AltRight"),
        KeyCode::KEY_LEFTMETA => Some("MetaLeft"),
        KeyCode::KEY_RIGHTMETA => Some("MetaRight"),

        KeyCode::KEY_SEMICOLON => Some("Semicolon"),
        KeyCode::KEY_COMMA => Some("Comma"),
        KeyCode::KEY_DOT => Some("Period"),
        KeyCode::KEY_SLASH => Some("Slash"),
        KeyCode::KEY_BACKSLASH => Some("Backslash"),
        KeyCode::KEY_APOSTROPHE => Some("Quote"),
        KeyCode::KEY_GRAVE => Some("Backquote"),
        KeyCode::KEY_LEFTBRACE => Some("BracketLeft"),
        KeyCode::KEY_RIGHTBRACE => Some("BracketRight"),
        KeyCode::KEY_MINUS => Some("Minus"),
        KeyCode::KEY_EQUAL => Some("Equal"),
        KeyCode::KEY_102ND => Some("IntlBackslash"),
        KeyCode::KEY_MENU | KeyCode::KEY_COMPOSE => Some("ContextMenu"),

        _ => None,
    }
}

pub fn listen_linux<T>(callback: T) -> Result<(), String>
where
    T: Fn(LinuxKeyEvent) + Send + Sync + 'static,
{
    let accessible_keyboards = accessible_keyboard_devices();

    if accessible_keyboards.is_empty() {
        let candidates = keyboard_candidates_from_by_path();
        if candidates.is_empty() {
            return Err("No readable keyboard input devices found under /dev/input".to_string());
        }

        let candidate_list = candidates
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");

        return Err(format!(
            "No accessible keyboard input devices. Wayland global key detection requires read access to /dev/input/event*. Candidates: {}. Add the user to the 'input' group or install a udev rule, then restart the session.",
            candidate_list
        ));
    }

    let callback = Arc::new(callback);
    let mut handles = Vec::new();

    for (path, mut device) in accessible_keyboards {
        let callback = callback.clone();
        let path_for_log = path.clone();
        let device_name = device.name().unwrap_or("unknown keyboard").to_string();
        tracing::info!(
            "Starting Linux evdev keyboard listener on {} ({})",
            path.display(),
            device_name
        );

        let handle = std::thread::spawn(move || loop {
            match device.fetch_events() {
                Ok(events) => {
                    for event in events {
                        if let EventSummary::Key(_, key, value) = event.destructure() {
                            let Some(code) = keycode_to_code(key) else {
                                continue;
                            };

                            match value {
                                1 => callback(LinuxKeyEvent::Press(code.to_string())),
                                0 => callback(LinuxKeyEvent::Release(code.to_string())),
                                _ => {}
                            }
                        }
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        "Linux evdev keyboard listener stopped on {}: {}",
                        path_for_log.display(),
                        err
                    );
                    break;
                }
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.join();
    }

    Ok(())
}
