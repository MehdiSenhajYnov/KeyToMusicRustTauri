use std::collections::HashMap;

/// Read labels on the UI thread: GDK shares the compositor's keyboard map,
/// including on Wayland where navigator.keyboard is unavailable in WebKitGTK.
#[cfg(target_os = "linux")]
fn read_keymap(keymap: &gdk::Keymap) -> HashMap<String, String> {
    let group = ((keymap.modifier_state() >> 13) & 3) as i32;
    let mut labels = HashMap::new();
    for raw in 1..=86 {
        let Some(code) = super::linux_listener::keycode_to_code(evdev::KeyCode::new(raw)) else {
            continue;
        };
        // GDK/XKB keycodes have an offset of 8 relative to Linux input keycodes.
        if let Some((keyval, _, _, _)) =
            keymap.translate_keyboard_state(u32::from(raw) + 8, gdk::ModifierType::empty(), group)
        {
            if let Some(character) = gdk::keys::Key::from(keyval).to_unicode() {
                if !character.is_control() && !character.is_whitespace() {
                    labels.insert(code.to_string(), character.to_string());
                }
            }
        }
    }
    labels
}

#[cfg(target_os = "linux")]
fn current_keymap() -> Option<gdk::Keymap> {
    gdk::Display::default().and_then(|display| gdk::Keymap::for_display(&display))
}

pub async fn current(app: &tauri::AppHandle) -> Result<HashMap<String, String>, String> {
    #[cfg(target_os = "linux")]
    {
        let (tx, rx) = tokio::sync::oneshot::channel();
        app.run_on_main_thread(move || {
            let labels = current_keymap()
                .map(|keymap| read_keymap(&keymap))
                .unwrap_or_default();
            let _ = tx.send(labels);
        })
        .map_err(|error| error.to_string())?;
        rx.await.map_err(|error| error.to_string())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = app;
        Ok(HashMap::new()) // Browser Keyboard API, then persisted labels, on other platforms.
    }
}

/// Notify the UI when the system changes layout, including while the app has focus.
pub fn watch(app: &tauri::AppHandle) {
    #[cfg(target_os = "linux")]
    if let Some(keymap) = current_keymap() {
        use std::{cell::RefCell, rc::Rc};
        use tauri::Emitter;
        let previous = RefCell::new(read_keymap(&keymap));
        let app = app.clone();
        let notify = Rc::new(move |keymap: &gdk::Keymap| {
            let labels = read_keymap(keymap);
            if *previous.borrow() != labels {
                *previous.borrow_mut() = labels.clone();
                let _ = app.emit("keyboard_layout_changed", labels);
            }
        });
        let on_keys = notify.clone();
        keymap.connect_keys_changed(move |keymap| on_keys(keymap));
        keymap.connect_state_changed(move |keymap| notify(keymap));
    }
    #[cfg(not(target_os = "linux"))]
    let _ = app;
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    #[ignore = "Requires an isolated X11 display configured with setxkbmap fr"]
    fn french_system_layout_before_any_keypress() {
        gdk::init();
        let keymap = current_keymap().expect("GDK keymap");
        let labels = read_keymap(&keymap);
        assert_eq!(labels.get("KeyA").map(String::as_str), Some("q"));
        assert_eq!(labels.get("KeyQ").map(String::as_str), Some("a"));
        assert_eq!(labels.get("KeyW").map(String::as_str), Some("z"));
        assert_eq!(labels.get("KeyZ").map(String::as_str), Some("w"));
        assert_eq!(labels.get("Digit1").map(String::as_str), Some("&"));
    }
}
