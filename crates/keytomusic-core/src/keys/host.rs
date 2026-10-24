//! Physical key codes of the host toolkit's hardware keycodes (GDK's
//! `hardware_keycode`), for native UIs that label or filter keys.

use std::ops::RangeInclusive;

/// Hardware keycodes worth scanning to build a layout table.
pub fn host_keycodes() -> RangeInclusive<u32> {
    if cfg!(target_os = "windows") {
        1..=254 // virtual keys
    } else {
        9..=263 // evdev codes + 8 (X11/GDK numbering)
    }
}

/// Physical key ("KeyQ") of a hardware keycode, if it names one.
pub fn host_keycode_to_code(keycode: u32) -> Option<&'static str> {
    #[cfg(target_os = "linux")]
    {
        keycode
            .checked_sub(8)
            .and_then(|raw| u16::try_from(raw).ok())
            .and_then(super::linux_listener::raw_keycode_to_code)
    }
    #[cfg(target_os = "windows")]
    {
        super::windows_listener::virtual_key_to_code(keycode)
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        let _ = keycode;
        None
    }
}

/// Report keys by physical position everywhere (native UIs). Linux and macOS
/// already do; the Windows listener otherwise follows virtual keys.
pub fn use_physical_codes() {
    #[cfg(target_os = "windows")]
    super::windows_listener::set_physical_codes(true);
}
