//! Custom macOS global key listener using CGEventTap (pure FFI).
//! Replaces rdev::listen on macOS to avoid calling TSMGetInputSourceProperty
//! from a background thread, which crashes on macOS 13+.
//! The CGEventTap still works globally (when app is in background).

use std::os::raw::c_void;

/// Key event types emitted by the macOS listener.
#[derive(Debug, Clone)]
pub enum MacKeyEvent {
    Press(String),
    Release(String),
}

// --- CoreFoundation / CoreGraphics FFI types ---

type CFRunLoopRef = *mut c_void;
type CFRunLoopSourceRef = *mut c_void;
type CFMachPortRef = *mut c_void;
type CFAllocatorRef = *const c_void;
type CFStringRef = *const c_void;
type CGEventRef = *mut c_void;
type CGEventMask = u64;

#[repr(u32)]
#[allow(dead_code)]
enum CGEventTapLocation {
    HID = 0,
}

#[repr(u32)]
#[allow(dead_code)]
enum CGEventTapPlacement {
    HeadInsert = 0,
}

#[repr(u32)]
#[allow(dead_code)]
enum CGEventTapOptions {
    ListenOnly = 1,
}

#[allow(dead_code)]
const KCG_EVENT_KEY_DOWN: u32 = 10;
#[allow(dead_code)]
const KCG_EVENT_KEY_UP: u32 = 11;
const KCG_EVENT_FLAGS_CHANGED: u32 = 12;

const KCG_KEYBOARD_EVENT_KEYCODE: u32 = 9;

type CGEventTapCallBack = unsafe extern "C" fn(
    proxy: *mut c_void,
    event_type: u32,
    event: CGEventRef,
    user_info: *mut c_void,
) -> CGEventRef;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventTapCreate(
        tap: u32,     // CGEventTapLocation
        place: u32,   // CGEventTapPlacement
        options: u32, // CGEventTapOptions
        events_of_interest: CGEventMask,
        callback: CGEventTapCallBack,
        user_info: *mut c_void,
    ) -> CFMachPortRef;

    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);

    fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
    fn CGEventGetFlags(event: CGEventRef) -> u64;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFMachPortCreateRunLoopSource(
        allocator: CFAllocatorRef,
        port: CFMachPortRef,
        order: i64,
    ) -> CFRunLoopSourceRef;

    fn CFRunLoopGetCurrent() -> CFRunLoopRef;
    fn CFRunLoopAddSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFStringRef);
    fn CFRunLoopRun();
    fn CFRelease(cf: *const c_void);

    static kCFRunLoopCommonModes: CFStringRef;
}

// --- Callback context ---

struct ListenerContext {
    callback: Box<dyn Fn(MacKeyEvent) + Send + Sync>,
    prev_flags: u64,
}

/// Start listening for global keyboard events using CGEventTap.
/// This function blocks the current thread (runs a CFRunLoop).
/// Call this from a spawned thread.
pub fn listen_macos<F>(callback: F)
where
    F: Fn(MacKeyEvent) + Send + Sync + 'static,
{
    unsafe {
        let context = Box::new(ListenerContext {
            callback: Box::new(callback),
            prev_flags: 0,
        });
        let context_ptr = Box::into_raw(context) as *mut c_void;

        // Listen to keyDown, keyUp, and flagsChanged events
        let event_mask: CGEventMask =
            (1 << KCG_EVENT_KEY_DOWN) | (1 << KCG_EVENT_KEY_UP) | (1 << KCG_EVENT_FLAGS_CHANGED);

        let tap = CGEventTapCreate(
            0, // HID
            0, // HeadInsert
            1, // ListenOnly
            event_mask,
            event_callback,
            context_ptr,
        );

        if tap.is_null() {
            eprintln!("[macos_listener] Failed to create CGEventTap. Enable Accessibility permissions in System Settings > Privacy & Security > Accessibility.");
            let _ = Box::from_raw(context_ptr as *mut ListenerContext);
            return;
        }

        let source = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
        if source.is_null() {
            eprintln!("[macos_listener] Failed to create run loop source");
            CFRelease(tap as *const c_void);
            let _ = Box::from_raw(context_ptr as *mut ListenerContext);
            return;
        }

        let run_loop = CFRunLoopGetCurrent();
        CFRunLoopAddSource(run_loop, source, kCFRunLoopCommonModes);
        CGEventTapEnable(tap, true);

        // Block this thread on the run loop
        CFRunLoopRun();

        // Cleanup (unreachable in normal operation)
        CFRelease(source as *const c_void);
        CFRelease(tap as *const c_void);
        let _ = Box::from_raw(context_ptr as *mut ListenerContext);
    }
}

/// CGEventTap callback
unsafe extern "C" fn event_callback(
    _proxy: *mut c_void,
    event_type: u32,
    event: CGEventRef,
    user_info: *mut c_void,
) -> CGEventRef {
    let context = &mut *(user_info as *mut ListenerContext);
    let keycode = CGEventGetIntegerValueField(event, KCG_KEYBOARD_EVENT_KEYCODE) as u16;

    match event_type {
        KCG_EVENT_KEY_DOWN => {
            if let Some(code) = keycode_to_string(keycode) {
                (context.callback)(MacKeyEvent::Press(code));
            }
        }
        KCG_EVENT_KEY_UP => {
            if let Some(code) = keycode_to_string(keycode) {
                (context.callback)(MacKeyEvent::Release(code));
            }
        }
        KCG_EVENT_FLAGS_CHANGED => {
            handle_flags_changed(context, keycode, CGEventGetFlags(event));
        }
        _ => {}
    }

    event
}

