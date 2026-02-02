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
    /// Uses equal-power crossfade (sine/cosine curve) so the combined perceived
    /// loudness stays constant throughout the transition. This avoids the volume
    /// dip that a linear crossfade causes at the midpoint.
    pub fn get_volumes(&self) -> (f32, f32) {
        let elapsed = self.start_time.elapsed();
        let duration_ms = self.duration.as_millis() as f32;
        if duration_ms <= 0.0 {
            return (0.0, 1.0);
        }
        let progress = (elapsed.as_millis() as f32 / duration_ms).clamp(0.0, 1.0);

        // Equal-power: cos/sin amplitudes (power = amplitude², so cos²+sin² = 1.0)
        let half_pi = std::f32::consts::FRAC_PI_2;
        let outgoing_vol = (half_pi * progress).cos();
        let incoming_vol = (half_pi * progress).sin();

        (outgoing_vol.max(0.0), incoming_vol.max(0.0))
    }
}
