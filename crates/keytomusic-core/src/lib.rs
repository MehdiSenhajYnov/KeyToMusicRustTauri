//! KeyToMusic core: audio engine, global key detection, YouTube, discovery,
//! Manga Mood AI, storage and import/export, independent of any UI toolkit.

pub mod audio;
pub mod discovery;
pub mod events;
pub mod import_export;
pub mod keys;
pub mod mood;
pub mod runtime;
pub mod service;
pub mod state;
pub mod storage;
pub mod types;
pub mod youtube;

pub use events::{CoreEvent, EventSink};
pub use state::AppState;
