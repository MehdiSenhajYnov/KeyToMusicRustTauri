import type { TrackId } from "../types";

const pendingTracks = new Set<TrackId>();

/** Keep manual, mood and loop triggers from decoding competing sounds on one track. */
export async function withTrackPlayback<T>(
  trackId: TrackId,
  play: () => Promise<T>
): Promise<T | undefined> {
  if (pendingTracks.has(trackId)) return undefined;
  pendingTracks.add(trackId);
  try {
    return await play();
  } finally {
    pendingTracks.delete(trackId);
  }
}
