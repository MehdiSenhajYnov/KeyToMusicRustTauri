use rdev::{listen, EventType};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::keys::mapping::{is_modifier, key_to_code, KeyEvent};

/// Global keyboard detector using rdev.
/// Runs in a separate thread and invokes a callback on key events.
#[derive(Clone)]
pub struct KeyDetector {
    enabled: Arc<Mutex<bool>>,
    cooldown_ms: Arc<Mutex<u32>>,
    pressed_keys: Arc<Mutex<HashSet<String>>>,
    master_stop_shortcut: Arc<Mutex<Vec<String>>>,
    key_detection_shortcut: Arc<Mutex<Vec<String>>>,
    auto_momentum_shortcut: Arc<Mutex<Vec<String>>>,
}

impl KeyDetector {
    pub fn new(cooldown_ms: u32, master_stop_shortcut: Vec<String>) -> Self {
        Self {
            enabled: Arc::new(Mutex::new(true)),
            cooldown_ms: Arc::new(Mutex::new(cooldown_ms)),
            pressed_keys: Arc::new(Mutex::new(HashSet::new())),
            master_stop_shortcut: Arc::new(Mutex::new(master_stop_shortcut)),
            key_detection_shortcut: Arc::new(Mutex::new(Vec::new())),
            auto_momentum_shortcut: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Start listening for global keyboard events in a separate thread.
    /// The callback is invoked with KeyEvent when a valid key press is detected.
    pub fn start<F>(&self, callback: F)
    where
        F: Fn(KeyEvent) + Send + Sync + 'static,
    {
        let enabled = self.enabled.clone();
        let pressed_keys = self.pressed_keys.clone();
        let master_stop_shortcut = self.master_stop_shortcut.clone();
        let key_detection_shortcut = self.key_detection_shortcut.clone();
        let auto_momentum_shortcut = self.auto_momentum_shortcut.clone();

        std::thread::spawn(move || {
            let callback = Arc::new(callback);

            listen(move |event| {
                match event.event_type {
                    EventType::KeyPress(key) => {
                        let code = key_to_code(key);

                        let mut pressed = pressed_keys.lock().unwrap();

                        // Avoid key repeat (key held down)
                        if pressed.contains(&code) {
                            return;
                        }
                        pressed.insert(code.clone());

                        // Global shortcuts: work even when key detection is disabled
                        let kd_keys = key_detection_shortcut.lock().unwrap();
                        if !kd_keys.is_empty()
                            && is_shortcut_pressed(&pressed, &kd_keys)
                        {
                            drop(pressed);
                            drop(kd_keys);
                            callback(KeyEvent::ToggleKeyDetection);
                            return;
                        }
                        drop(kd_keys);

                        let stop_keys = master_stop_shortcut.lock().unwrap();
                        if !stop_keys.is_empty()
                            && is_shortcut_pressed(&pressed, &stop_keys)
                        {
                            drop(pressed);
                            drop(stop_keys);
                            callback(KeyEvent::MasterStop);
                            return;
                        }
                        drop(stop_keys);

                        let am_keys = auto_momentum_shortcut.lock().unwrap();
                        if !am_keys.is_empty()
                            && is_shortcut_pressed(&pressed, &am_keys)
                        {
                            drop(pressed);
                            drop(am_keys);
                            callback(KeyEvent::ToggleAutoMomentum);
                            return;
                        }
                        drop(am_keys);

                        // If detection is disabled, don't trigger sound key presses
                        if !*enabled.lock().unwrap() {
                            return;
                        }

                        // Skip modifier-only presses (don't trigger sounds)
                        if is_modifier(&key) {
                            return;
                        }

                        // Detect if Shift is pressed
                        let with_shift = pressed.contains("ShiftLeft")
                            || pressed.contains("ShiftRight");

                        drop(pressed);

                        callback(KeyEvent::KeyPressed { key_code: code, with_shift });
                    }
                    EventType::KeyRelease(key) => {
                        let code = key_to_code(key);
                        pressed_keys.lock().unwrap().remove(&code);
                    }
                    _ => {}
                }
            })
            .expect("Failed to listen to keyboard events");
        });
    }

    /// Enable or disable key detection.
    pub fn set_enabled(&self, enabled: bool) {
        *self.enabled.lock().unwrap() = enabled;
    }

    /// Check if key detection is enabled.
    pub fn is_enabled(&self) -> bool {
        *self.enabled.lock().unwrap()
    }

    /// Update the cooldown duration.
    pub fn set_cooldown(&self, cooldown_ms: u32) {
        *self.cooldown_ms.lock().unwrap() = cooldown_ms;
    }

    /// Update the master stop shortcut keys.
    pub fn set_master_stop_shortcut(&self, keys: Vec<String>) {
        *self.master_stop_shortcut.lock().unwrap() = keys;
    }

    /// Update the key detection toggle shortcut.
    pub fn set_key_detection_shortcut(&self, keys: Vec<String>) {
        *self.key_detection_shortcut.lock().unwrap() = keys;
    }

    /// Update the auto momentum toggle shortcut.
    pub fn set_auto_momentum_shortcut(&self, keys: Vec<String>) {
        *self.auto_momentum_shortcut.lock().unwrap() = keys;
    }
}

/// Check if all keys in the shortcut are currently pressed.
fn is_shortcut_pressed(pressed_keys: &HashSet<String>, shortcut_keys: &[String]) -> bool {
    if shortcut_keys.is_empty() {
        return false;
    }
    shortcut_keys.iter().all(|key| pressed_keys.contains(key))
}
