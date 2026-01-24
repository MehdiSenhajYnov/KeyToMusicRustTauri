const ERROR_PATTERNS: [RegExp, string][] = [
  [/Sound file not found: (.+)/, "Sound file not found: $1"],
  [/Failed to open default audio device/, "No audio output device available"],
  [/Failed to switch audio device: (.+)/, "Could not switch audio device: $1"],
  [/Failed to initialize audio output/, "Audio system initialization failed"],
  [/Maximum number of tracks \(20\) reached/, "Maximum track limit (20) reached"],
  [/Failed to resume track/, "Failed to resume playback after device switch"],
  [/Unsupported audio format/, "Unsupported audio format"],
  [/YouTube download failed/, "YouTube download failed"],
  [/yt-dlp not found/, "yt-dlp is not installed"],
];

export function formatErrorMessage(rawError: string): string {
  for (const [pattern, template] of ERROR_PATTERNS) {
    const match = rawError.match(pattern);
    if (match) {
      return template.replace(/\$(\d+)/g, (_, idx) => match[Number(idx)] || "");
    }
  }
  return rawError;
}
