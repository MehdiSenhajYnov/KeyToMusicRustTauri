//! Mood AI: a vision model behind any OpenAI-compatible API reads the manga
//! chapter open in the browser (pages sent by the extension) and splits it
//! into scenes; the scene of the page being read decides which tagged keys
//! play.
//!
//! - [`provider`]: OpenAI-compatible client (images in, JSON out)
//! - [`analyzer`]: prompt, answer schema, scenes → page moods
//! - [`chapter`]: pages received, moods decided, next batch to send
//! - [`engine`]: background workers, playback decisions, disk cache
//! - [`server`]: local HTTP API for the extension
//! - [`extension`]: the extension itself, embedded for export
//! - [`secret`]: API key storage

pub mod analyzer;
pub mod chapter;
pub mod engine;
pub mod extension;
pub mod provider;
pub mod secret;
pub mod server;
