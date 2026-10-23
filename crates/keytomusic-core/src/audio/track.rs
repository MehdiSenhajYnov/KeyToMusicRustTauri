use std::time::{Duration, Instant};

use rodio::{OutputStreamHandle, Sink};

use crate::audio::crossfade::CrossfadeState;
use crate::audio::symphonia_source::SymphoniaSource;
use crate::types::SoundId;

/// Maximum volume change per update step (~16ms at 60fps).
/// Limits to full volume sweep in ~10 updates = ~160ms, which
/// eliminates audible clicks while staying responsive.
const VOLUME_RAMP_STEP: f32 = 0.1;

/// Fade applied to a sink that must be cut (re-trigger during a crossfade),
/// short enough to feel instant but long enough to avoid a click.
const CUT_FADE: Duration = Duration::from_millis(60);

/// A sink ramping down to silence before it is stopped.
struct FadingSink {
    sink: Sink,
    from: f32,
    start: Instant,
}

/// An active audio track that can play one sound at a time with crossfade support.
pub struct AudioTrack {
    pub volume: f32,
    pub currently_playing: Option<SoundId>,
    pub start_time: Option<Instant>,
    pub start_position_secs: f64,
    pub file_path: Option<String>,

    // Primary sink for current playback
    sink: Option<Sink>,
    // Secondary sink used during crossfade (holds the outgoing sound)
    outgoing_sink: Option<Sink>,
    // Crossfade state
    pub crossfade: Option<CrossfadeState>,
    // Sinks cut mid-fade, ramping to silence
    fading: Vec<FadingSink>,

    // Volume ramping state
    current_volume: f32,
    target_volume: f32,

    // Reference to the output stream handle
    stream_handle: OutputStreamHandle,
}

impl AudioTrack {
    pub fn new(stream_handle: OutputStreamHandle) -> Self {
        Self {
            volume: 1.0,
            currently_playing: None,
            start_time: None,
            start_position_secs: 0.0,
            file_path: None,
            sink: None,
            outgoing_sink: None,
            crossfade: None,
            fading: Vec::new(),
            current_volume: 0.0,
            target_volume: 0.0,
            stream_handle,
        }
    }

    /// Advance the volume ramp one step. Call from the audio thread main loop.
    pub fn tick_volume_ramp(&mut self) {
        if self.crossfade.is_some() {
            return;
        }
        if (self.current_volume - self.target_volume).abs() < f32::EPSILON {
            return;
        }
        if let Some(ref sink) = self.sink {
            let diff = self.target_volume - self.current_volume;
            if diff.abs() <= VOLUME_RAMP_STEP {
                self.current_volume = self.target_volume;
            } else if diff > 0.0 {
                self.current_volume += VOLUME_RAMP_STEP;
            } else {
                self.current_volume -= VOLUME_RAMP_STEP;
            }
            sink.set_volume(self.current_volume.max(0.0));
        }
    }

    /// Advance the short fade-outs of cut sinks. Call from the audio thread main loop.
    pub fn tick_fading(&mut self) {
        self.fading.retain(|f| {
            let t = f.start.elapsed().as_secs_f32() / CUT_FADE.as_secs_f32();
            if t >= 1.0 {
                f.sink.stop();
                false
            } else {
                f.sink.set_volume(f.from * (1.0 - t));
                true
            }
        });
    }

    /// True while a crossfade or a cut fade-out needs regular ticks.
    pub fn is_animating(&self) -> bool {
        self.crossfade.is_some() || !self.fading.is_empty()
    }

    fn fade_out(&mut self, sink: Sink, from: f32) {
        self.fading.push(FadingSink {
            sink,
            from,
            start: Instant::now(),
        });
    }

    /// Hand playback over to `new_sink`: crossfade from the current sink's actual gain,
    /// and ramp out (instead of hard-stopping) anything already fading.
    fn hand_over(&mut self, new_sink: &Sink, final_volume: f32, crossfade_duration_ms: u32) {
        let is_playing = self.is_playing();

        if let Some(outgoing) = self.outgoing_sink.take() {
            let from = self
                .crossfade
                .as_ref()
                .map_or(0.0, CrossfadeState::outgoing_gain);
            self.fade_out(outgoing, from);
        }
        self.crossfade = None;

        if is_playing && crossfade_duration_ms > 0 {
            // The current sink becomes the outgoing one, starting from its own gain.
            self.outgoing_sink = self.sink.take();
            self.crossfade = Some(CrossfadeState::new(
                crossfade_duration_ms,
                self.current_volume,
            ));
            new_sink.set_volume(0.0); // incoming starts at 0
            self.current_volume = 0.0;
            self.target_volume = final_volume;
        } else {
            // No crossfade: cut the current sound (with a short ramp) and play at full.
            if let Some(sink) = self.sink.take() {
                if is_playing {
                    let from = self.current_volume;
                    self.fade_out(sink, from);
                } else {
                    sink.stop();
                }
            }
            new_sink.set_volume(final_volume);
            self.current_volume = final_volume;
            self.target_volume = final_volume;
        }
    }

