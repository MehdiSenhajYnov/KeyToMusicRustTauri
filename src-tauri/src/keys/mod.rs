pub mod chord;
pub mod detector;
pub mod mapping;
#[cfg(target_os = "macos")]
pub mod macos_listener;
#[cfg(target_os = "windows")]
pub mod windows_listener;

pub use chord::{ChordDetectorHandle, ChordResult};
pub use detector::KeyDetector;
pub use mapping::KeyEvent;
