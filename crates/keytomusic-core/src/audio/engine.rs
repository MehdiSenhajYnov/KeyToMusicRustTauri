use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait};
use rodio::{OutputStream, OutputStreamHandle};

use crate::audio::symphonia_source::SymphoniaSource;
use crate::audio::track::AudioTrack;
use crate::types::{SoundId, TrackId};

/// Atomically store/load an f32 via AtomicU32.
fn atomic_store_f32(atomic: &AtomicU32, val: f32) {
    atomic.store(val.to_bits(), Ordering::Relaxed);
}

fn atomic_load_f32(atomic: &AtomicU32) -> f32 {
    f32::from_bits(atomic.load(Ordering::Relaxed))
}

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
    /// Fallback: create source on audio thread (used by device resume).
    PlaySound {
        track_id: TrackId,
        sound_id: SoundId,
        file_path: String,
        start_position: f64,
        sound_volume: f32,
        crossfade_duration_ms: u32,
    },
    /// Play a sound with a pre-created source (avoids file I/O on the audio thread).
    PlaySoundPrepared {
        reply: tokio::sync::oneshot::Sender<Result<(), String>>,
        track_id: TrackId,
        sound_id: SoundId,
        file_path: String,
        start_position: f64,
        source: SymphoniaSource,
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
    SetAudioDevice {
        device_name: Option<String>,
    },
    SetErrorSoundPath {
        path: String,
    },
    PlayErrorSound,
    Shutdown,
}

/// Events emitted by the audio thread.
#[derive(Debug, Clone, Copy)]
pub enum SoundEndedReason {
    Finished,
    Stopped,
}

impl SoundEndedReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            SoundEndedReason::Finished => "finished",
            SoundEndedReason::Stopped => "stopped",
        }
    }
}

#[derive(Debug, Clone)]
pub enum AudioEvent {
    SoundStarted {
        track_id: TrackId,
        sound_id: SoundId,
    },
    SoundEnded {
        track_id: TrackId,
        sound_id: SoundId,
        reason: SoundEndedReason,
    },
    PlaybackProgress {
        track_id: TrackId,
        position: f64,
    },
    Error {
        message: String,
    },
}

#[derive(Default)]
struct PlaybackRequests {
    next: u64,
    tracks: HashMap<TrackId, u64>,
}

impl PlaybackRequests {
    fn begin(&mut self, track_id: &str) -> u64 {
        self.next = self.next.wrapping_add(1);
        self.tracks.insert(track_id.to_string(), self.next);
        self.next
    }
    fn is_current(&self, track_id: &str, request: u64) -> bool {
        self.tracks.get(track_id) == Some(&request)
    }
}

/// Handle to communicate with the audio engine thread.
pub struct AudioEngineHandle {
    command_tx: Sender<AudioCommand>,
    playback_requests: Mutex<PlaybackRequests>,
    pub event_rx: Arc<Mutex<Receiver<AudioEvent>>>,
    pub master_volume: Arc<AtomicU32>,
}

impl AudioEngineHandle {
    /// Start the audio engine in a separate thread and return a handle.
    pub fn new(initial_device: Option<String>) -> Result<Self, String> {
        let (command_tx, command_rx) = mpsc::channel::<AudioCommand>();
        let (event_tx, event_rx) = mpsc::channel::<AudioEvent>();
        let master_volume = Arc::new(AtomicU32::new(0.8f32.to_bits()));

        let master_volume_clone = master_volume.clone();

        // Spawn audio thread
        thread::spawn(move || {
            audio_thread_main(command_rx, event_tx, master_volume_clone, initial_device);
        });

        Ok(Self {
            command_tx,
            playback_requests: Mutex::new(PlaybackRequests::default()),
            event_rx: Arc::new(Mutex::new(event_rx)),
            master_volume,
        })
    }

