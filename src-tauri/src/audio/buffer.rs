use std::fs::File;
use std::io::BufReader;

/// Metadata about an audio file.
pub struct AudioMetadata {
    pub duration_secs: f64,
}

/// Provides audio file metadata reading and duration computation.
pub struct BufferManager;

impl BufferManager {
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

}
