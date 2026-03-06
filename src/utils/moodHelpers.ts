import type { BaseMood, MoodIntensity } from "../types";

export const BASE_MOODS: BaseMood[] = [
  "epic",
  "tension",
  "sadness",
  "comedy",
  "romance",
  "horror",
  "peaceful",
  "mystery",
];

// Legacy alias
export const MOOD_CATEGORIES = BASE_MOODS;

export const MOOD_DISPLAY: Record<BaseMood, string> = {
  epic: "Epic",
  tension: "Tension",
  sadness: "Sadness",
  comedy: "Comedy",
  romance: "Romance",
  horror: "Horror",
  peaceful: "Peaceful",
  mystery: "Mystery",
};

export const MOOD_COLORS: Record<BaseMood, { bg: string; text: string }> = {
  epic: { bg: "bg-red-500/20", text: "text-red-400" },
  tension: { bg: "bg-amber-500/20", text: "text-amber-400" },
  sadness: { bg: "bg-blue-500/20", text: "text-blue-400" },
  comedy: { bg: "bg-yellow-500/20", text: "text-yellow-400" },
  romance: { bg: "bg-pink-500/20", text: "text-pink-400" },
  horror: { bg: "bg-purple-500/20", text: "text-purple-400" },
  peaceful: { bg: "bg-green-500/20", text: "text-green-400" },
  mystery: { bg: "bg-indigo-500/20", text: "text-indigo-400" },
};

export const INTENSITY_DISPLAY: Record<MoodIntensity, string> = {
  1: "Calm",
  2: "Moderate",
  3: "Intense",
};

export const INTENSITY_COLORS: Record<MoodIntensity, string> = {
  1: "opacity-50",
  2: "",
  3: "ring-1 ring-current font-bold",
};
