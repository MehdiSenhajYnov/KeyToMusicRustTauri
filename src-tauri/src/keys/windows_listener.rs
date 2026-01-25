//! Windows-specific global keyboard listener using Raw Input API.
//! This implementation uses RegisterRawInputDevices and processes WM_INPUT messages
//! in a hidden message-only window, which should work regardless of window focus.

use std::sync::mpsc::{channel, Sender};
use std::sync::OnceLock;
use tracing::{info, warn};
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

/// Global window handle for raw input
static mut RAW_INPUT_HWND: HWND = HWND(std::ptr::null_mut());

/// Start listening for global keyboard events using Raw Input API.
/// This function blocks and runs the Windows message loop.
/// Call this from a dedicated thread.
pub fn listen_windows<F>(callback: F)
where
    F: Fn(WinKeyEvent) + Send + 'static,
{
    let (tx, rx) = channel::<WinKeyEvent>();
    KEY_SENDER.set(tx).ok();

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

        RAW_INPUT_HWND = hwnd;
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

        info!(
            "[WinRawInput] Raw input: vk=0x{:02X} scan=0x{:02X} flags=0x{:02X} extended={} keyup={} code={}",
            vk_code, scan_code, flags, is_extended, is_key_up, key_code
        );

        if let Some(sender) = KEY_SENDER.get() {
            let event = if is_key_up {
                WinKeyEvent::Release(key_code)
            } else {
                WinKeyEvent::Press(key_code)
            };

            info!("[WinRawInput] Sending event: {:?}", event);
            let _ = sender.send(event);
        }
    }
}

/// Convert Windows virtual key code to Web KeyboardEvent.code format
fn vk_to_code(vk: u32, scan_code: u32, is_extended: bool) -> String {
    match vk {
        // Letters A-Z (0x41-0x5A)
        0x41..=0x5A => format!("Key{}", (vk as u8) as char),

        // Digits 0-9 (0x30-0x39)
        0x30..=0x39 => format!("Digit{}", (vk as u8 - 0x30)),

        // Numpad 0-9 (0x60-0x69)
        0x60..=0x69 => format!("Numpad{}", vk - 0x60),

        // Numpad operators
        0x6A => "NumpadMultiply".to_string(),
        0x6B => "NumpadAdd".to_string(),
        0x6C => "NumpadComma".to_string(),
        0x6D => "NumpadSubtract".to_string(),
        0x6E => "NumpadDecimal".to_string(),
        0x6F => "NumpadDivide".to_string(),

        // Function keys F1-F12 (0x70-0x7B)
        0x70..=0x7B => format!("F{}", vk - 0x70 + 1),

        // Function keys F13-F24 (0x7C-0x87)
        0x7C..=0x87 => format!("F{}", vk - 0x7C + 13),

        // Arrow keys
        0x25 => "ArrowLeft".to_string(),
        0x26 => "ArrowUp".to_string(),
        0x27 => "ArrowRight".to_string(),
        0x28 => "ArrowDown".to_string(),

        // Special keys
        0x08 => "Backspace".to_string(),
        0x09 => "Tab".to_string(),
        0x0D => {
            if is_extended {
                "NumpadEnter".to_string()
            } else {
                "Enter".to_string()
            }
        }
        // Shift: scan code 0x36 is right shift, 0x2A is left shift
        0x10 | 0xA0 | 0xA1 => {
            if scan_code == 0x36 {
                "ShiftRight".to_string()
            } else {
                "ShiftLeft".to_string()
            }
        }
        // Control
        0x11 | 0xA2 | 0xA3 => {
            if is_extended {
                "ControlRight".to_string()
            } else {
                "ControlLeft".to_string()
            }
        }
        // Alt (Menu)
        0x12 | 0xA4 | 0xA5 => {
            if is_extended {
                "AltRight".to_string()
            } else {
                "AltLeft".to_string()
            }
        }
        0x13 => "Pause".to_string(),
        0x14 => "CapsLock".to_string(),
        0x1B => "Escape".to_string(),
        0x20 => "Space".to_string(),
        0x21 => "PageUp".to_string(),
        0x22 => "PageDown".to_string(),
        0x23 => "End".to_string(),
        0x24 => "Home".to_string(),
        0x2C => "PrintScreen".to_string(),
        0x2D => "Insert".to_string(),
        0x2E => "Delete".to_string(),
        0x5B => "MetaLeft".to_string(),
        0x5C => "MetaRight".to_string(),
        0x5D => "ContextMenu".to_string(),
        0x90 => "NumLock".to_string(),
        0x91 => "ScrollLock".to_string(),

        // Punctuation and symbols (US keyboard layout)
        0xBA => "Semicolon".to_string(),
        0xBB => "Equal".to_string(),
        0xBC => "Comma".to_string(),
        0xBD => "Minus".to_string(),
        0xBE => "Period".to_string(),
        0xBF => "Slash".to_string(),
        0xC0 => "Backquote".to_string(),
        0xDB => "BracketLeft".to_string(),
        0xDC => "Backslash".to_string(),
        0xDD => "BracketRight".to_string(),
        0xDE => "Quote".to_string(),
        0xE2 => "IntlBackslash".to_string(),

        // Unknown
        _ => format!("Unknown(0x{:02X})", vk),
    }
}