    /// Play a sound file on this track. If something is already playing, initiates crossfade.
    /// Uses symphonia for instant seeking when start_position > 0.
    pub fn play(
        &mut self,
        sound_id: SoundId,
        file_path: &str,
        start_position_secs: f64,
        sound_volume: f32,
        master_volume: f32,
        crossfade_duration_ms: u32,
    ) -> Result<(), String> {
        // Always use SymphoniaSource for consistent format support (mp3, m4a, ogg, flac, wav)
        let source = SymphoniaSource::new(file_path, start_position_secs)?;
        self.play_prepared(
            sound_id,
            file_path,
            start_position_secs,
            source,
            sound_volume,
            master_volume,
            crossfade_duration_ms,
        )
    }

    /// Play a pre-created source on this track (no file I/O).
    #[allow(clippy::too_many_arguments)]
    pub fn play_prepared(
        &mut self,
        sound_id: SoundId,
        file_path: &str,
        start_position_secs: f64,
        source: SymphoniaSource,
        sound_volume: f32,
        master_volume: f32,
        crossfade_duration_ms: u32,
    ) -> Result<(), String> {
        let new_sink = Sink::try_new(&self.stream_handle)
            .map_err(|e| format!("Failed to create audio sink: {}", e))?;

        let final_volume = sound_volume * self.volume * master_volume;
        self.hand_over(&new_sink, final_volume, crossfade_duration_ms);

        new_sink.append(source);

        self.sink = Some(new_sink);
        self.currently_playing = Some(sound_id);
        self.start_time = Some(Instant::now());
        self.start_position_secs = start_position_secs;
        self.file_path = Some(file_path.to_string());

        Ok(())
    }

    /// Stop the current sound on this track.
    pub fn stop(&mut self) {
        if let Some(ref sink) = self.sink {
            sink.stop();
        }
        if let Some(ref sink) = self.outgoing_sink {
            sink.stop();
        }
        for fading in self.fading.drain(..) {
            fading.sink.stop();
        }
        self.sink = None;
        self.outgoing_sink = None;
        self.crossfade = None;
        self.currently_playing = None;
        self.start_time = None;
        self.file_path = None;
        self.current_volume = 0.0;
        self.target_volume = 0.0;
    }

    /// Set the volume for this track (0.0 to 1.0).
    pub fn set_volume(&mut self, volume: f32, sound_volume: f32, master_volume: f32) {
        self.volume = volume;
        let final_volume = sound_volume * self.volume * master_volume;
        self.target_volume = final_volume;
    }

    /// Update the master volume on the current playback.
    pub fn update_master_volume(&mut self, sound_volume: f32, master_volume: f32) {
        let final_volume = sound_volume * self.volume * master_volume;
        self.target_volume = final_volume;
    }

    /// Check if this track is currently playing.
    pub fn is_playing(&self) -> bool {
        self.sink.as_ref().map(|s| !s.empty()).unwrap_or(false)
    }

    /// Check if the current sound has finished playing.
    pub fn has_finished(&self) -> bool {
        self.sink.as_ref().map(|s| s.empty()).unwrap_or(true) && self.currently_playing.is_some()
    }

    /// Get the current playback position in seconds.
    pub fn get_position(&self) -> f64 {
        if let Some(start_time) = self.start_time {
            self.start_position_secs + start_time.elapsed().as_secs_f64()
        } else {
            0.0
        }
    }

    /// Update crossfade volumes. Returns true if crossfade is still active.
    pub fn update_crossfade(&mut self, sound_volume: f32, master_volume: f32) -> bool {
        if let Some(ref crossfade) = self.crossfade {
            if crossfade.is_complete() {
                // Crossfade done: stop outgoing sink
                if let Some(ref outgoing) = self.outgoing_sink {
                    outgoing.stop();
                }
                self.outgoing_sink = None;
                self.crossfade = None;

                // Set final volume on incoming
                let final_volume = sound_volume * self.volume * master_volume;
                if let Some(ref sink) = self.sink {
                    sink.set_volume(final_volume);
                }
                self.current_volume = final_volume;
                self.target_volume = final_volume;
                return false;
            }

            let (_, in_vol) = crossfade.get_volumes();
            let base_volume = sound_volume * self.volume * master_volume;

            if let Some(ref outgoing) = self.outgoing_sink {
                outgoing.set_volume(crossfade.outgoing_gain());
            }
            if let Some(ref sink) = self.sink {
                sink.set_volume(base_volume * in_vol);
                self.current_volume = base_volume * in_vol;
            }
            return true;
        }
        false
    }
}
