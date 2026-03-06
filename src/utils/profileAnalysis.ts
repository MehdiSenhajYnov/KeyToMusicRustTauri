import type { Profile, Sound } from "../types";
import type { DiscoverySuggestion } from "../stores/discoveryStore";
import { findNextAvailableKey } from "./keyMapping";
import { findLeastUsedTrack } from "./soundHelpers";

export type ProfileMode = "single-sound" | "multi-sound";

export interface AutoAssign {
  suggestedKey: string;
  suggestedTrackId: string;
}

/**
 * Analyze profile usage pattern.
 * "multi-sound" if average sounds per binding > 2, else "single-sound".
 */
export function analyzeProfile(profile: Profile): ProfileMode {
  const { keyBindings } = profile;
  if (keyBindings.length === 0) return "single-sound";

  const totalSounds = keyBindings.reduce(
    (sum, kb) => sum + kb.soundIds.length,
    0
  );
  const avg = totalSounds / keyBindings.length;
  return avg > 2 ? "multi-sound" : "single-sound";
}

/**
 * Extract the video ID from a YouTube URL.
 * Handles youtube.com/watch?v=ID, youtu.be/ID, and similar patterns.
 */
function extractVideoId(url: string): string | null {
  // youtube.com/watch?v=ID
  const longMatch = url.match(/[?&]v=([^&#]+)/);
  if (longMatch) return longMatch[1];

  // youtu.be/ID
  const shortMatch = url.match(/youtu\.be\/([^?&#]+)/);
  if (shortMatch) return shortMatch[1];

  return null;
}

/**
 * Build a map from soundId -> videoId for all YouTube sounds in the profile.
 */
function buildSoundVideoIdMap(sounds: Sound[]): Map<string, string> {
  const map = new Map<string, string>();
  for (const sound of sounds) {
    if (sound.source.type === "youtube") {
      const vid = extractVideoId(sound.source.url);
      if (vid) map.set(sound.id, vid);
    }
  }
  return map;
}

/**
 * Compute auto-assign for a discovery suggestion.
 *
 * In single-sound mode: next available key from AUTO_KEY_ORDER.
 * In multi-sound mode: find the existing binding whose sounds share the most
 * seed video IDs with the suggestion, or fallback to the binding with the
 * most sounds (the user's accumulation pattern).
 */
export function computeAutoAssign(
  suggestion: DiscoverySuggestion,
  profile: Profile,
  usedKeys: Set<string>,
  alreadySuggested: Set<string>
): AutoAssign {
  const mode = analyzeProfile(profile);

  if (mode === "single-sound" || profile.keyBindings.length === 0) {
    return singleSoundAssign(profile, usedKeys, alreadySuggested);
  }

  return multiSoundAssign(suggestion, profile);
}

function singleSoundAssign(
  profile: Profile,
  usedKeys: Set<string>,
  alreadySuggested: Set<string>
): AutoAssign {
  const key = findNextAvailableKey(usedKeys, alreadySuggested);
  const trackId = findLeastUsedTrack(profile.tracks, profile.keyBindings);
  return { suggestedKey: key, suggestedTrackId: trackId };
}

function multiSoundAssign(
  suggestion: DiscoverySuggestion,
  profile: Profile
): AutoAssign {
  const seedIds = new Set(suggestion.sourceSeedIds ?? []);
  const soundVideoIdMap = buildSoundVideoIdMap(profile.sounds);

  let bestKey = "";
  let bestTrackId = "";
  let bestMatchCount = 0;
  let bestSoundCount = 0;

  for (const kb of profile.keyBindings) {
    // Count how many sounds in this binding have a video_id in the seed IDs
    let matchCount = 0;
    for (const soundId of kb.soundIds) {
      const vid = soundVideoIdMap.get(soundId);
      if (vid && seedIds.has(vid)) {
        matchCount++;
      }
    }

    const soundCount = kb.soundIds.length;

    // Prefer binding with highest match count, break ties by sound count
    if (
      matchCount > bestMatchCount ||
      (matchCount === bestMatchCount && soundCount > bestSoundCount)
    ) {
      bestMatchCount = matchCount;
      bestSoundCount = soundCount;
      bestKey = kb.keyCode;
      bestTrackId = kb.trackId;
    }
  }

  // If no seed match at all, fallback to binding with most sounds
  if (bestMatchCount === 0) {
    let maxSounds = 0;
    for (const kb of profile.keyBindings) {
      if (kb.soundIds.length > maxSounds) {
        maxSounds = kb.soundIds.length;
        bestKey = kb.keyCode;
        bestTrackId = kb.trackId;
      }
    }
  }

  return { suggestedKey: bestKey, suggestedTrackId: bestTrackId };
}
