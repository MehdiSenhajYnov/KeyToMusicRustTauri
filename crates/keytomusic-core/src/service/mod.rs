//! Host-agnostic operations behind the UI commands. Every function takes the
//! shared [`AppState`](crate::state::AppState) and reports progress through its
//! event sink, so the Tauri commands and the native UI call the same code.

pub mod config;
pub mod discovery;
pub mod input_access;
pub mod library;
pub mod mood;
pub mod playback;
pub mod waveform;
pub mod youtube;
