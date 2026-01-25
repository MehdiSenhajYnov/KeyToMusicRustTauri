import { invoke } from "@tauri-apps/api/core";
import type { AppConfig, Profile, Sound } from "../types";

interface ProfileSummary {
  id: string;
  name: string;
  created_at: string;
  updated_at: string;
  sound_count: number;
  track_count: number;
}

// ─── Config ────────────────────────────────────────────────────────────────

export async function getConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("get_config");
}

export async function updateConfig(
  updates: Partial<AppConfig>
): Promise<void> {
  return invoke("update_config", { updates });
}

// ─── Profiles ──────────────────────────────────────────────────────────────

export async function listProfiles(): Promise<ProfileSummary[]> {
  return invoke<ProfileSummary[]>("list_profiles");
}

export async function createProfile(name: string): Promise<Profile> {
  return invoke<Profile>("create_profile", { name });
}

export async function loadProfile(id: string): Promise<Profile> {
  return invoke<Profile>("load_profile", { id });
}

export async function saveProfile(profile: Profile): Promise<void> {
  return invoke("save_profile", { profile });
}

export async function deleteProfile(id: string): Promise<void> {
  return invoke("delete_profile", { id });
}

export async function duplicateProfile(
  id: string,
  newName?: string
): Promise<Profile> {
  return invoke<Profile>("duplicate_profile", { id, newName });
}

// ─── Audio ─────────────────────────────────────────────────────────────────

export async function playSound(
  trackId: string,
  soundId: string,
  filePath: string,
  startPosition: number,
  soundVolume: number
): Promise<void> {
  return invoke("play_sound", {
    trackId,
    soundId,
    filePath,
    startPosition,
    soundVolume,
  });
}

export async function stopSound(trackId: string): Promise<void> {
  return invoke("stop_sound", { trackId });
}

export async function stopAllSounds(): Promise<void> {
  return invoke("stop_all_sounds");
}

export async function setMasterVolume(volume: number): Promise<void> {
  return invoke("set_master_volume", { volume });
}

export async function setTrackVolume(
  trackId: string,
  volume: number
): Promise<void> {
  return invoke("set_track_volume", { trackId, volume });
}

export async function setSoundVolume(
  trackId: string,
  soundId: string,
  volume: number
): Promise<void> {
  return invoke("set_sound_volume", { trackId, soundId, volume });
}

export async function getAudioDuration(path: string): Promise<number> {
  return invoke<number>("get_audio_duration", { path });
}

export interface SoundPreloadEntry {
  soundId: string;
  filePath: string;
  needsDuration: boolean;
}

export async function preloadProfileSounds(
  sounds: SoundPreloadEntry[]
): Promise<Record<string, number>> {
  return invoke<Record<string, number>>("preload_profile_sounds", { sounds });
}

// ─── Keys ──────────────────────────────────────────────────────────────────

export async function setKeyDetection(enabled: boolean): Promise<void> {
  return invoke("set_key_detection", { enabled });
}

export async function setMasterStopShortcut(
  keys: string[]
): Promise<void> {
  return invoke("set_master_stop_shortcut", { keys });
}

export async function setKeyCooldown(cooldownMs: number): Promise<void> {
  return invoke("set_key_cooldown", { cooldownMs });
}

export async function setProfileBindings(bindings: string[]): Promise<void> {
  return invoke("set_profile_bindings", { bindings });
}

// ─── YouTube ──────────────────────────────────────────────────────────────

export async function addSoundFromYoutube(url: string, downloadId: string): Promise<Sound> {
  return invoke<Sound>("add_sound_from_youtube", { url, downloadId });
}

export async function checkYtDlpInstalled(): Promise<boolean> {
  return invoke<boolean>("check_yt_dlp_installed");
}

export async function installYtDlp(): Promise<void> {
  return invoke("install_yt_dlp");
}

export async function checkFfmpegInstalled(): Promise<boolean> {
  return invoke<boolean>("check_ffmpeg_installed");
}

export async function installFfmpeg(): Promise<void> {
  return invoke("install_ffmpeg");
}

// ─── Audio Devices ───────────────────────────────────────────────────────

export async function listAudioDevices(): Promise<string[]> {
  return invoke<string[]>("list_audio_devices");
}

export async function setAudioDevice(
  device: string | null
): Promise<void> {
  return invoke("set_audio_device", { device });
}

// ─── Import/Export ────────────────────────────────────────────────────────

export async function exportProfile(
  profileId: string,
  outputPath: string
): Promise<void> {
  return invoke("export_profile", { profileId, outputPath });
}

export async function importProfile(ktmPath: string): Promise<string> {
  return invoke<string>("import_profile", { ktmPath });
}

export async function pickSaveLocation(
  defaultName: string
): Promise<string | null> {
  return invoke<string | null>("pick_save_location", { defaultName });
}

export async function pickKtmFile(): Promise<string | null> {
  return invoke<string | null>("pick_ktm_file");
}

export async function cleanupExportTemp(): Promise<void> {
  return invoke("cleanup_export_temp");
}

export async function cancelExport(): Promise<void> {
  return invoke("cancel_export");
}

// ─── Legacy Import ────────────────────────────────────────────────────────

export async function pickLegacyFile(): Promise<string | null> {
  return invoke<string | null>("pick_legacy_file");
}

export async function importLegacySave(path: string): Promise<Profile> {
  return invoke<Profile>("import_legacy_save", { path });
}

// ─── Error Handling ───────────────────────────────────────────────────────

export interface MissingSoundInfo {
  soundId: string;
  soundName: string;
  filePath: string;
  sourceType: string;
}

export async function verifyProfileSounds(
  profile: Profile
): Promise<MissingSoundInfo[]> {
  return invoke<MissingSoundInfo[]>("verify_profile_sounds", { profile });
}

export async function pickAudioFile(): Promise<string | null> {
  return invoke<string | null>("pick_audio_file");
}

export async function pickAudioFiles(): Promise<string[]> {
  return invoke<string[]>("pick_audio_files");
}

export async function getLogsFolder(): Promise<string> {
  return invoke<string>("get_logs_folder");
}

export async function getDataFolder(): Promise<string> {
  return invoke<string>("get_data_folder");
}

export async function openFolder(path: string): Promise<void> {
  return invoke<void>("open_folder", { path });
}

export { type ProfileSummary };
