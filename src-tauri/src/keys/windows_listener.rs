//! Windows-specific global keyboard listener using Raw Input API.
//! This implementation uses RegisterRawInputDevices and processes WM_INPUT messages
//! in a hidden message-only window, which should work regardless of window focus.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, OnceLock};
use tracing::{debug, info, warn};
use windows::Win32::Devices::HumanInterfaceDevice::{
    HID_USAGE_GENERIC_KEYBOARD, HID_USAGE_PAGE_GENERIC,
};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::{
    GetRawInputData, RegisterRawInputDevices, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE,
    RAWINPUTHEADER, RIDEV_INPUTSINK, RID_INPUT, RIM_TYPEKEYBOARD,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, RegisterClassW,
    TranslateMessage, CS_HREDRAW, CS_VREDRAW, HWND_MESSAGE, MSG, WINDOW_EX_STYLE,
    WM_INPUT, WNDCLASSW, WS_OVERLAPPEDWINDOW,
};

/// Key event sent from the hook to the main handler
#[derive(Debug, Clone)]
pub enum WinKeyEvent {
    Press(String),
    Release(String),
}

/// Global sender for the keyboard events
static KEY_SENDER: OnceLock<Sender<WinKeyEvent>> = OnceLock::new();

/// Global flag indicating if key detection is enabled
/// When false, textual keys (letters, digits, space, etc.) are not sent to avoid interfering with text inputs
static KEY_DETECTION_ENABLED: OnceLock<Arc<AtomicBool>> = OnceLock::new();

/// Start listening for global keyboard events using Raw Input API.
/// This function blocks and runs the Windows message loop.
/// Call this from a dedicated thread.
///
/// The `enabled` flag is checked before sending textual keys (letters, digits, space, etc.)
/// to avoid interfering with text inputs when key detection is disabled.
pub fn listen_windows<F>(callback: F, enabled: Arc<AtomicBool>)
where
    F: Fn(WinKeyEvent) + Send + 'static,
{
    let (tx, rx) = channel::<WinKeyEvent>();
    KEY_SENDER.set(tx).ok();
    KEY_DETECTION_ENABLED.set(enabled).ok();

    // Spawn a thread to process received events
    std::thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            callback(event);
        }
    });

    unsafe {
        info!("[WinRawInput] Registering window class...");

        // Register a window class for our hidden window
        let class_name: Vec<u16> = "KeyToMusicRawInput\0".encode_utf16().collect();

        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(raw_input_wnd_proc),
            hInstance: windows::Win32::System::LibraryLoader::GetModuleHandleW(None)
                .unwrap_or_default()
                .into(),
            lpszClassName: windows::core::PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };

        let atom = RegisterClassW(&wc);
        if atom == 0 {
            warn!("[WinRawInput] Failed to register window class");
            return;
        }

        info!("[WinRawInput] Creating message-only window...");

        // Create a message-only window (HWND_MESSAGE parent)
        let hwnd = match CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            windows::core::PCWSTR(class_name.as_ptr()),
            windows::core::PCWSTR::null(),
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            0,
            0,
            HWND_MESSAGE, // Message-only window
            None,
            wc.hInstance,
            None,
        ) {
            Ok(h) => h,
            Err(e) => {
                warn!("[WinRawInput] Failed to create window: {:?}", e);
                return;
            }
        };

        if hwnd.0.is_null() {
            warn!("[WinRawInput] Window handle is null");
            return;
        }

        info!("[WinRawInput] Window created: {:?}", hwnd);

        // Register for raw keyboard input
        // RIDEV_INPUTSINK allows receiving input even when not in foreground
        let rid = RAWINPUTDEVICE {
            usUsagePage: HID_USAGE_PAGE_GENERIC,
            usUsage: HID_USAGE_GENERIC_KEYBOARD,
            dwFlags: RIDEV_INPUTSINK,
            hwndTarget: hwnd,
        };

        if let Err(e) = RegisterRawInputDevices(&[rid], std::mem::size_of::<RAWINPUTDEVICE>() as u32) {
            warn!("[WinRawInput] Failed to register raw input devices: {:?}", e);
            return;
        }

        info!("[WinRawInput] Raw input registered successfully, starting message loop...");

        // Run the message loop
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        info!("[WinRawInput] Message loop ended");
    }
}

