use std::fs::File;
use std::time::Duration;

use rodio::Source;
use symphonia::core::audio::{SampleBuffer, SignalSpec};
use symphonia::core::codecs::{Decoder, DecoderOptions};
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

/// A rodio-compatible Source that uses symphonia for fast byte-level seeking.
/// Unlike rodio's skip_duration (which decodes sample-by-sample),
/// symphonia seeks directly to the byte offset in the file.
pub struct SymphoniaSource {
    reader: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    sample_rate: u32,
    channels: u16,
    // Current decoded samples buffer
    sample_buf: Vec<f32>,
    sample_pos: usize,
    finished: bool,
}

impl SymphoniaSource {
    /// Open a file and seek to the given position in seconds.
    /// The seek is fast (byte-level for most formats).
    pub fn new(file_path: &str, seek_to_secs: f64) -> Result<Self, String> {
        let file = File::open(file_path)
            .map_err(|e| format!("Failed to open file: {}", e))?;

        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        // Probe the file format
        let mut hint = Hint::new();
        if let Some(ext) = std::path::Path::new(file_path).extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
            .map_err(|e| format!("Failed to probe audio format: {}", e))?;

        let reader = probed.format;

        // Find the first audio track
        let track = reader
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
            .ok_or_else(|| "No audio track found".to_string())?;

        let track_id = track.id;
        let codec_params = track.codec_params.clone();

        let sample_rate = codec_params.sample_rate
            .ok_or_else(|| "Unknown sample rate".to_string())?;
        let channels = codec_params.channels
            .map(|c| c.count() as u16)
            .unwrap_or(2);

        // Create decoder
        let decoder = symphonia::default::get_codecs()
            .make(&codec_params, &DecoderOptions::default())
            .map_err(|e| format!("Failed to create decoder: {}", e))?;

        let mut source = Self {
            reader,
            decoder,
            track_id,
            sample_rate,
            channels,
            sample_buf: Vec::new(),
            sample_pos: 0,
            finished: false,
        };

        // Seek to the target position
        if seek_to_secs > 0.0 {
            source.seek(seek_to_secs);
        }

        // Pre-fill the first buffer
        source.decode_next_packet();

        Ok(source)
    }

    /// Seek to a position in seconds (fast byte-level seek).
    fn seek(&mut self, position_secs: f64) {
        let seek_to = SeekTo::Time {
            time: Time::from(position_secs),
            track_id: Some(self.track_id),
        };

        match self.reader.seek(SeekMode::Coarse, seek_to) {
            Ok(_seeked_to) => {
                // Reset decoder state after seeking
                self.decoder.reset();
                self.sample_buf.clear();
                self.sample_pos = 0;
                self.finished = false;
            }
            Err(_) => {
                // Seek failed - mark as finished
                self.finished = true;
            }
        }
    }

    /// Decode the next packet into the sample buffer.
    fn decode_next_packet(&mut self) {
        loop {
            let packet = match self.reader.next_packet() {
                Ok(p) => p,
                Err(_) => {
                    self.finished = true;
                    return;
                }
            };

            // Skip packets from other tracks
            if packet.track_id() != self.track_id {
                continue;
            }

            match self.decoder.decode(&packet) {
                Ok(decoded) => {
                    let spec = decoded.spec().clone();
                    let duration = decoded.capacity();
                    if duration == 0 {
                        continue;
                    }
                    let signal_spec = SignalSpec::new(self.sample_rate, spec.channels);
                    let mut sample_buf = SampleBuffer::<f32>::new(duration as u64, signal_spec);
                    sample_buf.copy_interleaved_ref(decoded);
                    self.sample_buf = sample_buf.samples().to_vec();
                    self.sample_pos = 0;
                    return;
                }
                Err(symphonia::core::errors::Error::DecodeError(_)) => {
                    // Skip corrupted packets
                    continue;
                }
                Err(_) => {
                    self.finished = true;
                    return;
                }
            }
        }
    }
}

impl Iterator for SymphoniaSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.finished {
            return None;
        }

        if self.sample_pos >= self.sample_buf.len() {
            self.decode_next_packet();
            if self.finished {
                return None;
            }
        }

        let sample = self.sample_buf[self.sample_pos];
        self.sample_pos += 1;
        Some(sample)
    }
}

impl Source for SymphoniaSource {
    fn current_frame_len(&self) -> Option<usize> {
        if self.finished {
            Some(0)
        } else if self.sample_pos < self.sample_buf.len() {
            Some(self.sample_buf.len() - self.sample_pos)
        } else {
            // Next packet will be decoded on demand
            Some(4096)
        }
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