/// Handle modifier key press/release via flag changes.
fn handle_flags_changed(context: &mut ListenerContext, keycode: u16, flags: u64) {
    let code = match keycode {
        0x38 => "ShiftLeft",
        0x3C => "ShiftRight",
        0x3B => "ControlLeft",
        0x3E => "ControlRight",
        0x3A => "AltLeft",
        0x3D => "AltRight",
        0x37 => "MetaLeft",
        0x36 => "MetaRight",
        0x39 => "CapsLock",
        _ => return,
    };

    let modifier_bit: u64 = match keycode {
        0x38 | 0x3C => 0x020000, // Shift
        0x3B | 0x3E => 0x040000, // Control
        0x3A | 0x3D => 0x080000, // Option/Alt
        0x37 | 0x36 => 0x100000, // Command/Meta
        0x39 => 0x010000,        // CapsLock
        _ => return,
    };

    let is_pressed = (flags & modifier_bit) != 0;
    context.prev_flags = flags;

    if is_pressed {
        (context.callback)(MacKeyEvent::Press(code.to_string()));
    } else {
        (context.callback)(MacKeyEvent::Release(code.to_string()));
    }
}

/// Map macOS virtual keycode to Web KeyboardEvent.code string.
/// These are hardware position codes, independent of keyboard layout.
fn keycode_to_string(keycode: u16) -> Option<String> {
    let code = match keycode {
        // Letters
        0x00 => "KeyA",
        0x01 => "KeyS",
        0x02 => "KeyD",
        0x03 => "KeyF",
        0x04 => "KeyH",
        0x05 => "KeyG",
        0x06 => "KeyZ",
        0x07 => "KeyX",
        0x08 => "KeyC",
        0x09 => "KeyV",
        0x0B => "KeyB",
        0x0C => "KeyQ",
        0x0D => "KeyW",
        0x0E => "KeyE",
        0x0F => "KeyR",
        0x10 => "KeyY",
        0x11 => "KeyT",
        0x1F => "KeyO",
        0x20 => "KeyU",
        0x22 => "KeyI",
        0x23 => "KeyP",
        0x25 => "KeyL",
        0x26 => "KeyJ",
        0x28 => "KeyK",
        0x2D => "KeyN",
        0x2E => "KeyM",

        // Digits
        0x12 => "Digit1",
        0x13 => "Digit2",
        0x14 => "Digit3",
        0x15 => "Digit4",
        0x16 => "Digit6",
        0x17 => "Digit5",
        0x19 => "Digit9",
        0x1A => "Digit7",
        0x1C => "Digit8",
        0x1D => "Digit0",

        // Punctuation/Symbols
        0x18 => "Equal",
        0x1B => "Minus",
        0x1E => "BracketRight",
        0x21 => "BracketLeft",
        0x27 => "Quote",
        0x29 => "Semicolon",
        0x2A => "Backslash",
        0x2B => "Comma",
        0x2C => "Slash",
        0x2F => "Period",
        0x32 => "Backquote",
        0x0A => "IntlBackslash",

        // Special keys
        0x24 => "Enter",
        0x30 => "Tab",
        0x31 => "Space",
        0x33 => "Backspace",
        0x35 => "Escape",

        // Modifiers (also emitted via FlagsChanged)
        0x38 => "ShiftLeft",
        0x3C => "ShiftRight",
        0x3B => "ControlLeft",
        0x3E => "ControlRight",
        0x3A => "AltLeft",
        0x3D => "AltRight",
        0x37 => "MetaLeft",
        0x36 => "MetaRight",
        0x39 => "CapsLock",

        // Function keys
        0x7A => "F1",
        0x78 => "F2",
        0x63 => "F3",
        0x76 => "F4",
        0x60 => "F5",
        0x61 => "F6",
        0x62 => "F7",
        0x64 => "F8",
        0x65 => "F9",
        0x6D => "F10",
        0x67 => "F11",
        0x6F => "F12",

        // Arrow keys
        0x7B => "ArrowLeft",
        0x7C => "ArrowRight",
        0x7D => "ArrowDown",
        0x7E => "ArrowUp",

        // Navigation
        0x72 => "Insert",
        0x73 => "Home",
        0x74 => "PageUp",
        0x75 => "Delete",
        0x77 => "End",
        0x79 => "PageDown",

        // Numpad
        0x52 => "Numpad0",
        0x53 => "Numpad1",
        0x54 => "Numpad2",
        0x55 => "Numpad3",
        0x56 => "Numpad4",
        0x57 => "Numpad5",
        0x58 => "Numpad6",
        0x59 => "Numpad7",
        0x5B => "Numpad8",
        0x5C => "Numpad9",
        0x41 => "NumpadDecimal",
        0x43 => "NumpadMultiply",
        0x45 => "NumpadAdd",
        0x4B => "NumpadDivide",
        0x4C => "NumpadEnter",
        0x4E => "NumpadSubtract",
        0x47 => "NumLock",

        _ => return Some(format!("Unknown({})", keycode)),
    };
    Some(code.to_string())
}