/// Window procedure for raw input messages
unsafe extern "system" fn raw_input_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_INPUT {
        process_raw_input(HRAWINPUT(lparam.0 as *mut std::ffi::c_void));
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

/// Check if a virtual key code represents a textual key that should be filtered
/// when key detection is disabled (to allow normal typing in text inputs)
fn is_textual_key(vk: u32) -> bool {
    matches!(
        vk,
        // Letters A-Z
        0x41..=0x5A |
        // Digits 0-9 (top row)
        0x30..=0x39 |
        // Space
        0x20 |
        // Numpad 0-9
        0x60..=0x69 |
        // Numpad operators
        0x6A | 0x6B | 0x6C | 0x6D | 0x6E | 0x6F |
        // Punctuation and symbols
        0xBA | 0xBB | 0xBC | 0xBD | 0xBE | 0xBF | 0xC0 | 0xDB | 0xDC | 0xDD | 0xDE | 0xE2 |
        // Backspace, Tab, Enter
        0x08 | 0x09 | 0x0D
    )
}

/// Process a raw input event
unsafe fn process_raw_input(hrawinput: HRAWINPUT) {
    let mut size: u32 = 0;

    // Get required buffer size
    let result = GetRawInputData(
        hrawinput,
        RID_INPUT,
        None,
        &mut size,
        std::mem::size_of::<RAWINPUTHEADER>() as u32,
    );

    if result == u32::MAX {
        return;
    }

    // Allocate buffer and get the data
    let mut buffer = vec![0u8; size as usize];
    let result = GetRawInputData(
        hrawinput,
        RID_INPUT,
        Some(buffer.as_mut_ptr() as *mut std::ffi::c_void),
        &mut size,
        std::mem::size_of::<RAWINPUTHEADER>() as u32,
    );

    if result == u32::MAX || result == 0 {
        return;
    }

    let raw = &*(buffer.as_ptr() as *const RAWINPUT);

    // Only process keyboard input
    if raw.header.dwType == RIM_TYPEKEYBOARD.0 {
        let keyboard = &raw.data.keyboard;
        let vk_code = keyboard.VKey;
        let scan_code = keyboard.MakeCode;
        let flags = keyboard.Flags;

        // Check if this is a key press or release
        // RI_KEY_BREAK (bit 0) = 1 means key up, 0 means key down
        let is_key_up = (flags & 0x01) != 0;
        // RI_KEY_E0 (bit 1) = extended key
        let is_extended = (flags & 0x02) != 0;

        let key_code = vk_to_code(vk_code as u32, scan_code as u32, is_extended);

        debug!(
            "[WinRawInput] Raw input: vk=0x{:02X} scan=0x{:02X} flags=0x{:02X} extended={} keyup={} code={}",
            vk_code, scan_code, flags, is_extended, is_key_up, key_code
        );

        // Filter textual keys when key detection is disabled to avoid interfering with text inputs
        if is_textual_key(vk_code as u32) {
            if let Some(enabled_flag) = KEY_DETECTION_ENABLED.get() {
                if !enabled_flag.load(Ordering::Relaxed) {
                    debug!("[WinRawInput] Key detection disabled, filtering textual key: {}", key_code);
                    return;
                }
            }
        }

        if let Some(sender) = KEY_SENDER.get() {
            let event = if is_key_up {
                WinKeyEvent::Release(key_code)
            } else {
                WinKeyEvent::Press(key_code)
            };

            debug!("[WinRawInput] Sending event: {:?}", event);
            let _ = sender.send(event);
        }
    }
}

/// Pre-computed lookup tables for zero-allocation key code conversion.
static LETTER_CODES: [&str; 26] = [
    "KeyA", "KeyB", "KeyC", "KeyD", "KeyE", "KeyF", "KeyG", "KeyH", "KeyI",
    "KeyJ", "KeyK", "KeyL", "KeyM", "KeyN", "KeyO", "KeyP", "KeyQ", "KeyR",
    "KeyS", "KeyT", "KeyU", "KeyV", "KeyW", "KeyX", "KeyY", "KeyZ",
];

static DIGIT_CODES: [&str; 10] = [
    "Digit0", "Digit1", "Digit2", "Digit3", "Digit4",
    "Digit5", "Digit6", "Digit7", "Digit8", "Digit9",
];

static NUMPAD_CODES: [&str; 10] = [
    "Numpad0", "Numpad1", "Numpad2", "Numpad3", "Numpad4",
    "Numpad5", "Numpad6", "Numpad7", "Numpad8", "Numpad9",
];

static F_KEY_CODES: [&str; 24] = [
    "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12",
    "F13", "F14", "F15", "F16", "F17", "F18", "F19", "F20", "F21", "F22", "F23", "F24",
];

/// Convert Windows virtual key code to Web KeyboardEvent.code format
fn vk_to_code(vk: u32, scan_code: u32, is_extended: bool) -> String {
    let static_code: Option<&'static str> = match vk {
        // Letters A-Z (0x41-0x5A)
        0x41..=0x5A => Some(LETTER_CODES[(vk - 0x41) as usize]),

        // Digits 0-9 (0x30-0x39)
        0x30..=0x39 => Some(DIGIT_CODES[(vk - 0x30) as usize]),

        // Numpad 0-9 (0x60-0x69)
        0x60..=0x69 => Some(NUMPAD_CODES[(vk - 0x60) as usize]),

        // Numpad operators
        0x6A => Some("NumpadMultiply"),
        0x6B => Some("NumpadAdd"),
        0x6C => Some("NumpadComma"),
        0x6D => Some("NumpadSubtract"),
        0x6E => Some("NumpadDecimal"),
        0x6F => Some("NumpadDivide"),

        // Function keys F1-F24 (0x70-0x87)
        0x70..=0x87 => Some(F_KEY_CODES[(vk - 0x70) as usize]),

        // Arrow keys
        0x25 => Some("ArrowLeft"),
        0x26 => Some("ArrowUp"),
        0x27 => Some("ArrowRight"),
        0x28 => Some("ArrowDown"),

        // Special keys
        0x08 => Some("Backspace"),
        0x09 => Some("Tab"),
        0x0D => Some(if is_extended { "NumpadEnter" } else { "Enter" }),
        // Shift: scan code 0x36 is right shift, 0x2A is left shift
        0x10 | 0xA0 | 0xA1 => Some(if scan_code == 0x36 { "ShiftRight" } else { "ShiftLeft" }),
        // Control
        0x11 | 0xA2 | 0xA3 => Some(if is_extended { "ControlRight" } else { "ControlLeft" }),
        // Alt (Menu)
        0x12 | 0xA4 | 0xA5 => Some(if is_extended { "AltRight" } else { "AltLeft" }),
        0x13 => Some("Pause"),
        0x14 => Some("CapsLock"),
        0x1B => Some("Escape"),
        0x20 => Some("Space"),
        0x21 => Some("PageUp"),
        0x22 => Some("PageDown"),
        0x23 => Some("End"),
        0x24 => Some("Home"),
        0x2C => Some("PrintScreen"),
        0x2D => Some("Insert"),
        0x2E => Some("Delete"),
        0x5B => Some("MetaLeft"),
        0x5C => Some("MetaRight"),
        0x5D => Some("ContextMenu"),
        0x90 => Some("NumLock"),
        0x91 => Some("ScrollLock"),

        // Punctuation and symbols (US keyboard layout)
        0xBA => Some("Semicolon"),
        0xBB => Some("Equal"),
        0xBC => Some("Comma"),
        0xBD => Some("Minus"),
        0xBE => Some("Period"),
        0xBF => Some("Slash"),
        0xC0 => Some("Backquote"),
        0xDB => Some("BracketLeft"),
        0xDC => Some("Backslash"),
        0xDD => Some("BracketRight"),
        0xDE => Some("Quote"),
        0xE2 => Some("IntlBackslash"),

        _ => None,
    };

    match static_code {
        Some(s) => s.to_string(),
        None => format!("Unknown(0x{:02X})", vk),
    }
}
