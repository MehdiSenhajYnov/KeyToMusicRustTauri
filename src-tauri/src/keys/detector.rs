use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock};
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

/// Configuration fields that are read on every keypress but written rarely.
/// Consolidated into a single RwLock for efficient concurrent reads.
struct KeyDetectorConfig {
    enabled: bool,
    cooldown_ms: u32,
    master_stop_shortcut: Vec<String>,
    key_detection_shortcut: Vec<String>,
    auto_momentum_shortcut: Vec<String>,
}

/// Global keyboard detector.
/// Runs in a separate thread and invokes a callback on key events.
/// On macOS: uses a custom CGEventTap (avoids rdev crash on macOS 13+).
/// On other platforms: uses rdev::listen.
#[derive(Clone)]
pub struct KeyDetector {
    config: Arc<RwLock<KeyDetectorConfig>>,
    pressed_keys: Arc<Mutex<HashSet<String>>>,
    chord_detector: ChordDetectorHandle,
    /// Flag to signal the timer thread to stop
    timer_running: Arc<AtomicBool>,
    /// Condvar to wake the timer thread when a chord is pending
    timer_notify: Arc<(Mutex<bool>, Condvar)>,
}

impl KeyDetector {
    pub fn new(cooldown_ms: u32, master_stop_shortcut: Vec<String>, chord_window_ms: u32) -> Self {
        Self {
            config: Arc::new(RwLock::new(KeyDetectorConfig {
                enabled: true,
                cooldown_ms,
                master_stop_shortcut,
                key_detection_shortcut: Vec::new(),
                auto_momentum_shortcut: Vec::new(),
            })),
            pressed_keys: Arc::new(Mutex::new(HashSet::new())),
            chord_detector: ChordDetectorHandle::new(chord_window_ms),
            timer_running: Arc::new(AtomicBool::new(false)),
            timer_notify: Arc::new((Mutex::new(false), Condvar::new())),
        }
    }

    /// Start listening for global keyboard events in a separate thread.
    /// The callback is invoked with KeyEvent when a valid key press is detected.
    pub fn start<F>(&self, callback: F)
    where
        F: Fn(KeyEvent) + Send + Sync + 'static,
    {
        let config = self.config.clone();
        let pressed_keys = self.pressed_keys.clone();
        let chord_detector = self.chord_detector.clone();
        let timer_running = self.timer_running.clone();
        let timer_notify_clone = self.timer_notify.clone();

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

                    // Single read-lock for all config fields (shortcuts + enabled)
                    let cfg = config.read().unwrap();

                    // Global shortcuts: work even when key detection is disabled
                    if !cfg.key_detection_shortcut.is_empty()
                        && is_shortcut_pressed(&pressed, &cfg.key_detection_shortcut)
                    {
                        drop(pressed);
                        drop(cfg);
                        cb(KeyEvent::ToggleKeyDetection);
                        return;
                    }

                    if !cfg.master_stop_shortcut.is_empty()
                        && is_shortcut_pressed(&pressed, &cfg.master_stop_shortcut)
                    {
                        drop(pressed);
                        drop(cfg);
                        cb(KeyEvent::MasterStop);
                        return;
                    }

                    if !cfg.auto_momentum_shortcut.is_empty()
                        && is_shortcut_pressed(&pressed, &cfg.auto_momentum_shortcut)
                    {
                        drop(pressed);
                        drop(cfg);
                        cb(KeyEvent::ToggleAutoMomentum);
                        return;
                    }

                    // If detection is disabled, don't trigger sound key presses
                    if !cfg.enabled {
                        return;
                    }

                    drop(cfg);

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
                            // Wake the timer thread
                            let (lock, cvar) = &*timer_notify_clone;
                            let mut notified = lock.lock().unwrap();
                            *notified = true;
                            cvar.notify_one();
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

            #[cfg(target_os = "windows")]
            {
                use crate::keys::windows_listener::{listen_windows, WinKeyEvent};

                let handler = handle_key_event;
                listen_windows(move |event| {
                    match event {
                        WinKeyEvent::Press(code) => handler(code, true),
                        WinKeyEvent::Release(code) => handler(code, false),
                    }
                });
            }

            #[cfg(not(any(target_os = "macos", target_os = "windows")))]
            {
                use rdev::{listen, EventType};
                use crate::keys::mapping::key_to_code;

                let handler = handle_key_event;
                if let Err(e) = listen(move |event| {
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
                }) {
                    tracing::warn!("Global keyboard listener failed: {:?}", e);
                }
            }
        });
    }

    /// Start a thread that waits for pending chord timer expiration using Condvar.
    /// Instead of polling every 5ms, it sleeps until notified by a key press
    /// or until the chord window timeout expires.
    fn start_timer_thread<F>(&self, callback: Arc<F>)
    where
        F: Fn(KeyEvent) + Send + Sync + 'static,
    {
        let chord_detector = self.chord_detector.clone();
        let config = self.config.clone();
        let pressed_keys = self.pressed_keys.clone();
        let timer_running = self.timer_running.clone();
        let timer_notify = self.timer_notify.clone();

        std::thread::spawn(move || {
            // Wait for the main detector to start
            while !timer_running.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(10));
            }

            let (lock, cvar) = &*timer_notify;
            loop {
                // Wait until notified or timeout (chord window + margin)
                let guard = lock.lock().unwrap();
                let result = cvar.wait_timeout(guard, Duration::from_millis(50)).unwrap();
                let mut guard = result.0;
                *guard = false;

                if !config.read().unwrap().enabled {
                    continue;
                }

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
        self.config.write().unwrap().enabled = enabled;
        if !enabled {
            // Clear chord detector state when disabled
            self.chord_detector.clear();
        }
    }

    /// Update the cooldown duration.
    pub fn set_cooldown(&self, cooldown_ms: u32) {
        self.config.write().unwrap().cooldown_ms = cooldown_ms;
    }

    /// Update the master stop shortcut keys.
    pub fn set_master_stop_shortcut(&self, keys: Vec<String>) {
        self.config.write().unwrap().master_stop_shortcut = keys;
    }

    /// Update the key detection toggle shortcut.
    pub fn set_key_detection_shortcut(&self, keys: Vec<String>) {
        self.config.write().unwrap().key_detection_shortcut = keys;
    }

    /// Update the auto momentum toggle shortcut.
    pub fn set_auto_momentum_shortcut(&self, keys: Vec<String>) {
        self.config.write().unwrap().auto_momentum_shortcut = keys;
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
