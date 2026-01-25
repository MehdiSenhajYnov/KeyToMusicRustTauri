use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::keys::chord::{ChordDetectorHandle, ChordResult};
use crate::keys::mapping::KeyEvent;

/// Check if a key code string represents a modifier key.
fn is_modifier_code(code: &str) -> bool {
    matches!(
        code,
        "ShiftLeft" | "ShiftRight" | "ControlLeft" | "ControlRight"
            | "AltLeft" | "AltRight" | "MetaLeft" | "MetaRight"
    )
}

/// Global keyboard detector.
/// Runs in a separate thread and invokes a callback on key events.
/// On macOS: uses a custom CGEventTap (avoids rdev crash on macOS 13+).
/// On other platforms: uses rdev::listen.
#[derive(Clone)]
pub struct KeyDetector {
    enabled: Arc<Mutex<bool>>,
    cooldown_ms: Arc<Mutex<u32>>,
    pressed_keys: Arc<Mutex<HashSet<String>>>,
    master_stop_shortcut: Arc<Mutex<Vec<String>>>,
    key_detection_shortcut: Arc<Mutex<Vec<String>>>,
    auto_momentum_shortcut: Arc<Mutex<Vec<String>>>,
    chord_detector: ChordDetectorHandle,
    /// Flag to signal the timer thread to stop
    timer_running: Arc<AtomicBool>,
}

