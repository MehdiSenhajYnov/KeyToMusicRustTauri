use std::time::{Duration, Instant};

use rodio::{OutputStreamHandle, Sink};

use crate::audio::crossfade::CrossfadeState;
use crate::audio::symphonia_source::SymphoniaSource;
use crate::types::SoundId;

/// Maximum volume change per update step (~16ms at 60fps).
/// Limits to full volume sweep in ~10 updates = ~160ms, which
/// eliminates audible clicks while staying responsive.
const VOLUME_RAMP_STEP: f32 = 0.1;

/// Ducking moves fast out of the way and comes back gently (full scale per second).
const DUCK_ATTACK_PER_SEC: f32 = 1.0 / 0.12;
const DUCK_RELEASE_PER_SEC: f32 = 1.0 / 0.8;

/// Fade applied to a sink that must be cut (re-trigger during a crossfade),
/// short enough to feel instant but long enough to avoid a click.
const CUT_FADE: Duration = Duration::from_millis(60);

/// A sink ramping down to silence before it is stopped.
struct FadingSink {
    sink: Sink,
    from: f32,
    start: Instant,
    duration: Duration,
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

    // When playback was paused (position and sinks are frozen until resume)
    paused_at: Option<Instant>,

    // Ducking: gain applied on top of everything while another track's
    // effect plays (1.0 = untouched), moving toward `duck_target`.
    duck_gain: f32,
    duck_target: f32,

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
            paused_at: None,
            duck_gain: 1.0,
            duck_target: 1.0,
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
            sink.set_volume((self.current_volume * self.duck_gain).max(0.0));
        }
    }

    /// Where ducking should bring this track (1.0 = not ducked).
    pub fn set_duck_target(&mut self, target: f32) {
        self.duck_target = target.clamp(0.0, 1.0);
    }

    /// Advance the duck gain by `dt` seconds toward its target.
    pub fn tick_duck(&mut self, dt: f32) {
        let diff = self.duck_target - self.duck_gain;
        if diff.abs() < f32::EPSILON {
            return;
        }
        let rate = if diff < 0.0 {
            DUCK_ATTACK_PER_SEC
        } else {
            DUCK_RELEASE_PER_SEC
        };
        let step = rate * dt;
        self.duck_gain = if diff.abs() <= step {
            self.duck_target
        } else {
            self.duck_gain + step.copysign(diff)
        };
        // During a crossfade, update_crossfade applies the gain on every tick.
        if self.crossfade.is_none() {
            if let Some(ref sink) = self.sink {
                sink.set_volume((self.current_volume * self.duck_gain).max(0.0));
            }
        }
    }

    /// Advance the short fade-outs of cut sinks. Call from the audio thread main loop.
    pub fn tick_fading(&mut self) {
        self.fading.retain(|f| {
            let t = f.start.elapsed().as_secs_f32() / f.duration.as_secs_f32();
            if t >= 1.0 {
                f.sink.stop();
                false
            } else {
                f.sink.set_volume(f.from * (1.0 - t));
                true
            }
        });
    }

    /// True while a crossfade, a cut fade-out or ducking needs regular ticks.
    pub fn is_animating(&self) -> bool {
        self.crossfade.is_some()
            || !self.fading.is_empty()
            || (self.duck_gain - self.duck_target).abs() >= f32::EPSILON
    }

    fn fade_out(&mut self, sink: Sink, from: f32, duration: Duration) {
        // Callers pass the undocked gain: start from what is actually heard.
        let from = from * self.duck_gain;
        self.fading.push(FadingSink {
            sink,
            from,
            start: Instant::now(),
            duration,
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
            self.fade_out(outgoing, from, CUT_FADE);
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
                    self.fade_out(sink, from, CUT_FADE);
                } else {
                    sink.stop();
                }
            }
            new_sink.set_volume(final_volume * self.duck_gain);
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
        // A new sound plays at once, even on a track that was frozen.
        self.paused_at = None;

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
        self.paused_at = None;
    }

    /// Stop the current sound, fading it out over `fade` (instantly when
    /// zero or paused: a frozen sink has nothing audible to fade).
    pub fn stop_with_fade(&mut self, fade: Duration) {
        if fade.is_zero() || self.paused_at.is_some() {
            self.stop();
            return;
        }
        if let Some(outgoing) = self.outgoing_sink.take() {
            let from = self
                .crossfade
                .as_ref()
                .map_or(0.0, CrossfadeState::outgoing_gain);
            self.fade_out(outgoing, from, fade);
        }
        if let Some(sink) = self.sink.take() {
            if sink.empty() {
                sink.stop();
            } else {
                let from = self.current_volume;
                self.fade_out(sink, from, fade);
            }
        }
        self.crossfade = None;
        self.currently_playing = None;
        self.start_time = None;
        self.file_path = None;
        self.current_volume = 0.0;
        self.target_volume = 0.0;
    }

    /// Freeze playback where it is. A crossfade in progress completes at once
    /// and cut sounds stop, so that resuming brings back only the current one.
    pub fn pause(&mut self) {
        if self.paused_at.is_some() {
            return;
        }
        if self.crossfade.take().is_some() {
            if let Some(outgoing) = self.outgoing_sink.take() {
                outgoing.stop();
            }
            self.current_volume = self.target_volume;
            if let Some(ref sink) = self.sink {
                sink.set_volume(self.current_volume * self.duck_gain);
            }
        }
        for fading in self.fading.drain(..) {
            fading.sink.stop();
        }
        if let Some(ref sink) = self.sink {
            sink.pause();
        }
        self.paused_at = Some(Instant::now());
    }

    /// Continue after [`AudioTrack::pause`], from the same position.
    pub fn resume(&mut self) {
        let Some(paused_at) = self.paused_at.take() else {
            return;
        };
        if let Some(start_time) = self.start_time {
            self.start_time = Some(start_time + paused_at.elapsed());
        }
        if let Some(ref sink) = self.sink {
            sink.play();
        }
    }

    pub fn is_paused(&self) -> bool {
        self.paused_at.is_some()
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
            let elapsed = match self.paused_at {
                Some(paused_at) => paused_at.saturating_duration_since(start_time),
                None => start_time.elapsed(),
            };
            self.start_position_secs + elapsed.as_secs_f64()
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
                    sink.set_volume(final_volume * self.duck_gain);
                }
                self.current_volume = final_volume;
                self.target_volume = final_volume;
                return false;
            }

            let (_, in_vol) = crossfade.get_volumes();
            let base_volume = sound_volume * self.volume * master_volume;

            if let Some(ref outgoing) = self.outgoing_sink {
                outgoing.set_volume(crossfade.outgoing_gain() * self.duck_gain);
            }
            if let Some(ref sink) = self.sink {
                sink.set_volume(base_volume * in_vol * self.duck_gain);
                self.current_volume = base_volume * in_vol;
            }
            return true;
        }
        false
    }
}
