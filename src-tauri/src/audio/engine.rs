use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait};
use rodio::{OutputStream, OutputStreamHandle};

use crate::audio::track::AudioTrack;
use crate::types::{SoundId, TrackId};

/// Info needed to resume a track after device switch.
struct TrackResumeInfo {
    track_id: TrackId,
    sound_id: SoundId,
    file_path: String,
    position: f64,
    sound_volume: f32,
    track_volume: f32,
}

/// Commands sent to the audio thread.
pub enum AudioCommand {
    PlaySound {
        track_id: TrackId,
        sound_id: SoundId,
        file_path: String,
        start_position: f64,
        sound_volume: f32,
        crossfade_duration_ms: u32,
    },
    StopTrack {
        track_id: TrackId,
    },
    StopAll,
    SetMasterVolume {
        volume: f32,
    },
    SetTrackVolume {
        track_id: TrackId,
        volume: f32,
    },
    SetSoundVolume {
        track_id: TrackId,
        sound_id: SoundId,
        volume: f32,
    },
    CreateTrack {
        track_id: TrackId,
    },
    RemoveTrack {
        track_id: TrackId,
    },
    SetAudioDevice {
        device_name: Option<String>,
    },
    Shutdown,
}

/// Events emitted by the audio thread.
#[derive(Debug, Clone)]
pub enum AudioEvent {
    SoundStarted {
        track_id: TrackId,
        sound_id: SoundId,
    },
    SoundEnded {
        track_id: TrackId,
        sound_id: SoundId,
    },
    PlaybackProgress {
        track_id: TrackId,
        position: f64,
    },
    Error {
        message: String,
    },
}

/// Handle to communicate with the audio engine thread.
#[derive(Clone)]
pub struct AudioEngineHandle {
    command_tx: Sender<AudioCommand>,
    pub events: Arc<Mutex<Vec<AudioEvent>>>,
    pub master_volume: Arc<Mutex<f32>>,
    pub last_trigger_time: Arc<Mutex<Instant>>,
}

impl AudioEngineHandle {
    /// Start the audio engine in a separate thread and return a handle.
    pub fn new(initial_device: Option<String>) -> Result<Self, String> {
        let (command_tx, command_rx) = mpsc::channel::<AudioCommand>();
        let events = Arc::new(Mutex::new(Vec::<AudioEvent>::new()));
        let master_volume = Arc::new(Mutex::new(0.8f32));
        let last_trigger_time = Arc::new(Mutex::new(
            Instant::now() - Duration::from_secs(10),
        ));

        let events_clone = events.clone();
        let master_volume_clone = master_volume.clone();

        // Spawn audio thread
        thread::spawn(move || {
            audio_thread_main(command_rx, events_clone, master_volume_clone, initial_device);
        });

        Ok(Self {
            command_tx,
            events,
            master_volume,
            last_trigger_time,
        })
    }

    /// Send a command to the audio thread.
    pub fn send_command(&self, cmd: AudioCommand) -> Result<(), String> {
        self.command_tx
            .send(cmd)
            .map_err(|e| format!("Failed to send audio command: {}", e))
    }

    /// Play a sound on a track.
    pub fn play_sound(
        &self,
        track_id: TrackId,
        sound_id: SoundId,
        file_path: String,
        start_position: f64,
        sound_volume: f32,
        crossfade_duration_ms: u32,
    ) -> Result<(), String> {
        self.send_command(AudioCommand::PlaySound {
            track_id,
            sound_id,
            file_path,
            start_position,
            sound_volume,
            crossfade_duration_ms,
        })
    }

    /// Stop a specific track.
    pub fn stop_track(&self, track_id: TrackId) -> Result<(), String> {
        self.send_command(AudioCommand::StopTrack { track_id })
    }

    /// Stop all tracks.
    pub fn stop_all(&self) -> Result<(), String> {
        self.send_command(AudioCommand::StopAll)
    }