impl KeyDetector {
    pub fn new(cooldown_ms: u32, master_stop_shortcut: Vec<String>, chord_window_ms: u32) -> Self {
        Self {
            enabled: Arc::new(Mutex::new(true)),
            cooldown_ms: Arc::new(Mutex::new(cooldown_ms)),
            pressed_keys: Arc::new(Mutex::new(HashSet::new())),
            master_stop_shortcut: Arc::new(Mutex::new(master_stop_shortcut)),
            key_detection_shortcut: Arc::new(Mutex::new(Vec::new())),
            auto_momentum_shortcut: Arc::new(Mutex::new(Vec::new())),
            chord_detector: ChordDetectorHandle::new(chord_window_ms),
            timer_running: Arc::new(AtomicBool::new(false)),
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
        let chord_detector = self.chord_detector.clone();
        let timer_running = self.timer_running.clone();

        let callback = Arc::new(callback);

        // Start the timer polling thread
        self.start_timer_thread(callback.clone());

        std::thread::spawn(move || {
            let cb = callback.clone();
            let handle_key_event = move |code: String, is_press: bool| {
                if is_press {
                    let mut pressed = pressed_keys.lock().unwrap();

                    // Avoid key repeat (key held down)
                    if pressed.contains(&code) {
                        return;
                    }
                    pressed.insert(code.clone());

                    // Global shortcuts: work even when key detection is disabled
                    let kd_keys = key_detection_shortcut.lock().unwrap();
                    if !kd_keys.is_empty() && is_shortcut_pressed(&pressed, &kd_keys) {
                        drop(pressed);
                        drop(kd_keys);
                        cb(KeyEvent::ToggleKeyDetection);
                        return;
                    }
                    drop(kd_keys);

                    let stop_keys = master_stop_shortcut.lock().unwrap();
                    if !stop_keys.is_empty() && is_shortcut_pressed(&pressed, &stop_keys) {
                        drop(pressed);
                        drop(stop_keys);
                        cb(KeyEvent::MasterStop);
                        return;
                    }
                    drop(stop_keys);

                    let am_keys = auto_momentum_shortcut.lock().unwrap();
                    if !am_keys.is_empty() && is_shortcut_pressed(&pressed, &am_keys) {
                        drop(pressed);
                        drop(am_keys);
                        cb(KeyEvent::ToggleAutoMomentum);
                        return;
                    }
                    drop(am_keys);

                    // If detection is disabled, don't trigger sound key presses
                    if !*enabled.lock().unwrap() {
                        return;
                    }

                    // Track pressed modifiers for the with_shift flag
                    let has_shift = pressed.contains("ShiftLeft") || pressed.contains("ShiftRight");
                    drop(pressed);

                    // Use chord detector for key press
                    match chord_detector.on_key_press(&code) {
                        ChordResult::Trigger(combo) => {
                            cb(KeyEvent::KeyPressed {
                                key_code: combo,
                                with_shift: has_shift,
                            });
                        }
                        ChordResult::Pending => {
                            // Timer thread will handle it
                        }
                        ChordResult::NoMatch => {
                            // No binding matches - if it's a modifier-only press, ignore
                            // Otherwise, we could still emit for fallback handling
                            if !is_modifier_code(&code) {
                                // Build old-style combo for backwards compatibility
                                let pressed = pressed_keys.lock().unwrap();
                                let has_ctrl = pressed.contains("ControlLeft") || pressed.contains("ControlRight");
                                let has_alt = pressed.contains("AltLeft") || pressed.contains("AltRight");
                                let has_shift_now = pressed.contains("ShiftLeft") || pressed.contains("ShiftRight");
                                drop(pressed);

                                let mut combo = String::new();
                                if has_ctrl {
                                    combo.push_str("Ctrl+");
                                }
                                if has_shift_now {
                                    combo.push_str("Shift+");
                                }
                                if has_alt {
                                    combo.push_str("Alt+");
                                }
                                combo.push_str(&code);

                                cb(KeyEvent::KeyPressed {
                                    key_code: combo,
                                    with_shift: has_shift_now,
                                });
                            }
                        }
                    }
                } else {
                    // Key release
                    pressed_keys.lock().unwrap().remove(&code);
                    chord_detector.on_key_release(&code);
                }
            };

            timer_running.store(true, Ordering::SeqCst);

            #[cfg(target_os = "macos")]
            {
                use crate::keys::macos_listener::{listen_macos, MacKeyEvent};
                let handler = handle_key_event;
                listen_macos(move |event| {
                    match event {
                        MacKeyEvent::Press(code) => handler(code, true),
                        MacKeyEvent::Release(code) => handler(code, false),
                    }
                });
            }

            #[cfg(not(target_os = "macos"))]
            {
                use rdev::{listen, EventType};
                use crate::keys::mapping::key_to_code;

                let handler = handle_key_event;
                listen(move |event| {
                    match event.event_type {
                        EventType::KeyPress(key) => {
                            let code = key_to_code(key);
                            handler(code, true);
                        }
                        EventType::KeyRelease(key) => {
                            let code = key_to_code(key);
                            handler(code, false);
                        }
                        _ => {}
                    }
                })
                .expect("Failed to listen to keyboard events");
            }
        });
    }

    /// Start a thread that polls for pending chord timer expiration
    fn start_timer_thread<F>(&self, callback: Arc<F>)
    where
        F: Fn(KeyEvent) + Send + Sync + 'static,
    {
        let chord_detector = self.chord_detector.clone();
        let enabled = self.enabled.clone();
        let pressed_keys = self.pressed_keys.clone();
        let timer_running = self.timer_running.clone();

        std::thread::spawn(move || {
            // Wait for the main detector to start
            while !timer_running.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(10));
            }

            loop {
                // Poll every 5ms for responsive timing
                std::thread::sleep(Duration::from_millis(5));

                // Check if detection is enabled
                if !*enabled.lock().unwrap() {
                    continue;
                }

                // Check if timer has expired
                if let Some(combo) = chord_detector.check_timer() {
                    let pressed = pressed_keys.lock().unwrap();
                    let has_shift = pressed.contains("ShiftLeft") || pressed.contains("ShiftRight");
                    drop(pressed);

                    callback(KeyEvent::KeyPressed {
                        key_code: combo,
                        with_shift: has_shift,
                    });
                }
            }
        });
    }

    /// Enable or disable key detection.
    pub fn set_enabled(&self, enabled: bool) {
        *self.enabled.lock().unwrap() = enabled;
        if !enabled {
            // Clear chord detector state when disabled
            self.chord_detector.clear();
        }
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

    /// Update the profile bindings for chord detection.
    /// This rebuilds the Trie used for multi-key chord detection.
    pub fn set_profile_bindings(&self, bindings: &[String]) {
        self.chord_detector.set_bindings(bindings);
    }

    /// Update the chord window duration.
    pub fn set_chord_window(&self, ms: u32) {
        self.chord_detector.set_chord_window(ms);
    }
}

/// Check if all keys in the shortcut are currently pressed.
fn is_shortcut_pressed(pressed_keys: &HashSet<String>, shortcut_keys: &[String]) -> bool {
    if shortcut_keys.is_empty() {
        return false;
    }
    shortcut_keys.iter().all(|key| pressed_keys.contains(key))
}
