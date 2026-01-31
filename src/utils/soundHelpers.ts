import type { Sound } from "../types";

export function getSoundFilePath(sound: Sound): string {
  if (sound.source.type === "local") return sound.source.path;
  return sound.source.cachedPath;
}