    /// Set master volume.
    pub fn set_master_volume(&self, volume: f32) -> Result<(), String> {
        *self.master_volume.lock().unwrap() = volume;
        self.send_command(AudioCommand::SetMasterVolume { volume })
    }

    /// Set track volume.
    pub fn set_track_volume(&self, track_id: TrackId, volume: f32) -> Result<(), String> {
        self.send_command(AudioCommand::SetTrackVolume { track_id, volume })
    }

    /// Set sound volume (updates currently playing sound's volume).
    pub fn set_sound_volume(&self, track_id: TrackId, sound_id: SoundId, volume: f32) -> Result<(), String> {
        self.send_command(AudioCommand::SetSoundVolume { track_id, sound_id, volume })
    }

    /// Create a new track.
    pub fn create_track(&self, track_id: TrackId) -> Result<(), String> {
        self.send_command(AudioCommand::CreateTrack { track_id })
    }

    /// Remove a track.
    pub fn remove_track(&self, track_id: TrackId) -> Result<(), String> {
        self.send_command(AudioCommand::RemoveTrack { track_id })
    }

    /// Drain pending events from the audio thread.
    pub fn drain_events(&self) -> Vec<AudioEvent> {
        let mut events = self.events.lock().unwrap();
        events.drain(..).collect()
    }

    /// Check if the cooldown has elapsed.
    pub fn check_cooldown(&self, cooldown_ms: u32) -> bool {
        let last = self.last_trigger_time.lock().unwrap();
        last.elapsed() >= Duration::from_millis(cooldown_ms as u64)
    }

    /// Update the last trigger time.
    pub fn update_trigger_time(&self) {
        *self.last_trigger_time.lock().unwrap() = Instant::now();
    }

    /// Set the audio output device. None = follow system default.
    pub fn set_audio_device(&self, device_name: Option<String>) -> Result<(), String> {
        self.send_command(AudioCommand::SetAudioDevice { device_name })
    }

    /// Shutdown the audio engine.
    pub fn shutdown(&self) {
        let _ = self.send_command(AudioCommand::Shutdown);
    }
}

/// List available audio output devices.
pub fn list_audio_devices() -> Vec<String> {
    let host = cpal::default_host();
    let mut devices = Vec::new();
    if let Ok(output_devices) = host.output_devices() {
        for device in output_devices {
            if let Ok(name) = device.name() {
                devices.push(name);
            }
        }
    }
    devices
}

impl Drop for AudioEngineHandle {
    fn drop(&mut self) {
        let _ = self.command_tx.send(AudioCommand::Shutdown);
    }
}

/// Create an OutputStream for the given device name, or the system default if None.
fn create_output_stream(device_name: &Option<String>) -> Result<(OutputStream, OutputStreamHandle), String> {
    match device_name {
        Some(name) => {
            let host = cpal::default_host();
            let device = host
                .output_devices()
                .map_err(|e| format!("Failed to enumerate devices: {}", e))?
                .find(|d| d.name().map(|n| n == *name).unwrap_or(false))
                .ok_or_else(|| format!("Audio device '{}' not found", name))?;
            OutputStream::try_from_device(&device)
                .map_err(|e| format!("Failed to open device '{}': {}", name, e))
        }
        None => {
            OutputStream::try_default()
                .map_err(|e| format!("Failed to open default audio device: {}", e))
        }
    }
}

/// Get the name of the current default output device.
fn get_default_device_name() -> Option<String> {
    let host = cpal::default_host();
    host.default_output_device()
        .and_then(|d| d.name().ok())
}

