use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::{FormatOptions, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaveformData {
    pub points: Vec<f32>,
    pub duration: f64,
    pub sample_rate: u32,
    pub suggested_momentum: Option<f64>,
}

/// Compute a waveform (RMS amplitude per segment) for the given audio file.
/// `num_points` controls the resolution (e.g., 200 for a typical display width).
pub fn compute_waveform(file_path: &str, num_points: usize) -> Result<WaveformData, String> {
    let num_points = num_points.max(10).min(2000);

    let file =
        File::open(file_path).map_err(|e| format!("Failed to open audio file: {}", e))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

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

    let track = reader
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or_else(|| "No audio track found".to_string())?;

    let track_id = track.id;
    let codec_params = track.codec_params.clone();
    let sample_rate = codec_params.sample_rate.unwrap_or(44100);
    let channels = codec_params
        .channels
        .map(|c| c.count())
        .unwrap_or(2);

    // Get duration
    let duration = codec_params
        .n_frames
        .map(|n| n as f64 / sample_rate as f64)
        .unwrap_or(0.0);

    let mut decoder = symphonia::default::get_codecs()
        .make(&codec_params, &DecoderOptions::default())
        .map_err(|e| format!("Failed to create decoder: {}", e))?;

    // Decode all samples into a mono f32 buffer
    let mut all_samples: Vec<f32> = Vec::new();

    loop {
        let packet = match reader.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(_) => break,
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let spec = *decoded.spec();
        let num_frames = decoded.frames();
        let mut sample_buf = SampleBuffer::<f32>::new(
            num_frames as u64,
            spec,
        );
        sample_buf.copy_interleaved_ref(decoded);
        let samples = sample_buf.samples();

        // Downmix to mono
        let ch = channels.max(1);
        for frame_idx in 0..num_frames {
            let mut sum = 0.0f32;
            for c in 0..ch {
                let idx = frame_idx * ch + c;
                if idx < samples.len() {
                    sum += samples[idx];
                }
            }
            all_samples.push(sum / ch as f32);
        }
    }

    if all_samples.is_empty() {
        return Ok(WaveformData {
            points: vec![0.0; num_points],
            duration,
            sample_rate,
            suggested_momentum: None,
        });
    }

    // Compute RMS per segment
    let segment_size = (all_samples.len() as f64 / num_points as f64).ceil() as usize;
    let segment_size = segment_size.max(1);
    let mut rms_values: Vec<f32> = Vec::with_capacity(num_points);

    for i in 0..num_points {
        let start = i * segment_size;
        let end = ((i + 1) * segment_size).min(all_samples.len());
        if start >= all_samples.len() {
            rms_values.push(0.0);
            continue;
        }

        let mut sum_sq = 0.0f64;
        let count = end - start;
        for &s in &all_samples[start..end] {
            sum_sq += (s as f64) * (s as f64);
        }
        rms_values.push((sum_sq / count as f64).sqrt() as f32);
    }

    // Normalize to 0.0-1.0
    let max_val = rms_values.iter().cloned().fold(0.0f32, f32::max);
    if max_val > 0.0 {
        for v in &mut rms_values {
            *v /= max_val;
        }
    }

    // Smooth with 3-point moving average
    let mut smoothed = vec![0.0f32; rms_values.len()];
    for i in 0..rms_values.len() {
        let mut sum = rms_values[i];
        let mut count = 1.0f32;
        if i > 0 {
            sum += rms_values[i - 1];
            count += 1.0;
        }
        if i + 1 < rms_values.len() {
            sum += rms_values[i + 1];
            count += 1.0;
        }
        smoothed[i] = sum / count;
    }

    // Detect momentum point
    let suggested_momentum = detect_momentum_point(&smoothed, duration);

    Ok(WaveformData {
        points: smoothed,
        duration,
        sample_rate,
        suggested_momentum,
    })
}

/// Compute a waveform by seeking to N positions and decoding a small window at each.
/// ~40x faster than `compute_waveform` for files with seekable containers (M4A, MP3, FLAC).
/// Falls back to `compute_waveform` if seeking fails.
pub fn compute_waveform_sampled(file_path: &str, num_points: usize) -> Result<WaveformData, String> {
    let num_points = num_points.clamp(10, 2000);

    let file =
        File::open(file_path).map_err(|e| format!("Failed to open audio file: {}", e))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

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

    let track = reader
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or_else(|| "No audio track found".to_string())?;

    let track_id = track.id;
    let codec_params = track.codec_params.clone();
    let sample_rate = codec_params.sample_rate.unwrap_or(44100);
    let channels = codec_params
        .channels
        .map(|c| c.count())
        .unwrap_or(2);

    let duration = codec_params
        .n_frames
        .map(|n| n as f64 / sample_rate as f64)
        .unwrap_or(0.0);

    if duration <= 0.0 {
        // No duration info — fall back to full decode
        return compute_waveform(file_path, num_points);
    }

    let mut decoder = symphonia::default::get_codecs()
        .make(&codec_params, &DecoderOptions::default())
        .map_err(|e| format!("Failed to create decoder: {}", e))?;

    let mut rms_values: Vec<f32> = Vec::with_capacity(num_points);
    let ch = channels.max(1);

    for i in 0..num_points {
        let target_secs = (i as f64) * duration / (num_points as f64);
        let time = Time {
            seconds: target_secs as u64,
            frac: target_secs.fract(),
        };

        // Seek to the target position
        if reader.seek(SeekMode::Coarse, SeekTo::Time { time, track_id: Some(track_id) }).is_err() {
            // Seeking not supported — fall back to full decode
            return compute_waveform(file_path, num_points);
        }

        decoder.reset();

        // Decode one packet at this position
        let rms = match reader.next_packet() {
            Ok(packet) if packet.track_id() == track_id => {
                match decoder.decode(&packet) {
                    Ok(decoded) => {
                        let spec = *decoded.spec();
                        let num_frames = decoded.frames();
                        let mut sample_buf =
                            SampleBuffer::<f32>::new(num_frames as u64, spec);
                        sample_buf.copy_interleaved_ref(decoded);
                        let samples = sample_buf.samples();

                        // Compute RMS of downmixed mono
                        let mut sum_sq = 0.0f64;
                        let mut count = 0usize;
                        for frame_idx in 0..num_frames {
                            let mut mono = 0.0f32;
                            for c in 0..ch {
                                let idx = frame_idx * ch + c;
                                if idx < samples.len() {
                                    mono += samples[idx];
                                }
                            }
                            mono /= ch as f32;
                            sum_sq += (mono as f64) * (mono as f64);
                            count += 1;
                        }
                        if count > 0 {
                            (sum_sq / count as f64).sqrt() as f32
                        } else {
                            0.0
                        }
                    }
                    Err(_) => 0.0,
                }
            }
            _ => 0.0,
        };

        rms_values.push(rms);
    }

    if rms_values.iter().all(|&v| v == 0.0) {
        return Ok(WaveformData {
            points: vec![0.0; num_points],
            duration,
            sample_rate,
            suggested_momentum: None,
        });
    }

    // Normalize to 0.0-1.0
    let max_val = rms_values.iter().cloned().fold(0.0f32, f32::max);
    if max_val > 0.0 {
        for v in &mut rms_values {
            *v /= max_val;
        }
    }

    // Smooth with 3-point moving average
    let mut smoothed = vec![0.0f32; rms_values.len()];
    for i in 0..rms_values.len() {
        let mut sum = rms_values[i];
        let mut count = 1.0f32;
        if i > 0 {
            sum += rms_values[i - 1];
            count += 1.0;
        }
        if i + 1 < rms_values.len() {
            sum += rms_values[i + 1];
            count += 1.0;
        }
        smoothed[i] = sum / count;
    }

    // Detect momentum point
    let suggested_momentum = detect_momentum_point(&smoothed, duration);

    Ok(WaveformData {
        points: smoothed,
        duration,
        sample_rate,
        suggested_momentum,
    })
}

/// Detect a good momentum (start) point in the waveform.
/// Looks for the first significant rise after a quiet section, skipping the first 5%.
fn detect_momentum_point(points: &[f32], duration: f64) -> Option<f64> {
    if points.len() < 10 || duration <= 0.0 {
        return None;
    }

    let skip = (points.len() as f64 * 0.05).ceil() as usize;
    let window_size = (points.len() / 20).max(3); // 5% window for quiet detection
    let quiet_threshold = 0.15;
    let gradient_threshold = 0.05;

    for i in (skip + window_size)..points.len().saturating_sub(1) {
        // Check if preceding window was quiet
        let window_start = i.saturating_sub(window_size);
        let window_avg: f32 =
            points[window_start..i].iter().sum::<f32>() / (i - window_start) as f32;

        if window_avg >= quiet_threshold {
            continue;
        }

        // Check if gradient is significantly positive
        let gradient = points[i + 1] - points[i];
        if gradient > gradient_threshold {
            // Found a rise after quiet section
            let timestamp = (i as f64 / points.len() as f64) * duration;
            return Some(timestamp);
        }
    }

    None
}

/// A single disk-cache entry: waveform data + file modification timestamp for invalidation.
#[derive(Clone, Serialize, Deserialize)]
struct CacheEntry {
    data: WaveformData,
    file_modified: u64, // seconds since epoch
}

/// In-memory cache for waveform data with LRU eviction and disk persistence.
pub struct WaveformCache {
    entries: HashMap<String, CacheEntry>,
    access_order: Vec<String>,
    max_entries: usize,
    disk_path: Option<PathBuf>,
}

impl WaveformCache {
    #[allow(dead_code)]
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            access_order: Vec::new(),
            max_entries,
            disk_path: None,
        }
    }

    /// Create a cache and load persisted entries from disk.
    pub fn new_with_disk(max_entries: usize, disk_path: PathBuf) -> Self {
        let mut cache = Self {
            entries: HashMap::new(),
            access_order: Vec::new(),
            max_entries,
            disk_path: Some(disk_path),
        };
        cache.load_from_disk();
        cache
    }

    pub fn get(&mut self, key: &str) -> Option<&WaveformData> {
        // Check if the source file has changed since caching
        if let Some(entry) = self.entries.get(key) {
            let current_mtime = file_mtime(key);
            if current_mtime != entry.file_modified {
                // File changed — invalidate
                self.entries.remove(key);
                self.access_order.retain(|k| k != key);
                return None;
            }
        }

        if self.entries.contains_key(key) {
            // Move to end (most recently used)
            self.access_order.retain(|k| k != key);
            self.access_order.push(key.to_string());
            self.entries.get(key).map(|e| &e.data)
        } else {
            None
        }
    }

    pub fn insert(&mut self, key: String, data: WaveformData) {
        if self.entries.len() >= self.max_entries && !self.entries.contains_key(&key) {
            // Evict least recently used
            if let Some(oldest) = self.access_order.first().cloned() {
                self.entries.remove(&oldest);
                self.access_order.remove(0);
            }
        }
        let file_modified = file_mtime(&key);
        self.access_order.retain(|k| k != &key);
        self.access_order.push(key.clone());
        self.entries.insert(key, CacheEntry { data, file_modified });
        self.save_to_disk();
    }

    /// Load cache entries from disk (best-effort).
    fn load_from_disk(&mut self) {
        let path = match &self.disk_path {
            Some(p) => p.clone(),
            None => return,
        };
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return,
        };

        #[derive(Deserialize)]
        struct DiskCache {
            entries: HashMap<String, CacheEntry>,
            access_order: Vec<String>,
        }

        if let Ok(disk) = serde_json::from_str::<DiskCache>(&content) {
            self.entries = disk.entries;
            self.access_order = disk.access_order;
            // Trim to max_entries (keep most recent)
            while self.entries.len() > self.max_entries {
                if let Some(oldest) = self.access_order.first().cloned() {
                    self.entries.remove(&oldest);
                    self.access_order.remove(0);
                } else {
                    break;
                }
            }
            tracing::info!("Loaded {} waveform cache entries from disk", self.entries.len());
        }
    }

    /// Persist cache to disk (atomic write: tmp + rename).
    fn save_to_disk(&self) {
        let path = match &self.disk_path {
            Some(p) => p.clone(),
            None => return,
        };

        #[derive(Serialize)]
        struct DiskCache<'a> {
            entries: &'a HashMap<String, CacheEntry>,
            access_order: &'a Vec<String>,
        }

        let disk = DiskCache {
            entries: &self.entries,
            access_order: &self.access_order,
        };

        let tmp = path.with_extension("json.tmp");
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match serde_json::to_string(&disk) {
            Ok(json) => {
                if std::fs::write(&tmp, &json).is_ok() {
                    let _ = std::fs::rename(&tmp, &path);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to serialize waveform cache: {}", e);
            }
        }
    }
}

/// Get file modification time as seconds since epoch (0 if unavailable).
fn file_mtime(path: &str) -> u64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
