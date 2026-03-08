import { invoke } from "@tauri-apps/api/core";
import type {
  AppConfig,
  BaseMood,
  InitialState,
  LinuxInputAccessFixResult,
  LinuxInputAccessStatus,
  Profile,
  Sound,
  StreamUrlResult,
  WaveformData,
  YoutubePlaylist,
  YoutubeSearchResult,
} from "../types";

interface ProfileSummary {
  id: string;
  name: string;
  createdAt: string;
  updatedAt: string;
}

// ─── Startup ──────────────────────────────────────────────────────────────

export async function getInitialState(): Promise<InitialState> {
  return invoke<InitialState>("get_initial_state");
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

export async function setStopAllShortcut(
  keys: string[]
): Promise<void> {
  return invoke("set_stop_all_shortcut", { keys });
}

export async function setKeyCooldown(cooldownMs: number): Promise<void> {
  return invoke("set_key_cooldown", { cooldownMs });
}

export async function setProfileBindings(bindings: string[]): Promise<void> {
  return invoke("set_profile_bindings", { bindings });
}

export async function getLinuxInputAccessStatus(): Promise<LinuxInputAccessStatus> {
  return invoke<LinuxInputAccessStatus>("get_linux_input_access_status");
}

export async function enableLinuxBackgroundDetection(): Promise<LinuxInputAccessFixResult> {
  return invoke<LinuxInputAccessFixResult>("enable_linux_background_detection");
}

// ─── YouTube ──────────────────────────────────────────────────────────────

export async function addSoundFromYoutube(url: string, downloadId: string): Promise<Sound> {
  return invoke<Sound>("add_sound_from_youtube", { url, downloadId });
}

export async function searchYoutube(query: string, maxResults: number): Promise<YoutubeSearchResult[]> {
  return invoke<YoutubeSearchResult[]>("search_youtube", { query, maxResults });
}

export async function fetchPlaylist(url: string): Promise<YoutubePlaylist> {
  return invoke<YoutubePlaylist>("fetch_playlist", { url });
}

export async function getYoutubeStreamUrl(videoId: string): Promise<StreamUrlResult> {
  return invoke<StreamUrlResult>("get_youtube_stream_url", { videoId });
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

// ─── Waveform ────────────────────────────────────────────────────────────

export async function getWaveform(path: string, numPoints: number): Promise<WaveformData> {
  return invoke<WaveformData>("get_waveform", { path, numPoints });
}

export interface WaveformBatchEntry {
  path: string;
  numPoints: number;
}

export async function getWaveformsBatch(entries: WaveformBatchEntry[]): Promise<Record<string, WaveformData>> {
  return invoke<Record<string, WaveformData>>("get_waveforms_batch", { entries });
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

// ─── Discovery ───────────────────────────────────────────────────────────

import type { DiscoverySuggestion } from "../stores/discoveryStore";

export async function startDiscovery(
  profileId: string,
  excludeIds: string[] = [],
  background: boolean = false
): Promise<DiscoverySuggestion[]> {
  return invoke<DiscoverySuggestion[]>("start_discovery", { profileId, excludeIds, background });
}

export interface DiscoveryCacheResponse {
  suggestions: DiscoverySuggestion[];
  cursorIndex: number;
  revealedCount: number;
  visitedIndex: number;
}

export async function getDiscoverySuggestions(profileId: string): Promise<DiscoveryCacheResponse | null> {
  return invoke<DiscoveryCacheResponse | null>("get_discovery_suggestions", { profileId });
}

export async function saveDiscoveryCursor(
  profileId: string,
  cursorIndex: number,
  revealedCount: number,
  visitedIndex: number
): Promise<void> {
  return invoke("save_discovery_cursor", { profileId, cursorIndex, revealedCount, visitedIndex });
}

export async function updateDiscoveryPool(
  profileId: string,
  suggestions: DiscoverySuggestion[],
  cursorIndex: number,
  revealedCount: number,
  visitedIndex: number
): Promise<void> {
  return invoke("update_discovery_pool", { profileId, suggestions, cursorIndex, revealedCount, visitedIndex });
}

export async function dismissDiscovery(profileId: string, videoId: string): Promise<void> {
  return invoke("dismiss_discovery", { profileId, videoId });
}

export async function dislikeDiscovery(profileId: string, videoId: string): Promise<void> {
  return invoke("dislike_discovery", { profileId, videoId });
}

export async function undislikeDiscovery(profileId: string, videoId: string): Promise<void> {
  return invoke("undislike_discovery", { profileId, videoId });
}

export interface DislikedVideoInfo {
  videoId: string;
  title: string;
  channel: string;
  duration: number;
  url: string;
}

export async function listDislikedVideos(profileId: string): Promise<DislikedVideoInfo[]> {
  return invoke("list_disliked_videos", { profileId });
}

export async function cancelDiscovery(): Promise<void> {
  return invoke("cancel_discovery");
}

export interface PredownloadResult {
  videoId: string;
  cachedPath: string;
  title: string;
  duration: number;
  waveform: WaveformData | null;
}

export async function predownloadSuggestion(
  url: string,
  videoId: string,
  downloadId: string
): Promise<PredownloadResult> {
  return invoke<PredownloadResult>("predownload_suggestion", {
    url,
    videoId,
    downloadId,
  });
}

// ─── Mood AI ─────────────────────────────────────────────────────────────

export async function checkLlamaServerInstalled(): Promise<boolean> {
  return invoke<boolean>("check_llama_server_installed");
}

export async function installLlamaServer(): Promise<string> {
  return invoke<string>("install_llama_server");
}

export async function checkMoodModelInstalled(): Promise<boolean> {
  return invoke<boolean>("check_mood_model_installed");
}

export async function installMoodModel(): Promise<string> {
  return invoke<string>("install_mood_model");
}

export async function startMoodServer(): Promise<void> {
  return invoke("start_mood_server");
}

export async function stopMoodServer(): Promise<void> {
  return invoke("stop_mood_server");
}

export async function getMoodServerStatus(): Promise<string> {
  return invoke<string>("get_mood_server_status");
}

export interface MoodServiceStatus {
  runtime: "stopped" | "starting" | "running" | "error";
  api: "disabled" | "stopped" | "running" | "error";
  enabled: boolean;
  port: number;
}

export async function getMoodServiceStatus(): Promise<MoodServiceStatus> {
  return invoke<MoodServiceStatus>("get_mood_service_status");
}

export async function analyzeMood(imagePath: string): Promise<BaseMood> {
  return invoke<BaseMood>("analyze_mood", { imagePath });
}

export { type ProfileSummary };
