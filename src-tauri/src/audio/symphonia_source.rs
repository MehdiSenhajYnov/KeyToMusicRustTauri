use std::fs::File;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use rodio::Source;
use symphonia::core::audio::{SampleBuffer, SignalSpec};
use symphonia::core::codecs::{Decoder, DecoderOptions};
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

/// Max iterations when scanning for a valid packet (avoids unbounded loops on
/// files with many non-audio or corrupted packets).
const MAX_DECODE_ITERATIONS: usize = 50;

/// Number of decoded sample buffers to keep ahead in the channel.
const PREFETCH_BUFFERS: usize = 4;

/// A rodio-compatible Source that uses symphonia for fast byte-level seeking.
/// Decoding happens in a separate thread to avoid blocking the audio callback.
pub struct SymphoniaSource {
    sample_rate: u32,
    channels: u16,
    // Current decoded samples buffer (consumed by Iterator::next)
    sample_buf: Vec<f32>,
    sample_pos: usize,
    finished: bool,
    // Channel to receive pre-decoded buffers from the decode thread
    buf_rx: mpsc::Receiver<Vec<f32>>,
    // Signal to stop the decode thread
    stop_flag: Arc<AtomicBool>,
}

impl SymphoniaSource {
    /// Open a file and seek to the given position in seconds.
    /// The seek is fast (byte-level for most formats).
    /// Decoding is done in a background thread; the returned source reads
    /// from a bounded channel of pre-decoded sample buffers.
    pub fn new(file_path: &str, seek_to_secs: f64) -> Result<Self, String> {
        let file = File::open(file_path).map_err(|e| format!("Failed to open file: {}", e))?;

        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        // Probe the file format
        let mut hint = Hint::new();
        if let Some(ext) = std::path::Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
        {
            hint.with_extension(ext);
        }

        let probed = symphonia::default::get_probe()
            .format(
                &hint,
                mss,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .map_err(|e| format!("Failed to probe audio format: {}", e))?;

        let mut reader = probed.format;

        // Find the first audio track
        let track = reader
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
            .ok_or_else(|| "No audio track found".to_string())?;

        let track_id = track.id;
        let codec_params = track.codec_params.clone();

        let sample_rate = codec_params
            .sample_rate
            .ok_or_else(|| "Unknown sample rate".to_string())?;
        let channels = codec_params.channels.map(|c| c.count() as u16).unwrap_or(2);

        // Create decoder
        let mut decoder = symphonia::default::get_codecs()
            .make(&codec_params, &DecoderOptions::default())
            .map_err(|e| format!("Failed to create decoder: {}", e))?;

        // Seek to the target position
        if seek_to_secs > 0.0 {
            let seek_to = SeekTo::Time {
                time: Time::from(seek_to_secs),
                track_id: Some(track_id),
            };
            match reader.seek(SeekMode::Coarse, seek_to) {
                Ok(_) => {
                    decoder.reset();
                }
                Err(_) => {
                    return Err("Seek failed".to_string());
                }
            }
        }

        // Bounded channel for pre-decoded buffers
        let (buf_tx, buf_rx) = mpsc::sync_channel::<Vec<f32>>(PREFETCH_BUFFERS);
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_flag_clone = stop_flag.clone();

        // Spawn the decode thread
        thread::Builder::new()
            .name("symphonia-decode".into())
            .spawn(move || {
                decode_thread(
                    reader,
                    decoder,
                    track_id,
                    sample_rate,
                    buf_tx,
                    stop_flag_clone,
                );
            })
            .map_err(|e| format!("Failed to spawn decode thread: {}", e))?;

        // Pre-fill the first buffer so the source is immediately ready
        let (sample_buf, finished) = match buf_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(buf) => (buf, false),
            Err(_) => (Vec::new(), true),
        };

        Ok(Self {
            sample_rate,
            channels,
            sample_buf,
            sample_pos: 0,
            finished,
            buf_rx,
            stop_flag,
        })
    }
}

/// Background thread that decodes packets and sends sample buffers over the channel.
fn decode_thread(
    mut reader: Box<dyn FormatReader>,
    mut decoder: Box<dyn Decoder>,
    track_id: u32,
    sample_rate: u32,
    buf_tx: mpsc::SyncSender<Vec<f32>>,
    stop_flag: Arc<AtomicBool>,
) {
    let mut decode_buf: Option<SampleBuffer<f32>> = None;

    loop {
        if stop_flag.load(Ordering::Relaxed) {
            break;
        }

        let mut iterations = 0;
        let packet_result = loop {
            if iterations >= MAX_DECODE_ITERATIONS {
                break None;
            }
            iterations += 1;

            let packet = match reader.next_packet() {
                Ok(p) => p,
                Err(_) => break None,
            };

            // Skip packets from other tracks
            if packet.track_id() != track_id {
                continue;
            }

            match decoder.decode(&packet) {
                Ok(decoded) => {
                    let spec = *decoded.spec();
                    let duration = decoded.capacity();
                    if duration == 0 {
                        continue;
                    }
                    let signal_spec = SignalSpec::new(sample_rate, spec.channels);

                    // Reuse the persistent SampleBuffer if capacity is sufficient
                    let buf = match &mut decode_buf {
                        Some(buf) if buf.capacity() >= duration => buf,
                        _ => {
                            decode_buf =
                                Some(SampleBuffer::<f32>::new(duration as u64, signal_spec));
                            decode_buf.as_mut().unwrap()
                        }
                    };
                    buf.copy_interleaved_ref(decoded);

                    break Some(buf.samples().to_vec());
                }
                Err(symphonia::core::errors::Error::DecodeError(_)) => {
                    // Skip corrupted packets
                    continue;
                }
                Err(_) => break None,
            }
        };

        match packet_result {
            Some(samples) => {
                // Send blocks if channel is full (backpressure), which is fine —
                // it means the audio thread hasn't consumed yet.
                if buf_tx.send(samples).is_err() {
                    // Receiver dropped (source was dropped) — exit
                    break;
                }
            }
            None => {
                // End of stream or unrecoverable error
                break;
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
            // Try to get the next pre-decoded buffer (non-blocking)
            match self.buf_rx.try_recv() {
                Ok(buf) => {
                    self.sample_buf = buf;
                    self.sample_pos = 0;
                }
                Err(mpsc::TryRecvError::Empty) => {
                    // Decode thread is still working but no buffer ready yet.
                    // Return silence to avoid blocking the real-time audio thread.
                    // The prefetch buffer will catch up on the next callback.
                    return Some(0.0);
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    // Decode thread has exited (end of stream or error)
                    self.finished = true;
                    return None;
                }
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
            // Next buffer will come from decode thread
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

impl Drop for SymphoniaSource {
    fn drop(&mut self) {
        // Signal the decode thread to stop
        self.stop_flag.store(true, Ordering::Relaxed);
    }
}