/// The main loop of the audio thread.
fn audio_thread_main(
    command_rx: Receiver<AudioCommand>,
    events: Arc<Mutex<Vec<AudioEvent>>>,
    master_volume: Arc<Mutex<f32>>,
    initial_device: Option<String>,
) {
    // Initialize audio output
    let (mut _stream, mut stream_handle) = match create_output_stream(&initial_device) {
        Ok(s) => s,
        Err(e) => {
            emit_event(
                &events,
                AudioEvent::Error {
                    message: format!("Failed to initialize audio output: {}", e),
                },
            );
            return;
        }
    };

    let mut tracks: HashMap<TrackId, AudioTrack> = HashMap::new();
    let mut sound_volumes: HashMap<SoundId, f32> = HashMap::new();
    let mut track_sounds: HashMap<TrackId, SoundId> = HashMap::new();

    // Device management state
    let mut current_device: Option<String> = initial_device;
    let mut last_default_device_name: Option<String> = get_default_device_name();
    let mut last_device_check = Instant::now();
    let device_check_interval = Duration::from_secs(3);

    // Progress emission timer
    let mut last_progress_emit = Instant::now();
    let progress_interval = Duration::from_millis(100);

    loop {
        // Use shorter timeout when actively playing (crossfade needs ~60fps),
        // longer timeout when idle to reduce CPU usage
        let has_active_playback = tracks.values().any(|t| t.is_playing() || t.crossfade.is_some());
        let timeout = if has_active_playback {
            Duration::from_millis(16)
        } else {
            Duration::from_millis(200)
        };

        // Process commands (non-blocking with timeout)
        match command_rx.recv_timeout(timeout) {
            Ok(AudioCommand::PlaySound {
                track_id,
                sound_id,
                file_path,
                start_position,
                sound_volume,
                crossfade_duration_ms,
            }) => {
                let mv = *master_volume.lock().unwrap();

                // Create track if it doesn't exist
                if !tracks.contains_key(&track_id) {
                    if tracks.len() >= 20 {
                        emit_event(
                            &events,
                            AudioEvent::Error {
                                message: "Maximum number of tracks (20) reached".to_string(),
                            },
                        );
                        continue;
                    }
                    tracks.insert(
                        track_id.clone(),
                        AudioTrack::new(track_id.clone(), stream_handle.clone()),
                    );
                }

                if let Some(track) = tracks.get_mut(&track_id) {
                    sound_volumes.insert(sound_id.clone(), sound_volume);
                    track_sounds.insert(track_id.clone(), sound_id.clone());

                    match track.play(
                        sound_id.clone(),
                        &file_path,
                        start_position,
                        sound_volume,
                        mv,
                        crossfade_duration_ms,
                    ) {
                        Ok(()) => {
                            emit_event(
                                &events,
                                AudioEvent::SoundStarted {
                                    track_id: track_id.clone(),
                                    sound_id,
                                },
                            );
                        }
                        Err(e) => {
                            emit_event(&events, AudioEvent::Error { message: e });
                        }
                    }
                }
            }
            Ok(AudioCommand::StopTrack { track_id }) => {
                if let Some(track) = tracks.get_mut(&track_id) {
                    let sound_id = track.currently_playing.clone();
                    track.stop();
                    if let Some(sid) = sound_id {
                        emit_event(
                            &events,
                            AudioEvent::SoundEnded {
                                track_id: track_id.clone(),
                                sound_id: sid,
                            },
                        );
                    }
                }
            }
            Ok(AudioCommand::StopAll) => {
                for (tid, track) in tracks.iter_mut() {
                    let sound_id = track.currently_playing.clone();
                    track.stop();
                    if let Some(sid) = sound_id {
                        emit_event(
                            &events,
                            AudioEvent::SoundEnded {
                                track_id: tid.clone(),
                                sound_id: sid,
                            },
                        );
                    }
                }
            }
            Ok(AudioCommand::SetMasterVolume { volume }) => {
                let mv = volume;
                for (tid, track) in tracks.iter_mut() {
                    let sv = track_sounds
                        .get(tid)
                        .and_then(|sid| sound_volumes.get(sid))
                        .copied()
                        .unwrap_or(1.0);
                    track.update_master_volume(sv, mv);
                }
            }
            Ok(AudioCommand::SetTrackVolume { track_id, volume }) => {
                if let Some(track) = tracks.get_mut(&track_id) {
                    let mv = *master_volume.lock().unwrap();
                    let sv = track_sounds
                        .get(&track_id)
                        .and_then(|sid| sound_volumes.get(sid))
                        .copied()
                        .unwrap_or(1.0);
                    track.set_volume(volume, sv, mv);
                }
            }
            Ok(AudioCommand::SetSoundVolume { track_id, sound_id, volume }) => {
                // Update stored sound volume
                sound_volumes.insert(sound_id, volume);
                // Recalculate final volume on the track's sink
                if let Some(track) = tracks.get_mut(&track_id) {
                    let mv = *master_volume.lock().unwrap();
                    if track.crossfade.is_none() {
                        track.update_master_volume(volume, mv);
                    }
                }
            }
            Ok(AudioCommand::CreateTrack { track_id }) => {
                if tracks.len() < 20 && !tracks.contains_key(&track_id) {
                    tracks.insert(
                        track_id.clone(),
                        AudioTrack::new(track_id, stream_handle.clone()),
                    );
                }
            }
            Ok(AudioCommand::RemoveTrack { track_id }) => {
                if let Some(mut track) = tracks.remove(&track_id) {
                    track.stop();
                }
            }
            Ok(AudioCommand::SetAudioDevice { device_name }) => {
                // Capture resume info for all playing tracks
                let mut resume_list: Vec<TrackResumeInfo> = Vec::new();
                let mv = *master_volume.lock().unwrap();
                for (tid, track) in tracks.iter() {
                    if let (Some(sid), Some(fp)) = (&track.currently_playing, &track.file_path) {
                        if track.is_playing() {
                            let sv = sound_volumes.get(sid).copied().unwrap_or(1.0);
                            resume_list.push(TrackResumeInfo {
                                track_id: tid.clone(),
                                sound_id: sid.clone(),
                                file_path: fp.clone(),
                                position: track.get_position(),
                                sound_volume: sv,
                                track_volume: track.volume,
                            });
                        }
                    }
                }

                // Stop and clear all tracks
                for (_, track) in tracks.iter_mut() {
                    track.stop();
                }
                tracks.clear();

                // Rebuild the output stream
                match create_output_stream(&device_name) {
                    Ok((new_stream, new_handle)) => {
                        _stream = new_stream;
                        stream_handle = new_handle;
                        current_device = device_name;
                        last_default_device_name = get_default_device_name();
                    }
                    Err(e) => {
                        emit_event(&events, AudioEvent::Error {
                            message: format!("Failed to switch audio device: {}", e),
                        });
                        // Try falling back to default
                        if let Ok((new_stream, new_handle)) = create_output_stream(&None) {
                            _stream = new_stream;
                            stream_handle = new_handle;
                            current_device = None;
                            last_default_device_name = get_default_device_name();
                        }
                        // Cannot resume on failed switch
                        continue;
                    }
                }

                // Resume playback on the new device
                for info in resume_list {
                    let mut new_track = AudioTrack::new(info.track_id.clone(), stream_handle.clone());
                    new_track.volume = info.track_volume;
                    match new_track.play(
                        info.sound_id.clone(),
                        &info.file_path,
                        info.position,
                        info.sound_volume,
                        mv,
                        0, // no crossfade for resume
                    ) {
                        Ok(()) => {}
                        Err(e) => {
                            emit_event(&events, AudioEvent::Error {
                                message: format!("Failed to resume track {}: {}", info.track_id, e),
                            });
                        }
                    }
                    tracks.insert(info.track_id.clone(), new_track);
                    sound_volumes.insert(info.sound_id.clone(), info.sound_volume);
                    track_sounds.insert(info.track_id, info.sound_id);
                }
            }
            Ok(AudioCommand::Shutdown) => {
                // Stop all sounds and exit
                for (_, track) in tracks.iter_mut() {
                    track.stop();
                }
                break;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // No command, continue with maintenance
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                // Channel closed, shut down
                break;
            }
        }

        // Poll for default device changes (only when following system default)
        if current_device.is_none() && last_device_check.elapsed() >= device_check_interval {
            last_device_check = Instant::now();
            let new_default = get_default_device_name();
            if new_default != last_default_device_name {
                last_default_device_name = new_default;

                // Capture resume info for all playing tracks
                let mut resume_list: Vec<TrackResumeInfo> = Vec::new();
                let mv = *master_volume.lock().unwrap();
                for (tid, track) in tracks.iter() {
                    if let (Some(sid), Some(fp)) = (&track.currently_playing, &track.file_path) {
                        if track.is_playing() {
                            let sv = sound_volumes.get(sid).copied().unwrap_or(1.0);
                            resume_list.push(TrackResumeInfo {
                                track_id: tid.clone(),
                                sound_id: sid.clone(),
                                file_path: fp.clone(),
                                position: track.get_position(),
                                sound_volume: sv,
                                track_volume: track.volume,
                            });
                        }
                    }
                }

                // Stop and clear all tracks
                for (_, track) in tracks.iter_mut() {
                    track.stop();
                }
                tracks.clear();

                // Rebuild output on new default device
                if let Ok((new_stream, new_handle)) = create_output_stream(&None) {
                    _stream = new_stream;
                    stream_handle = new_handle;

                    // Resume playback
                    for info in resume_list {
                        let mut new_track = AudioTrack::new(info.track_id.clone(), stream_handle.clone());
                        new_track.volume = info.track_volume;
                        match new_track.play(
                            info.sound_id.clone(),
                            &info.file_path,
                            info.position,
                            info.sound_volume,
                            mv,
                            0,
                        ) {
                            Ok(()) => {}
                            Err(e) => {
                                emit_event(&events, AudioEvent::Error {
                                    message: format!("Failed to resume track {}: {}", info.track_id, e),
                                });
                            }
                        }
                        tracks.insert(info.track_id.clone(), new_track);
                        sound_volumes.insert(info.sound_id.clone(), info.sound_volume);
                        track_sounds.insert(info.track_id, info.sound_id);
                    }
                }
            }
        }

        // Update crossfades
        let mv = *master_volume.lock().unwrap();
        for (tid, track) in tracks.iter_mut() {
            if track.crossfade.is_some() {
                let sv = track_sounds
                    .get(tid)
                    .and_then(|sid| sound_volumes.get(sid))
                    .copied()
                    .unwrap_or(1.0);
                track.update_crossfade(sv, mv);
            }
        }

        // Check for finished sounds
        let mut finished = Vec::new();
        for (tid, track) in tracks.iter() {
            if track.has_finished() {
                if let Some(ref sid) = track.currently_playing {
                    finished.push((tid.clone(), sid.clone()));
                }
            }
        }
        for (tid, sid) in finished {
            if let Some(track) = tracks.get_mut(&tid) {
                track.currently_playing = None;
                track.start_time = None;
            }
            emit_event(
                &events,
                AudioEvent::SoundEnded {
                    track_id: tid,
                    sound_id: sid,
                },
            );
        }

        // Emit progress events
        if last_progress_emit.elapsed() >= progress_interval {
            last_progress_emit = Instant::now();
            for (tid, track) in tracks.iter() {
                if track.is_playing() {
                    emit_event(
                        &events,
                        AudioEvent::PlaybackProgress {
                            track_id: tid.clone(),
                            position: track.get_position(),
                        },
                    );
                }
            }
        }
    }
}

fn emit_event(events: &Arc<Mutex<Vec<AudioEvent>>>, event: AudioEvent) {
    if let Ok(mut events) = events.lock() {
        events.push(event);
    }
}
