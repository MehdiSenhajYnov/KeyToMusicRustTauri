//! Background key capture access on Linux/Wayland (evdev + udev uaccess rule).

#[cfg(target_os = "linux")]
use std::os::unix::fs::PermissionsExt;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinuxInputAccessStatusPayload {
    pub supported: bool,
    pub session_type: String,
    pub background_detection_available: bool,
    pub can_auto_fix: bool,
    pub relogin_recommended: bool,
    pub accessible_keyboard_devices: Vec<String>,
    pub keyboard_candidates: Vec<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinuxInputAccessFixResult {
    pub success: bool,
    pub message: String,
    pub status: LinuxInputAccessStatusPayload,
}

pub fn is_wayland_session() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
        || std::env::var("XDG_SESSION_TYPE")
            .map(|value| value.eq_ignore_ascii_case("wayland"))
            .unwrap_or(false)
}

#[cfg(target_os = "linux")]
pub fn linux_input_status() -> LinuxInputAccessStatusPayload {
    if is_wayland_session() {
        let status = crate::keys::linux_listener::get_linux_input_access_status();
        LinuxInputAccessStatusPayload {
            supported: status.supported,
            session_type: status.session_type,
            background_detection_available: status.background_detection_available,
            can_auto_fix: status.can_auto_fix,
            relogin_recommended: status.relogin_recommended,
            accessible_keyboard_devices: status.accessible_keyboard_devices,
            keyboard_candidates: status.keyboard_candidates,
            message: status.message,
        }
    } else {
        LinuxInputAccessStatusPayload {
            supported: true,
            session_type: "x11".to_string(),
            background_detection_available: true,
            can_auto_fix: false,
            relogin_recommended: false,
            accessible_keyboard_devices: Vec::new(),
            keyboard_candidates: Vec::new(),
            message: Some("Background detection uses the X11 hook in this session.".to_string()),
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub fn linux_input_status() -> LinuxInputAccessStatusPayload {
    LinuxInputAccessStatusPayload {
        supported: false,
        session_type: "other".to_string(),
        background_detection_available: false,
        can_auto_fix: false,
        relogin_recommended: false,
        accessible_keyboard_devices: Vec::new(),
        keyboard_candidates: Vec::new(),
        message: Some(
            "Automatic Linux background-input setup is only available on Linux.".to_string(),
        ),
    }
}

/// Install a udev rule granting the session user access to keyboards (via pkexec).
pub fn enable_linux_background_detection() -> Result<LinuxInputAccessFixResult, String> {
    #[cfg(not(target_os = "linux"))]
    {
        Err("Automatic background-input setup is only available on Linux".to_string())
    }

    #[cfg(target_os = "linux")]
    {
        if !is_wayland_session() {
            let status = linux_input_status();
            return Ok(LinuxInputAccessFixResult {
                success: true,
                message: "No extra setup is needed on X11.".to_string(),
                status,
            });
        }

        let current_status = linux_input_status();
        if current_status.background_detection_available {
            return Ok(LinuxInputAccessFixResult {
                success: true,
                message: "Background detection is already available.".to_string(),
                status: current_status,
            });
        }

        let temp_path = std::env::temp_dir().join(format!(
            "keytomusic-enable-wayland-input-{}.sh",
            std::process::id()
        ));

        let script = r#"#!/bin/sh
set -eu
RULE_PATH="/etc/udev/rules.d/70-keytomusic-background-input.rules"
cat > "$RULE_PATH" <<'EOF'
ACTION!="remove", SUBSYSTEM=="input", KERNEL=="event*", ENV{ID_INPUT_KEYBOARD}=="1", TAG+="uaccess"
EOF
chmod 0644 "$RULE_PATH"
udevadm control --reload-rules
udevadm trigger --subsystem-match=input --action=change
"#;

        std::fs::write(&temp_path, script)
            .map_err(|e| format!("Failed to prepare setup helper: {}", e))?;
        std::fs::set_permissions(&temp_path, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| format!("Failed to mark setup helper executable: {}", e))?;

        let output = std::process::Command::new("pkexec")
            .arg("/bin/sh")
            .arg(&temp_path)
            .output();

        let _ = std::fs::remove_file(&temp_path);

        let output = match output {
            Ok(output) => output,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(
                    "Automatic setup requires pkexec, but it is not installed on this system."
                        .to_string(),
                )
            }
            Err(e) => return Err(format!("Failed to launch automatic setup: {}", e)),
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let detail = if !stderr.is_empty() {
                stderr
            } else if !stdout.is_empty() {
                stdout
            } else {
                "The permission request was cancelled or denied.".to_string()
            };
            return Err(format!("Automatic setup failed: {}", detail));
        }

        let status = linux_input_status();
        Ok(LinuxInputAccessFixResult {
            success: true,
            message: if status.background_detection_available {
                "Background detection access has been enabled.".to_string()
            } else {
                "System access rules were installed. The app will keep retrying automatically; if background detection still does not recover, sign out and back in once.".to_string()
            },
            status,
        })
    }
}
