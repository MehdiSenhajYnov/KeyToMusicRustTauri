import type { MoodCategory } from "../types";

export const MOOD_CATEGORIES: MoodCategory[] = [
  "epic_battle",
  "tension",
  "sadness",
  "comedy",
  "romance",
  "horror",
  "peaceful",
  "emotional_climax",
  "mystery",
  "chase_action",
];

export const MOOD_DISPLAY: Record<MoodCategory, string> = {
  epic_battle: "Epic Battle",
  tension: "Tension",
  sadness: "Sadness",
  comedy: "Comedy",
  romance: "Romance",
  horror: "Horror",
  peaceful: "Peaceful",
  emotional_climax: "Emotional Climax",
  mystery: "Mystery",
  chase_action: "Chase/Action",
};

export const MOOD_COLORS: Record<MoodCategory, { bg: string; text: string }> = {
  epic_battle: { bg: "bg-red-500/20", text: "text-red-400" },
  tension: { bg: "bg-amber-500/20", text: "text-amber-400" },
  sadness: { bg: "bg-blue-500/20", text: "text-blue-400" },
  comedy: { bg: "bg-yellow-500/20", text: "text-yellow-400" },
  romance: { bg: "bg-pink-500/20", text: "text-pink-400" },
  horror: { bg: "bg-purple-500/20", text: "text-purple-400" },
  peaceful: { bg: "bg-green-500/20", text: "text-green-400" },
  emotional_climax: { bg: "bg-orange-500/20", text: "text-orange-400" },
  mystery: { bg: "bg-indigo-500/20", text: "text-indigo-400" },
  chase_action: { bg: "bg-cyan-500/20", text: "text-cyan-400" },
};
