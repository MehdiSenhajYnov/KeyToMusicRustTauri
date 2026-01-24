use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use crate::types::SoundId;

/// Metadata about an audio file.
pub struct AudioMetadata {
    pub duration_secs: f64,
    pub sample_rate: u32,
    pub channels: u16,
}

/// Pre-loaded audio buffer segments for a sound.
pub struct BufferedSound {
    pub sound_id: SoundId,
    pub file_path: String,
    pub duration_secs: f64,
    pub sample_rate: u32,
    pub channels: u16,
}

/// Manages pre-loaded audio buffers for fast playback startup.
pub struct BufferManager {
    pub buffers: HashMap<SoundId, BufferedSound>,
}

impl BufferManager {
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
        }
    }

    /// Read audio metadata (duration, sample rate, channels) from a file.
    pub fn read_audio_metadata(path: &str) -> Result<AudioMetadata, String> {
        let file = File::open(path).map_err(|e| format!("Failed to open audio file: {}", e))?;
        let reader = BufReader::new(file);
        let decoder = rodio::Decoder::new(reader)
            .map_err(|e| format!("Failed to decode audio file: {}", e))?;

        use rodio::Source;
        let sample_rate = decoder.sample_rate();
        let channels = decoder.channels();

        // Try to get duration from metadata first (instant for MP3/OGG/FLAC)
        let duration_secs = if let Some(dur) = decoder.total_duration() {
            dur.as_secs_f64()
        } else {
            // Fallback: count samples (slow but works for all formats)
            let total_samples: usize = decoder.count();
            total_samples as f64 / (sample_rate as f64 * channels as f64)
        };

        Ok(AudioMetadata {
            duration_secs,
            sample_rate,
            channels,
        })
    }

    /// Get audio duration in seconds for a file path.
    /// Uses symphonia to read duration from headers (instant, no decoding).
    pub fn get_audio_duration(path: &str) -> Result<f64, String> {
        use symphonia::core::formats::FormatOptions;
        use symphonia::core::io::MediaSourceStream;
        use symphonia::core::meta::MetadataOptions;
        use symphonia::core::probe::Hint;

        let file = File::open(path)
            .map_err(|e| format!("Failed to open audio file: {}", e))?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = std::path::Path::new(path).extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
            .map_err(|e| format!("Failed to probe audio format: {}", e))?;

        let reader = probed.format;
        let track = reader
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
            .ok_or_else(|| "No audio track found".to_string())?;

        let params = &track.codec_params;
        let sample_rate = params.sample_rate.unwrap_or(44100) as f64;

        if let Some(n_frames) = params.n_frames {
            Ok(n_frames as f64 / sample_rate)
        } else {
            // Fallback to rodio if symphonia can't determine frame count
            let metadata = Self::read_audio_metadata(path)?;
            Ok(metadata.duration_secs)
        }
    }

    /// Register a sound in the buffer manager (stores metadata, not full audio data).
    pub fn register_sound(&mut self, sound_id: SoundId, file_path: String) -> Result<(), String> {
        if !Path::new(&file_path).exists() {
            return Err(format!("Sound file not found: {}", file_path));
        }

        let metadata = Self::read_audio_metadata(&file_path)?;

        self.buffers.insert(
            sound_id.clone(),
            BufferedSound {
                sound_id,
                file_path,
                duration_secs: metadata.duration_secs,
                sample_rate: metadata.sample_rate,
                channels: metadata.channels,
            },
        );

        Ok(())
    }

    /// Remove a sound from the buffer manager.
    pub fn unregister_sound(&mut self, sound_id: &str) {
        self.buffers.remove(sound_id);
    }

    /// Check if a sound is registered.
    pub fn has_sound(&self, sound_id: &str) -> bool {
        self.buffers.contains_key(sound_id)
    }

    /// Get the file path for a registered sound.
    pub fn get_file_path(&self, sound_id: &str) -> Option<&str> {
        self.buffers.get(sound_id).map(|b| b.file_path.as_str())
    }

    /// Get the duration of a registered sound.
    pub fn get_duration(&self, sound_id: &str) -> Option<f64> {
        self.buffers.get(sound_id).map(|b| b.duration_secs)
    }

    /// Clear all buffers.
    pub fn clear(&mut self) {
        self.buffers.clear();
    }
}
