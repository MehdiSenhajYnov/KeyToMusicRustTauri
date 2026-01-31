use std::time::Instant;

use rodio::{OutputStreamHandle, Sink};

use crate::audio::crossfade::CrossfadeState;
use crate::audio::symphonia_source::SymphoniaSource;
use crate::types::SoundId;

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
            stream_handle,
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
        // Create new sink
        let new_sink = Sink::try_new(&self.stream_handle)
            .map_err(|e| format!("Failed to create audio sink: {}", e))?;

        // Check if something is currently playing for crossfade
        let is_playing = self
            .sink
            .as_ref()
            .map(|s| !s.empty())
            .unwrap_or(false);

        if is_playing && crossfade_duration_ms > 0 {
            // Start crossfade: move current sink to outgoing
            if let Some(outgoing) = self.outgoing_sink.take() {
                outgoing.stop();
            }
            self.outgoing_sink = self.sink.take();

            // Set initial crossfade volumes
            if let Some(ref outgoing) = self.outgoing_sink {
                outgoing.set_volume(sound_volume * self.volume * master_volume);
            }
            new_sink.set_volume(0.0); // incoming starts at 0

            // Initialize crossfade state
            self.crossfade = Some(CrossfadeState::new(crossfade_duration_ms));
        } else {
            // No crossfade, just stop current and play new
            if let Some(ref sink) = self.sink {
                sink.stop();
            }
            self.outgoing_sink = None;
            self.crossfade = None;
            new_sink.set_volume(sound_volume * self.volume * master_volume);
        }

        // Always use SymphoniaSource for consistent format support (mp3, m4a, ogg, flac, wav)
        let source = SymphoniaSource::new(file_path, start_position_secs)?;
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
        self.sink = None;
        self.outgoing_sink = None;
        self.crossfade = None;
        self.currently_playing = None;
        self.start_time = None;
        self.file_path = None;
    }

    /// Set the volume for this track (0.0 to 1.0).
    pub fn set_volume(&mut self, volume: f32, sound_volume: f32, master_volume: f32) {
        self.volume = volume;
        let final_volume = sound_volume * self.volume * master_volume;
        if let Some(ref sink) = self.sink {
            if self.crossfade.is_none() {
                sink.set_volume(final_volume);
            }
        }
    }

    /// Update the master volume on the current playback.
    pub fn update_master_volume(&mut self, sound_volume: f32, master_volume: f32) {
        let final_volume = sound_volume * self.volume * master_volume;
        if let Some(ref sink) = self.sink {
            if self.crossfade.is_none() {
                sink.set_volume(final_volume);
            }
        }
    }

    /// Check if this track is currently playing.
    pub fn is_playing(&self) -> bool {
        self.sink
            .as_ref()
            .map(|s| !s.empty())
            .unwrap_or(false)
    }

    /// Check if the current sound has finished playing.
    pub fn has_finished(&self) -> bool {
        self.sink
            .as_ref()
            .map(|s| s.empty())
            .unwrap_or(true)
            && self.currently_playing.is_some()
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
                if let Some(ref sink) = self.sink {
                    sink.set_volume(sound_volume * self.volume * master_volume);
                }
                return false;
            }

            let (out_vol, in_vol) = crossfade.get_volumes();
            let base_volume = sound_volume * self.volume * master_volume;

            if let Some(ref outgoing) = self.outgoing_sink {
                outgoing.set_volume(base_volume * out_vol);
            }
            if let Some(ref sink) = self.sink {
                sink.set_volume(base_volume * in_vol);
            }
            return true;
        }
        false
    }
}