    /// Send a command to the audio thread.
    pub fn send_command(&self, cmd: AudioCommand) -> Result<(), String> {
        self.command_tx
            .send(cmd)
            .map_err(|e| format!("Failed to send audio command: {}", e))
    }

    /// Play a sound on a track (fallback — creates source on audio thread).
    #[allow(dead_code)]
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

    /// Reserve before decoding so Stop and newer requests invalidate slow preparations.
    pub fn begin_playback(&self, track_id: &str) -> u64 {
        self.playback_requests
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .begin(track_id)
    }

    /// Play a pre-created source on a track (no file I/O on the audio thread).
    pub async fn play_sound_prepared(
        &self,
        request: u64,
        track_id: TrackId,
        sound_id: SoundId,
        file_path: String,
        start_position: f64,
        source: SymphoniaSource,
        sound_volume: f32,
        crossfade_duration_ms: u32,
    ) -> Result<(), String> {
        let (reply, response) = tokio::sync::oneshot::channel();
        {
            let requests = self
                .playback_requests
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if !requests.is_current(&track_id, request) {
                return Err("Playback cancelled".into());
            }
            self.send_command(AudioCommand::PlaySoundPrepared {
                reply,
                track_id,
                sound_id,
                file_path,
                start_position,
                source,
                sound_volume,
                crossfade_duration_ms,
            })?;
        }
        response
            .await
            .map_err(|_| "Audio engine stopped before playback could start".to_string())?
    }

    /// Stop a specific track.
    pub fn stop_track(&self, track_id: TrackId) -> Result<(), String> {
        let mut requests = self
            .playback_requests
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        requests.tracks.remove(&track_id);
        self.send_command(AudioCommand::StopTrack { track_id })
    }

    /// Stop all tracks.
    pub fn stop_all(&self) -> Result<(), String> {
        let mut requests = self
            .playback_requests
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        requests.tracks.clear();
        self.send_command(AudioCommand::StopAll)
    }

    /// Set master volume.
    pub fn set_master_volume(&self, volume: f32) -> Result<(), String> {
        atomic_store_f32(&self.master_volume, volume);
        self.send_command(AudioCommand::SetMasterVolume { volume })
    }

    /// Set track volume.
    pub fn set_track_volume(&self, track_id: TrackId, volume: f32) -> Result<(), String> {
        self.send_command(AudioCommand::SetTrackVolume { track_id, volume })
    }

    /// Set sound volume (updates currently playing sound's volume).
    pub fn set_sound_volume(
        &self,
        track_id: TrackId,
        sound_id: SoundId,
        volume: f32,
    ) -> Result<(), String> {
        self.send_command(AudioCommand::SetSoundVolume {
            track_id,
            sound_id,
            volume,
        })
    }

    /// Set the audio output device. None = follow system default.
    pub fn set_audio_device(&self, device_name: Option<String>) -> Result<(), String> {
        self.send_command(AudioCommand::SetAudioDevice { device_name })
    }

    /// Set the path to the error sound file.
    pub fn set_error_sound_path(&self, path: String) -> Result<(), String> {
        self.send_command(AudioCommand::SetErrorSoundPath { path })
    }

    /// Play the error sound (fire-and-forget).
    pub fn play_error_sound(&self) -> Result<(), String> {
        self.send_command(AudioCommand::PlayErrorSound)
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
fn create_output_stream(
    device_name: &Option<String>,
) -> Result<(OutputStream, OutputStreamHandle), String> {
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
        None => OutputStream::try_default()
            .map_err(|e| format!("Failed to open default audio device: {}", e)),
    }
}

/// Get the name of the current default output device.
fn get_default_device_name() -> Option<String> {
    let host = cpal::default_host();
    host.default_output_device().and_then(|d| d.name().ok())
}

fn is_preview_track(track_id: &str) -> bool {
    track_id == "__preview__" || track_id.starts_with("preview-")
}

/// The output stream and the device it should follow.
struct Output {
    stream: Option<(OutputStream, OutputStreamHandle)>,
    /// Device chosen by the user (None = follow the system default).
    preferred: Option<String>,
    /// Device actually open (None = system default).
    active: Option<String>,
    last_default: Option<String>,
}

impl Output {
    fn handle(&self) -> Option<&OutputStreamHandle> {
        self.stream.as_ref().map(|(_, handle)| handle)
    }

