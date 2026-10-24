use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::time::Duration;

use crate::keys::chord::{ChordDetectorHandle, ChordResult};
use crate::keys::mapping::KeyEvent;

#[cfg(target_os = "linux")]
fn is_wayland_session() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
        || std::env::var("XDG_SESSION_TYPE")
            .map(|value| value.eq_ignore_ascii_case("wayland"))
            .unwrap_or(false)
}

/// Check if a key code string represents a modifier key.
fn is_modifier_code(code: &str) -> bool {
    matches!(
        code,
        "ShiftLeft"
            | "ShiftRight"
            | "ControlLeft"
            | "ControlRight"
            | "AltLeft"
            | "AltRight"
            | "MetaLeft"
            | "MetaRight"
    )
}

/// Configuration fields that are read on every keypress but written rarely.
/// Consolidated into a single RwLock for efficient concurrent reads.
struct KeyDetectorConfig {
    enabled: bool,
    /// Shortcut capture in progress: every key belongs to the UI.
    suspended: bool,
    /// The user types in one of the host's text fields: sound keys stay quiet,
    /// but global shortcuts that use Ctrl/Alt/Super still work.
    typing: bool,
    cooldown_ms: u32,
    stop_all_shortcut: Vec<String>,
    pause_shortcut: Vec<String>,
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
    pub fn new(cooldown_ms: u32, stop_all_shortcut: Vec<String>, chord_window_ms: u32) -> Self {
        Self {
            config: Arc::new(RwLock::new(KeyDetectorConfig {
                enabled: true,
                suspended: false,
                typing: false,
                cooldown_ms,
                stop_all_shortcut,
                pause_shortcut: Vec::new(),
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
            #[cfg(target_os = "linux")]
            let cb_warning = callback.clone();
            let shared = Shared {
                config,
                pressed_keys,
                chord_detector,
                timer_notify: timer_notify_clone,
            };
            let handle_key_event = Arc::new(move |code: String, is_press: bool| {
                shared.process(code, is_press, &*cb);
            });

            timer_running.store(true, Ordering::SeqCst);

            #[cfg(target_os = "macos")]
            {
                use crate::keys::macos_listener::{listen_macos, MacKeyEvent};
                let handler = handle_key_event.clone();
                listen_macos(move |event| match event {
                    MacKeyEvent::Press(code) => handler(code, true),
                    MacKeyEvent::Release(code) => handler(code, false),
                });
            }

            #[cfg(target_os = "windows")]
            {
                use crate::keys::windows_listener::{listen_windows, WinKeyEvent};

                let handler = handle_key_event.clone();
                listen_windows(move |event| match event {
                    WinKeyEvent::Press(code) => handler(code, true),
                    WinKeyEvent::Release(code) => handler(code, false),
                });
            }

            #[cfg(not(any(target_os = "macos", target_os = "windows")))]
            {
                let handler = handle_key_event.clone();
                #[cfg(target_os = "linux")]
                {
                    if is_wayland_session() {
                        use crate::keys::linux_listener::{listen_linux, LinuxKeyEvent};
                        let mut warned_unavailable = false;

                        loop {
                            let handler = handler.clone();
                            match listen_linux(move |event| match event {
                                LinuxKeyEvent::Press(code) => handler(code, true),
                                LinuxKeyEvent::Release(code) => handler(code, false),
                            }) {
                                Ok(()) => {
                                    warned_unavailable = false;
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        "Linux global keyboard listener unavailable: {}",
                                        e
                                    );
                                    if !warned_unavailable {
                                        cb_warning(KeyEvent::BackendWarning {
                                            message: "Wayland background key detection needs one system permission. Open Settings > Key Detection > Background Detection to enable it automatically.".to_string(),
                                        });
                                        warned_unavailable = true;
                                    }
                                    std::thread::sleep(Duration::from_secs(5));
                                }
                            }
                        }
                    } else {
                        use crate::keys::mapping::key_to_code;
                        use rdev::{listen, EventType};

                        let handler = handler.clone();
                        if let Err(e) = listen(move |event| match event.event_type {
                            EventType::KeyPress(key) => {
                                let code = key_to_code(key);
                                handler(code, true);
                            }
                            EventType::KeyRelease(key) => {
                                let code = key_to_code(key);
                                handler(code, false);
                            }
                            _ => {}
                        }) {
                            tracing::warn!("Global keyboard listener failed: {:?}", e);
                        }
                    }
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

                let cfg = config.read().unwrap();
                if !cfg.enabled || cfg.suspended || cfg.typing {
                    continue;
                }

                drop(cfg);
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
            self.chord_detector.clear();
        }
    }

    /// Typing in a host text field: mute sound keys, keep modifier shortcuts.
    pub fn set_typing(&self, typing: bool) {
        let mut cfg = self.config.write().unwrap();
        if cfg.typing == typing {
            return;
        }
        cfg.typing = typing;
        drop(cfg);
        self.chord_detector.clear();
    }

    /// Suspend UI typing/capture without changing the persisted enabled preference.
    pub fn set_suspended(&self, suspended: bool) {
        self.config.write().unwrap().suspended = suspended;
        self.chord_detector.clear();
        self.pressed_keys.lock().unwrap().clear();
    }

    /// Update the cooldown duration.
    pub fn set_cooldown(&self, cooldown_ms: u32) {
        self.config.write().unwrap().cooldown_ms = cooldown_ms;
    }

    /// Update the stop all shortcut keys.
    pub fn set_stop_all_shortcut(&self, keys: Vec<String>) {
        self.config.write().unwrap().stop_all_shortcut = keys;
    }

    /// Update the pause/resume shortcut.
    pub fn set_pause_shortcut(&self, keys: Vec<String>) {
        self.config.write().unwrap().pause_shortcut = keys;
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

    /// Clear all tracked pressed keys and chord state.
    /// Called on window focus change to prevent stuck modifier keys
    /// (e.g. Alt remaining "pressed" after Alt+Tab).
    pub fn clear_pressed_keys(&self) {
        self.pressed_keys.lock().unwrap().clear();
        self.chord_detector.clear();
    }
}

/// State the listener threads share with the detector handle.
struct Shared {
    config: Arc<RwLock<KeyDetectorConfig>>,
    pressed_keys: Arc<Mutex<HashSet<String>>>,
    chord_detector: ChordDetectorHandle,
    timer_notify: Arc<(Mutex<bool>, Condvar)>,
}

impl Shared {
    /// Handle one physical key transition from any backend.
    fn process(&self, code: String, is_press: bool, cb: &dyn Fn(KeyEvent)) {
        if !is_press {
            self.pressed_keys.lock().unwrap().remove(&code);
            self.chord_detector.on_key_release(&code);
            return;
        }

        let mut pressed = self.pressed_keys.lock().unwrap();
        // Avoid key repeat (key held down)
        if !pressed.insert(code.clone()) {
            return;
        }

        // Single read-lock for all config fields (shortcuts + enabled)
        let cfg = self.config.read().unwrap();
        if cfg.suspended {
            return;
        }

        // Global shortcuts: work even when key detection is disabled. While the
        // user types, only shortcuts with a real modifier count, so plain
        // letters typed into a field never toggle anything.
        let shortcut_allowed = |keys: &[String]| {
            !keys.is_empty()
                && is_shortcut_pressed(&pressed, keys)
                && (!cfg.typing || keys.iter().any(|k| is_command_modifier(k)))
        };
        let shortcut = if shortcut_allowed(&cfg.key_detection_shortcut) {
            Some(KeyEvent::ToggleKeyDetection)
        } else if shortcut_allowed(&cfg.stop_all_shortcut) {
            Some(KeyEvent::StopAll)
        } else if shortcut_allowed(&cfg.pause_shortcut) {
            Some(KeyEvent::TogglePause)
        } else if shortcut_allowed(&cfg.auto_momentum_shortcut) {
            Some(KeyEvent::ToggleAutoMomentum)
        } else {
            None
        };
        if let Some(event) = shortcut {
            drop(pressed);
            drop(cfg);
            cb(event);
            return;
        }

        // If detection is disabled (or the user is typing), don't trigger sounds
        if !cfg.enabled || cfg.typing {
            return;
        }
        drop(cfg);

        // Track pressed modifiers for the with_shift flag
        let has_shift = pressed.contains("ShiftLeft") || pressed.contains("ShiftRight");
        drop(pressed);

        match self.chord_detector.on_key_press(&code) {
            ChordResult::Trigger(combo) => cb(KeyEvent::KeyPressed {
                key_code: combo,
                with_shift: has_shift,
            }),
            ChordResult::Pending => {
                // Wake the timer thread
                let (lock, cvar) = &*self.timer_notify;
                *lock.lock().unwrap() = true;
                cvar.notify_one();
            }
            ChordResult::NoMatch => {
                // No binding matches; a modifier alone never triggers anything.
                if is_modifier_code(&code) {
                    return;
                }
                // Build old-style combo for backwards compatibility
                let pressed = self.pressed_keys.lock().unwrap();
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
}

/// Ctrl, Alt or Super: keys that never produce text on their own.
fn is_command_modifier(code: &str) -> bool {
    matches!(
        code,
        "ControlLeft" | "ControlRight" | "AltLeft" | "AltRight" | "MetaLeft" | "MetaRight"
    )
}

/// Check if all keys in the shortcut are currently pressed.
fn is_shortcut_pressed(pressed_keys: &HashSet<String>, shortcut_keys: &[String]) -> bool {
    if shortcut_keys.is_empty() {
        return false;
    }
    shortcut_keys.iter().all(|key| pressed_keys.contains(key))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detector(bindings: &[&str]) -> (KeyDetector, Shared) {
        let detector = KeyDetector::new(0, vec!["AltLeft".into(), "KeyS".into()], 30);
        detector.set_key_detection_shortcut(vec!["AltLeft".into(), "KeyD".into()]);
        detector.set_auto_momentum_shortcut(vec!["AltLeft".into(), "KeyQ".into()]);
        detector.set_pause_shortcut(vec!["AltLeft".into(), "KeyP".into()]);
        let bindings: Vec<String> = bindings.iter().map(|b| b.to_string()).collect();
        detector.set_profile_bindings(&bindings);
        let shared = Shared {
            config: detector.config.clone(),
            pressed_keys: detector.pressed_keys.clone(),
            chord_detector: detector.chord_detector.clone(),
            timer_notify: detector.timer_notify.clone(),
        };
        (detector, shared)
    }

    /// Press then release each key in order, collecting emitted events.
    fn tap(shared: &Shared, keys: &[&str]) -> Vec<KeyEvent> {
        let events = std::cell::RefCell::new(Vec::new());
        let cb = |e: KeyEvent| events.borrow_mut().push(e);
        for key in keys {
            shared.process(key.to_string(), true, &cb);
        }
        for key in keys.iter().rev() {
            shared.process(key.to_string(), false, &cb);
        }
        events.into_inner()
    }

    fn names(events: &[KeyEvent]) -> Vec<String> {
        events
            .iter()
            .map(|e| match e {
                KeyEvent::KeyPressed { key_code, .. } => key_code.clone(),
                KeyEvent::StopAll => "StopAll".into(),
                KeyEvent::TogglePause => "TogglePause".into(),
                KeyEvent::ToggleKeyDetection => "ToggleKeyDetection".into(),
                KeyEvent::ToggleAutoMomentum => "ToggleAutoMomentum".into(),
                KeyEvent::BackendWarning { .. } => "Warning".into(),
            })
            .collect()
    }

    #[test]
    fn bound_key_triggers() {
        let (_d, shared) = detector(&["KeyA"]);
        assert_eq!(names(&tap(&shared, &["KeyA"])), ["KeyA"]);
    }

    #[test]
    fn held_key_does_not_repeat() {
        let (_d, shared) = detector(&["KeyA"]);
        let events = std::cell::RefCell::new(Vec::new());
        let cb = |e: KeyEvent| events.borrow_mut().push(e);
        shared.process("KeyA".into(), true, &cb);
        shared.process("KeyA".into(), true, &cb);
        assert_eq!(events.borrow().len(), 1);
    }

    #[test]
    fn shortcuts_work_while_detection_is_off() {
        let (d, shared) = detector(&["KeyA"]);
        d.set_enabled(false);
        assert!(tap(&shared, &["KeyA"]).is_empty());
        assert_eq!(
            names(&tap(&shared, &["AltLeft", "KeyD"])),
            ["ToggleKeyDetection"]
        );
        assert_eq!(
            names(&tap(&shared, &["AltLeft", "KeyQ"])),
            ["ToggleAutoMomentum"]
        );
        assert_eq!(names(&tap(&shared, &["AltLeft", "KeyS"])), ["StopAll"]);
        assert_eq!(names(&tap(&shared, &["AltLeft", "KeyP"])), ["TogglePause"]);
    }

    #[test]
    fn typing_mutes_sounds_but_keeps_modifier_shortcuts() {
        let (d, shared) = detector(&["KeyA"]);
        d.set_typing(true);
        assert!(tap(&shared, &["KeyA"]).is_empty());
        assert_eq!(
            names(&tap(&shared, &["AltLeft", "KeyQ"])),
            ["ToggleAutoMomentum"]
        );
        d.set_typing(false);
        assert_eq!(names(&tap(&shared, &["KeyA"])), ["KeyA"]);
    }

    #[test]
    fn typing_ignores_shortcuts_made_of_text_keys() {
        let (d, shared) = detector(&["KeyA"]);
        d.set_stop_all_shortcut(vec!["KeyX".into(), "KeyC".into()]);
        d.set_typing(true);
        assert!(tap(&shared, &["KeyX", "KeyC"]).is_empty());
    }

    #[test]
    fn capture_suspends_everything() {
        let (d, shared) = detector(&["KeyA"]);
        d.set_suspended(true);
        assert!(tap(&shared, &["AltLeft", "KeyD"]).is_empty());
        assert!(tap(&shared, &["KeyA"]).is_empty());
        d.set_suspended(false);
        assert_eq!(
            names(&tap(&shared, &["AltLeft", "KeyD"])),
            ["ToggleKeyDetection"]
        );
    }

    #[test]
    fn modifier_alone_triggers_nothing() {
        let (_d, shared) = detector(&["KeyA"]);
        assert!(tap(&shared, &["ShiftLeft"]).is_empty());
    }
}
