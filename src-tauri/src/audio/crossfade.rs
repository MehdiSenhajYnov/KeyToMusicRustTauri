use std::time::{Duration, Instant};

/// State of a crossfade transition between two sounds on the same track.
pub struct CrossfadeState {
    pub start_time: Instant,
    pub duration: Duration,
}

impl CrossfadeState {
    pub fn new(duration_ms: u32) -> Self {
        Self {
            start_time: Instant::now(),
            duration: Duration::from_millis(duration_ms as u64),
        }
    }

    /// Returns true if the crossfade is complete.
    pub fn is_complete(&self) -> bool {
        self.start_time.elapsed() >= self.duration
    }

    /// Calculate volumes for outgoing and incoming sounds based on elapsed time.
    /// Returns (outgoing_volume, incoming_volume) both in 0.0..1.0 range.
    ///
    /// Curve:
    /// - 0-35%: outgoing fades from 100% to 30%
    /// - 35-65%: outgoing fades from 30% to 0%, incoming fades from 0% to 30%
    /// - 65-100%: incoming fades from 30% to 100%
    pub fn get_volumes(&self) -> (f32, f32) {
        let elapsed = self.start_time.elapsed();
        let progress = (elapsed.as_millis() as f32) / (self.duration.as_millis() as f32);
        let progress = progress.clamp(0.0, 1.0);

        let outgoing_vol = if progress < 0.35 {
            // 100% -> 30%
            1.0 - (progress / 0.35) * 0.7
        } else if progress < 0.65 {
            // 30% -> 0%
            0.3 - ((progress - 0.35) / 0.3) * 0.3
        } else {
            0.0
        };

        let incoming_vol = if progress < 0.35 {
            0.0
        } else if progress < 0.65 {
            // 0% -> 30%
            ((progress - 0.35) / 0.3) * 0.3
        } else {
            // 30% -> 100%
            0.3 + ((progress - 0.65) / 0.35) * 0.7
        };

        (outgoing_vol.max(0.0), incoming_vol.max(0.0))
    }
}