    /// (Re)open the preferred device, falling back to the system default.
    /// Errors are only reported when `report` (not on silent periodic retries).
    fn open(&mut self, event_tx: &Sender<AudioEvent>, report: bool) -> bool {
        // Release the old device first: some backends only allow one stream per device.
        self.stream = None;
        let error = |message: String| {
            if report {
                let _ = event_tx.send(AudioEvent::Error { message });
            }
        };
        if self.preferred.is_some() {
            match create_output_stream(&self.preferred) {
                Ok(stream) => {
                    self.stream = Some(stream);
                    self.active = self.preferred.clone();
                    self.last_default = get_default_device_name();
                    return true;
                }
                Err(e) => error(format!("{}; using the system default output", e)),
            }
        }
        match create_output_stream(&None) {
            Ok(stream) => {
                self.stream = Some(stream);
                self.active = None;
                self.last_default = get_default_device_name();
                true
            }
            Err(e) => {
                error(format!("Failed to initialize audio output: {}", e));
                false
            }
        }
    }

    /// Periodic check: nothing open, the open device vanished, the preferred
    /// device came back, or the followed system default changed.
    fn needs_reopen(&mut self) -> bool {
        if self.stream.is_none() {
            return true;
        }
        if self.preferred.is_some() {
            let devices = list_audio_devices();
            let present = |d: &Option<String>| d.as_ref().is_some_and(|d| devices.contains(d));
            let active_gone = self.active.is_some() && !present(&self.active);
            let preferred_back = self.active != self.preferred && present(&self.preferred);
            if active_gone || preferred_back {
                return true;
            }
        }
        if self.active.is_none() {
            let default = get_default_device_name();
            if default != self.last_default {
                self.last_default = default;
                return true;
            }
        }
        false
    }
}

/// Move every track to a freshly (re)opened output. Tracks that cannot resume
/// (no output at all, or a failed restart) are reported as stopped.
fn reopen_output(
    output: &mut Output,
    tracks: &mut HashMap<TrackId, AudioTrack>,
    sound_volumes: &mut HashMap<SoundId, f32>,
    track_sounds: &mut HashMap<TrackId, SoundId>,
    master_volume: f32,
    event_tx: &Sender<AudioEvent>,
    report: bool,
) {
    let resume_list = capture_and_stop_tracks(tracks, sound_volumes, event_tx);
    track_sounds.clear();
    if output.open(event_tx, report) {
        if let Some(handle) = output.handle() {
            resume_tracks(
                resume_list,
                tracks,
                sound_volumes,
                track_sounds,
                handle,
                master_volume,
                event_tx,
            );
        }
    } else {
        for info in resume_list {
            let _ = event_tx.send(AudioEvent::SoundEnded {
                track_id: info.track_id,
                sound_id: info.sound_id,
                reason: SoundEndedReason::Stopped,
            });
        }
    }
}

const NO_OUTPUT: &str = "No audio output device available";

/// The main loop of the audio thread.
fn audio_thread_main(
    command_rx: Receiver<AudioCommand>,
    event_tx: Sender<AudioEvent>,
    master_volume: Arc<AtomicU32>,
    initial_device: Option<String>,
) {
    // Initialize audio output. A missing saved device falls back to the default;
    // with no output at all the thread stays alive and retries periodically.
    let mut output = Output {
        stream: None,
        preferred: initial_device,
        active: None,
        last_default: None,
    };
    output.open(&event_tx, true);

    let mut tracks: HashMap<TrackId, AudioTrack> = HashMap::new();
    let mut sound_volumes: HashMap<SoundId, f32> = HashMap::new();
    let mut track_sounds: HashMap<TrackId, SoundId> = HashMap::new();
    let mut track_volumes: HashMap<TrackId, f32> = HashMap::new();
    let mut error_sound_path: Option<String> = None;

    // Device management state
    let mut last_device_check = Instant::now();
    let device_check_interval = Duration::from_secs(5);

    // Progress emission timer
    let mut last_progress_emit = Instant::now();
    let progress_interval = Duration::from_millis(250);

    loop {
        // Use shorter timeout when actively playing (crossfade needs ~60fps),
        // longer timeout when idle to reduce CPU usage
        let has_active_playback = tracks.values().any(|t| t.is_playing() || t.is_animating());
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
                let mv = atomic_load_f32(&master_volume);
                let Some(stream_handle) = output.handle() else {
                    let _ = event_tx.send(AudioEvent::Error {
                        message: NO_OUTPUT.to_string(),
                    });
                    continue;
                };

                // Create track if it doesn't exist
                if !tracks.contains_key(&track_id) {
                    if tracks.keys().filter(|id| !is_preview_track(id)).count() >= 20
                        && !is_preview_track(&track_id)
                    {
                        let _ = event_tx.send(AudioEvent::Error {
                            message: "Maximum number of tracks (20) reached".to_string(),
                        });
                        continue;
                    }
                    let mut track = AudioTrack::new(stream_handle.clone());
                    track.volume = track_volumes.get(&track_id).copied().unwrap_or(1.0);
                    tracks.insert(track_id.clone(), track);
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
                            let _ = event_tx.send(AudioEvent::SoundStarted {
                                track_id: track_id.clone(),
                                sound_id,
                            });
                        }
                        Err(e) => {
                            let _ = event_tx.send(AudioEvent::Error { message: e });
                        }
                    }
                }
            }
            Ok(AudioCommand::PlaySoundPrepared {
                reply,
                track_id,
                sound_id,
                file_path,
                start_position,
                source,
                sound_volume,
                crossfade_duration_ms,
            }) => {
                let mv = atomic_load_f32(&master_volume);
                let Some(stream_handle) = output.handle() else {
                    let _ = reply.send(Err(NO_OUTPUT.to_string()));
                    continue;
                };

                if !tracks.contains_key(&track_id) {
                    if tracks.keys().filter(|id| !is_preview_track(id)).count() >= 20
                        && !is_preview_track(&track_id)
                    {
                        let _ = event_tx.send(AudioEvent::Error {
                            message: "Maximum number of tracks (20) reached".to_string(),
                        });
                        let _ = reply.send(Err("Maximum number of tracks (20) reached".into()));
                        continue;
                    }
                    let mut track = AudioTrack::new(stream_handle.clone());
                    track.volume = track_volumes.get(&track_id).copied().unwrap_or(1.0);
                    tracks.insert(track_id.clone(), track);
                }

                if let Some(track) = tracks.get_mut(&track_id) {
                    sound_volumes.insert(sound_id.clone(), sound_volume);
                    track_sounds.insert(track_id.clone(), sound_id.clone());

                    match track.play_prepared(
                        sound_id.clone(),
                        &file_path,
                        start_position,
                        source,
                        sound_volume,
                        mv,
                        crossfade_duration_ms,
                    ) {
                        Ok(()) => {
                            let _ = reply.send(Ok(()));
                            let _ = event_tx.send(AudioEvent::SoundStarted {
                                track_id: track_id.clone(),
                                sound_id,
                            });
                        }
                        Err(e) => {
                            let _ = reply.send(Err(e));
                        }
                    }
                }
            }
            Ok(AudioCommand::StopTrack { track_id }) => {
                if let Some(mut track) = tracks.remove(&track_id) {
                    let sound_id = track.currently_playing.clone();
                    track.stop();
                    if let Some(sid) = sound_id {
                        let _ = event_tx.send(AudioEvent::SoundEnded {
                            track_id: track_id.clone(),
                            sound_id: sid,
                            reason: SoundEndedReason::Stopped,
                        });
                    }
                }
            }
            Ok(AudioCommand::StopAll) => {
                for (tid, track) in tracks.iter_mut() {
                    let sound_id = track.currently_playing.clone();
                    track.stop();
                    if let Some(sid) = sound_id {
                        let _ = event_tx.send(AudioEvent::SoundEnded {
                            track_id: tid.clone(),
                            sound_id: sid,
                            reason: SoundEndedReason::Stopped,
                        });
                    }
                }
                tracks.clear();
                track_sounds.clear();
                sound_volumes.clear();
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
                track_volumes.insert(track_id.clone(), volume);
                if let Some(track) = tracks.get_mut(&track_id) {
                    let mv = atomic_load_f32(&master_volume);
                    let sv = track_sounds
                        .get(&track_id)
                        .and_then(|sid| sound_volumes.get(sid))
                        .copied()
                        .unwrap_or(1.0);
                    track.set_volume(volume, sv, mv);
                }
            }
            Ok(AudioCommand::SetSoundVolume {
                track_id,
                sound_id,
                volume,
            }) => {
                // Update stored sound volume
                sound_volumes.insert(sound_id.clone(), volume);
                // Recalculate final volume on the track's sink
                if let Some(track) = tracks.get_mut(&track_id) {
                    let mv = atomic_load_f32(&master_volume);
                    if track.currently_playing.as_ref() == Some(&sound_id)
                        && track.crossfade.is_none()
                    {
                        track.update_master_volume(volume, mv);
                    }
                }
            }
            Ok(AudioCommand::SetAudioDevice { device_name }) => {
                // Rebuild the output stream (falls back to the default device)
                // and resume on whichever stream opened.
                output.preferred = device_name;
                reopen_output(
                    &mut output,
                    &mut tracks,
                    &mut sound_volumes,
                    &mut track_sounds,
                    atomic_load_f32(&master_volume),
                    &event_tx,
                    true,
                );
                last_device_check = Instant::now();
            }
            Ok(AudioCommand::SetErrorSoundPath { path }) => {
                error_sound_path = Some(path);
            }
            Ok(AudioCommand::PlayErrorSound) => {
                if let Some(ref path) = error_sound_path {
                    use crate::audio::symphonia_source::SymphoniaSource;
                    match SymphoniaSource::new(path, 0.0) {
                        Ok(source) => {
                            if let Some(Ok(sink)) = output.handle().map(rodio::Sink::try_new) {
                                let mv = atomic_load_f32(&master_volume);
                                sink.set_volume(mv * 0.5);
                                sink.append(source);
                                sink.detach(); // fire-and-forget
                            }
                        }
                        Err(e) => {
                            let _ = event_tx.send(AudioEvent::Error {
                                message: format!("Failed to play error sound: {}", e),
                            });
                        }
                    }
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

        // Poll for device changes: default device switch, selected device unplugged
        // or plugged back, or no output at all (retry).
        if last_device_check.elapsed() >= device_check_interval {
            last_device_check = Instant::now();
            if output.needs_reopen() {
                // Silent retries while nothing is open; report losing a working output.
                let report = output.stream.is_some();
                reopen_output(
                    &mut output,
                    &mut tracks,
                    &mut sound_volumes,
                    &mut track_sounds,
                    atomic_load_f32(&master_volume),
                    &event_tx,
                    report,
                );
            }
        }

        // Update crossfades and volume ramps
        let mv = atomic_load_f32(&master_volume);
        for (tid, track) in tracks.iter_mut() {
            track.tick_fading();
            if track.crossfade.is_some() {
                let sv = track_sounds
                    .get(tid)
                    .and_then(|sid| sound_volumes.get(sid))
                    .copied()
                    .unwrap_or(1.0);
                track.update_crossfade(sv, mv);
            } else {
                track.tick_volume_ramp();
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
            let _ = event_tx.send(AudioEvent::SoundEnded {
                track_id: tid,
                sound_id: sid,
                reason: SoundEndedReason::Finished,
            });
        }

        // Emit progress events
        if last_progress_emit.elapsed() >= progress_interval {
            last_progress_emit = Instant::now();
            for (tid, track) in tracks.iter() {
                if track.is_playing() {
                    let _ = event_tx.send(AudioEvent::PlaybackProgress {
                        track_id: tid.clone(),
                        position: track.get_position(),
                    });
                }
            }
        }
    }
}

/// Capture resume info from all playing tracks, then stop and clear them.
/// Sounds that just finished are reported as such instead of being lost.
fn capture_and_stop_tracks(
    tracks: &mut HashMap<TrackId, AudioTrack>,
    sound_volumes: &HashMap<SoundId, f32>,
    event_tx: &Sender<AudioEvent>,
) -> Vec<TrackResumeInfo> {
    let mut resume_list = Vec::new();
    for (tid, track) in tracks.iter() {
        if let (Some(sid), Some(fp)) = (&track.currently_playing, &track.file_path) {
            if !track.is_playing() {
                let _ = event_tx.send(AudioEvent::SoundEnded {
                    track_id: tid.clone(),
                    sound_id: sid.clone(),
                    reason: SoundEndedReason::Finished,
                });
            } else {
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
    for (_, track) in tracks.iter_mut() {
        track.stop();
    }
    tracks.clear();
    resume_list
}

/// Resume previously playing tracks on a new stream handle.
fn resume_tracks(
    resume_list: Vec<TrackResumeInfo>,
    tracks: &mut HashMap<TrackId, AudioTrack>,
    sound_volumes: &mut HashMap<SoundId, f32>,
    track_sounds: &mut HashMap<TrackId, SoundId>,
    stream_handle: &OutputStreamHandle,
    master_volume: f32,
    event_tx: &Sender<AudioEvent>,
) {
    for info in resume_list {
        let mut new_track = AudioTrack::new(stream_handle.clone());
        new_track.volume = info.track_volume;
        match new_track.play(
            info.sound_id.clone(),
            &info.file_path,
            info.position,
            info.sound_volume,
            master_volume,
            0, // no crossfade for resume
        ) {
            Ok(()) => {}
            Err(e) => {
                let _ = event_tx.send(AudioEvent::Error {
                    message: format!("Failed to resume track {}: {}", info.track_id, e),
                });
                let _ = event_tx.send(AudioEvent::SoundEnded {
                    track_id: info.track_id,
                    sound_id: info.sound_id,
                    reason: SoundEndedReason::Stopped,
                });
                continue;
            }
        }
        tracks.insert(info.track_id.clone(), new_track);
        sound_volumes.insert(info.sound_id.clone(), info.sound_volume);
        track_sounds.insert(info.track_id, info.sound_id);
    }
}

#[cfg(test)]
mod request_tests {
    use super::PlaybackRequests;

    #[test]
    fn stop_invalidates_decoding_even_if_track_was_not_playing() {
        let mut requests = PlaybackRequests::default();
        let request = requests.begin("a");
        requests.tracks.clear();
        assert!(!requests.is_current("a", request));
        let next = requests.begin("a");
        assert_ne!(request, next);
        assert!(!requests.is_current("a", request));
        assert!(requests.is_current("a", next));
    }

    #[test]
    fn newer_playback_only_supersedes_the_same_track() {
        let mut requests = PlaybackRequests::default();
        let a = requests.begin("a");
        let b = requests.begin("b");
        requests.begin("a");
        assert!(!requests.is_current("a", a));
        assert!(requests.is_current("b", b));
    }
}
