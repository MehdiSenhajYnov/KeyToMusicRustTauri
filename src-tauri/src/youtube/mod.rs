pub mod cache;
pub mod downloader;
pub mod ffmpeg_manager;
pub mod search;
pub mod yt_dlp_manager;

pub use cache::YouTubeCache;
pub use downloader::download_audio;
pub use ffmpeg_manager::{is_installed as is_ffmpeg_installed, download_ffmpeg};
pub use yt_dlp_manager::{download_yt_dlp, ensure_yt_dlp_up_to_date, is_installed as is_yt_dlp_installed};
