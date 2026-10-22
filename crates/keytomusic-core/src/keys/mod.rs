pub mod chord;
pub mod detector;
#[cfg(target_os = "linux")]
pub mod linux_listener;
#[cfg(target_os = "macos")]
pub mod macos_listener;
pub mod mapping;
#[cfg(target_os = "windows")]
pub mod windows_listener;

pub use detector::KeyDetector;
pub use mapping::KeyEvent;
