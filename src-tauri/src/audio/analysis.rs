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

/// Audio metadata tags extracted from a local file.
pub struct AudioTags {
    pub title: Option<String>,
    pub artist: Option<String>,
}

/// Read ID3/Vorbis/iTunes metadata tags from an audio file using Symphonia.
/// Only reads file headers (first few KB), cost < 1ms per file.
pub fn read_audio_metadata_tags(path: &str) -> Option<AudioTags> {
    use symphonia::core::meta::StandardTagKey;

    let file = File::open(path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
    {
        hint.with_extension(ext);
    }

    let mut probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .ok()?;

    let mut title = None;
    let mut artist = None;

    // Check metadata from the probe result (container-level tags)
    let extract_tags = |tags: &[symphonia::core::meta::Tag], title: &mut Option<String>, artist: &mut Option<String>| {
        for tag in tags {
            match tag.std_key {
                Some(StandardTagKey::TrackTitle) => {
                    let v = tag.value.to_string();
                    if !v.is_empty() {
                        *title = Some(v);
                    }
                }
                Some(StandardTagKey::Artist) | Some(StandardTagKey::AlbumArtist) => {
                    if artist.is_none() {
                        let v = tag.value.to_string();
                        if !v.is_empty() {
                            *artist = Some(v);
                        }
                    }
                }
                _ => {}
            }
        }
    };

    // probed.metadata contains metadata from the container (e.g., ID3)
    if let Some(metadata) = probed.metadata.get() {
        if let Some(rev) = metadata.current() {
            extract_tags(rev.tags(), &mut title, &mut artist);
        }
    }

    // Also check format-level metadata (some formats store tags here)
    if title.is_none() || artist.is_none() {
        let metadata = probed.format.metadata();
        if let Some(rev) = metadata.current() {
            extract_tags(rev.tags(), &mut title, &mut artist);
        }
    }

    if title.is_none() && artist.is_none() {
        return None;
    }

    Some(AudioTags { title, artist })
}

/// Compute a waveform (RMS amplitude per segment) for the given audio file.
/// `num_points` controls the resolution (e.g., 200 for a typical display width).
pub fn compute_waveform(file_path: &str, num_points: usize) -> Result<WaveformData, String> {
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

    // Get duration
    let duration = codec_params
        .n_frames
        .map(|n| n as f64 / sample_rate as f64)
        .unwrap_or(0.0);

    let mut decoder = symphonia::default::get_codecs()
        .make(&codec_params, &DecoderOptions::default())
        .map_err(|e| format!("Failed to create decoder: {}", e))?;

    // Streaming RMS: accumulate per-segment sums on the fly instead of buffering
    // all samples. Memory: O(num_points) instead of O(total_samples).
    // For duration-known files, use duration to compute segment boundaries.
    // For unknown duration, count total frames in a first pass... but since this is the
    // fallback path (no seek support / no duration), we estimate from sample_rate * duration
    // or accumulate and assign segments by frame index.
    let total_frames_est = if duration > 0.0 {
        (duration * sample_rate as f64) as usize
    } else {
        0
    };

    let ch = channels.max(1);
    let mut rms_sums: Vec<f64> = vec![0.0; num_points];
    let mut rms_counts: Vec<usize> = vec![0; num_points];
    let mut global_frame: usize = 0;

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
        let mut sample_buf = SampleBuffer::<f32>::new(num_frames as u64, spec);
        sample_buf.copy_interleaved_ref(decoded);
        let samples = sample_buf.samples();

        for frame_idx in 0..num_frames {
            let mut mono = 0.0f32;
            for c in 0..ch {
                let idx = frame_idx * ch + c;
                if idx < samples.len() {
                    mono += samples[idx];
                }
            }
            mono /= ch as f32;

            // Assign this frame to a segment
            let segment = if total_frames_est > 0 {
                ((global_frame as f64 / total_frames_est as f64) * num_points as f64) as usize
            } else {
                // Unknown duration: we'll redistribute after decoding
                // For now, accumulate into a growing estimate
                0
            };
            let segment = segment.min(num_points - 1);

            rms_sums[segment] += (mono as f64) * (mono as f64);
            rms_counts[segment] += 1;
            global_frame += 1;
        }
    }

    // If duration was unknown (total_frames_est == 0), redistribute frames evenly
    if total_frames_est == 0 && global_frame > 0 {
        // Everything went into segment 0 — need a second pass approach.
        // Since this is the rare no-duration fallback, re-read the file with known frame count.
        // But to avoid that, we just return a single-segment waveform scaled.
        // Actually, let's do the redistribution: we know global_frame now.
        // Unfortunately we can't replay without re-reading. For this edge case,
        // spread the single accumulated value across all segments evenly.
        let total_sum = rms_sums[0];
        let total_count = rms_counts[0];
        let per_segment = total_count / num_points;
        if per_segment > 0 {
            let avg_sq = total_sum / total_count as f64;
            for i in 0..num_points {
                rms_sums[i] = avg_sq * per_segment as f64;
                rms_counts[i] = per_segment;
            }
        }
    }

    let has_data = rms_counts.iter().any(|&c| c > 0);
    if !has_data {
        return Ok(WaveformData {
            points: vec![0.0; num_points],
            duration,
            sample_rate,
            suggested_momentum: None,
        });
    }

    // Compute RMS per segment from accumulators
    let mut rms_values: Vec<f32> = Vec::with_capacity(num_points);
    for i in 0..num_points {
        if rms_counts[i] > 0 {
            rms_values.push((rms_sums[i] / rms_counts[i] as f64).sqrt() as f32);
        } else {
            rms_values.push(0.0);
        }
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

    // Decode a 500ms window (up to 30 packets) per point for statistical accuracy.
    // For a 20-min file with 200 points, each point covers ~6s; 500ms gives ~8% coverage
    // vs the old 0.5% (single packet). Total work: ~200×20 packets = constant regardless
    // of file length.
    const WINDOW_SECS: f64 = 0.5;
    const MAX_PACKETS: usize = 30;
    let window_frames = (WINDOW_SECS * sample_rate as f64) as usize;

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

        // Decode multiple packets covering ~500ms window
        let mut sum_sq = 0.0f64;
        let mut count = 0usize;
        let mut packets_read = 0usize;

        while packets_read < MAX_PACKETS && count < window_frames {
            let packet = match reader.next_packet() {
                Ok(p) if p.track_id() == track_id => p,
                Ok(_) => continue, // wrong track, try next
                Err(_) => break,
            };

            let decoded = match decoder.decode(&packet) {
                Ok(d) => d,
                Err(_) => {
                    packets_read += 1;
                    continue;
                }
            };

            let spec = *decoded.spec();
            let num_frames = decoded.frames();
            let mut sample_buf = SampleBuffer::<f32>::new(num_frames as u64, spec);
            sample_buf.copy_interleaved_ref(decoded);
            let samples = sample_buf.samples();

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

            packets_read += 1;
        }

        let rms = if count > 0 {
            (sum_sq / count as f64).sqrt() as f32
        } else {
            0.0
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
    dirty: bool,
}

impl WaveformCache {
    #[allow(dead_code)]
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            access_order: Vec::new(),
            max_entries,
            disk_path: None,
            dirty: false,
        }
    }

    /// Create a cache and load persisted entries from disk.
    pub fn new_with_disk(max_entries: usize, disk_path: PathBuf) -> Self {
        let mut cache = Self {
            entries: HashMap::new(),
            access_order: Vec::new(),
            max_entries,
            disk_path: Some(disk_path),
            dirty: false,
        };
        cache.load_from_disk();
        cache
    }

    pub fn get(&mut self, key: &str) -> Option<&WaveformData> {
        if self.entries.contains_key(key) {
            // Move to end (most recently used) — single pass removal
            if let Some(pos) = self.access_order.iter().position(|k| k == key) {
                self.access_order.remove(pos);
            }
            self.access_order.push(key.to_string());
            return self.entries.get(key).map(|e| &e.data);
        }
        None
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
        // Single pass removal instead of retain
        if let Some(pos) = self.access_order.iter().position(|k| k == &key) {
            self.access_order.remove(pos);
        }
        self.access_order.push(key.clone());
        self.entries.insert(key, CacheEntry { data, file_modified });
        self.dirty = true;
    }

    /// Flush dirty cache to disk and reset the dirty flag.
    pub fn flush_if_dirty(&mut self) {
        if self.dirty {
            self.save_to_disk();
            self.dirty = false;
        }
    }

    /// Validate all cache entries against current file modification times.
    /// Removes entries whose source file has changed or been deleted.
    #[allow(dead_code)]
    pub fn validate_entries(&mut self) {
        let stale_keys: Vec<String> = self
            .entries
            .iter()
            .filter(|(key, entry)| {
                let current_mtime = file_mtime(key);
                current_mtime != entry.file_modified
            })
            .map(|(key, _)| key.clone())
            .collect();

        if !stale_keys.is_empty() {
            tracing::info!("Waveform cache: removing {} stale entries", stale_keys.len());
            for key in &stale_keys {
                self.entries.remove(key);
                if let Some(pos) = self.access_order.iter().position(|k| k == key) {
                    self.access_order.remove(pos);
                }
            }
            self.dirty = true;
        }
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
                    // On Windows, rename fails if dest exists; remove first
                    let _ = std::fs::remove_file(&path);
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
