"""
Manga Mood AI — Optimization Benchmark for Qwen3-VL 2B.

Tests the champion model with different optimization combos:
  1. baseline     — Current config (short prompt, num_predict=5000, full-size images)
  2. resize_448   — Resize images to max 448px
  3. resize_672   — Resize images to max 672px
  4. structured   — Structured output (JSON schema with enum, no thinking)
  5. struct+448   — Structured output + 448px resize
  6. struct+672   — Structured output + 672px resize

Usage:
    python test_optimizations.py                  # Run all configs
    python test_optimizations.py --config baseline resize_448 structured
    python test_optimizations.py --runs 3         # Average over 3 runs for stable timing
"""

import argparse
import json
import os
import time
import threading
from pathlib import Path
from typing import Literal

import ollama
from PIL import Image
from pydantic import BaseModel, Field
from rich.console import Console
from rich.table import Table
from rich.panel import Panel
from rich import box

console = Console()

# ============================================================
# Ground truth
# ============================================================

GROUND_TRUTH = {
    "1.jpg":   "epic_battle",
    "10.jpg":  "emotional_climax",
    "11.jpg":  "emotional_climax",
    "12.jpg":  "emotional_climax",
    "13.png":  "sadness",
    "14.png": "emotional_climax",
    "15.png": "emotional_climax",
    "16.png": "sadness",
    "17.png": "sadness",
    "18.png": "peaceful",
    "2.jpeg":  "tension",
    "4.jpg":   "tension",
    "5.png":   "tension",
    "6.jpg":   "romance",
    "7.jpg":   "sadness",
    "8.jpg":   "horror",
    "3.jpeg":  "epic_battle",
    "9.png":   "horror",
}

# Acceptable alternatives (mood is subjective)
ACCEPTABLE_ALT = {
    "5.png":   ["emotional_climax"],  # "I don't give up" — tension or emotional_climax both valid
    "10.jpg":  ["tension"],           # Blue Lock stadium — could be tension too
    "3.jpeg":  ["tension"],           # Solo Leveling — epic battle in tense moment, both valid
    "13.png":  ["emotional_climax"],  # Robin crying — sadness is best but emotional_climax acceptable
    "14.png": ["sadness"],            # Coach post-defeat speech — emotional climax but sadness of defeat also valid
    "16.png": ["peaceful"],          # "Would my fate have been different?" — melancholy, could be peaceful retrospection
    "15.png": ["sadness"],            # "Best team in Japan" tears — emotional climax but sadness of defeat also valid
    "17.png": ["emotional_climax"],  # "I wanted to win" crying — sadness but also emotional climax of defeat
}

MOODS = [
    "epic_battle", "tension", "sadness", "comedy", "romance",
    "horror", "peaceful", "emotional_climax", "mystery", "chase_action",
]

# ============================================================
# Structured output schema (Pydantic)
# ============================================================

class MoodResult(BaseModel):
    mood: Literal[
        "epic_battle", "tension", "sadness", "comedy", "romance",
        "horror", "peaceful", "emotional_climax", "mystery", "chase_action",
    ]
    intensity: int = Field(ge=1, le=5, description="Mood intensity from 1 (subtle) to 5 (overwhelming)")

# Schema with a thinking field — lets the model reason inside JSON
class MoodResultThinking(BaseModel):
    thinking: str = Field(description="Your reasoning about the image mood")
    mood: Literal[
        "epic_battle", "tension", "sadness", "comedy", "romance",
        "horror", "peaceful", "emotional_climax", "mystery", "chase_action",
    ]
    intensity: int = Field(ge=1, le=5, description="Mood intensity from 1 (subtle) to 5 (overwhelming)")

MOOD_SCHEMA = MoodResult.model_json_schema()
MOOD_SCHEMA_THINKING = MoodResultThinking.model_json_schema()

# ============================================================
# Prompts
# ============================================================

# Baseline: same as champion config
PROMPT_SHORT = (
    "Analyze this manga page's emotional mood. Categories:\n"
    "- epic_battle: intense fighting, action clash, combat\n"
    "- tension: suspense, confrontation, anticipation, high stakes\n"
    "- sadness: grief, loss, defeat, crying, melancholy\n"
    "- comedy: humor, funny reactions, slapstick, chibi\n"
    "- romance: love, intimacy, tender moments\n"
    "- horror: fear, dark atmosphere, monsters, gore\n"
    "- peaceful: calm, daily life, relaxed conversation\n"
    "- emotional_climax: triumphant speech, breakthrough, peak emotion, resolve\n"
    "- mystery: enigmatic, questioning, investigation\n"
    "- chase_action: pursuit, running, escape, fast movement\n"
    "Reply with ONLY the category name."
)

# Enhanced prompt with visual cues to guide the model's thinking
PROMPT_VISUAL = (
    "Analyze this manga page's emotional mood.\n"
    "Focus on: facial expressions, body language, visual effects (speed lines, "
    "dark shading, screentones, sparkles), panel composition, and text intensity.\n\n"
    "Categories:\n"
    "- epic_battle: intense fighting, action clash, impact effects, combat poses\n"
    "- tension: suspense, confrontation, anticipation, high stakes standoff\n"
    "- sadness: grief, loss, defeat, crying, tears, melancholy, head down\n"
    "- comedy: humor, funny reactions, slapstick, chibi/super-deformed faces\n"
    "- romance: love, intimacy, tender moments, blushing\n"
    "- horror: fear, dark atmosphere, monsters, gore, distorted faces\n"
    "- peaceful: calm, daily life, relaxed conversation, normal expressions\n"
    "- emotional_climax: triumphant speech, breakthrough moment, intense resolve, "
    "motivational words, team rallying, peak emotion (NOT sadness if characters show determination)\n"
    "- mystery: enigmatic, questioning, investigation, hidden information\n"
    "- chase_action: pursuit, running, escape, fast movement, speed lines\n\n"
    "Reply with ONLY the category name."
)

# ── Visual V2: visual cues + key distinctions + narrative framing ──
PROMPT_VISUAL_V2 = (
    "You are analyzing a manga page. Think about what emotion the author wants "
    "the reader to feel.\n"
    "Focus on: facial expressions, body language, visual effects, panel composition.\n\n"
    "Key distinctions:\n"
    "- sadness vs emotional_climax: sorrow/regret/nostalgia = sadness, triumph/determination = emotional_climax\n"
    "- sadness can have bold text or movement (flashbacks, memories)\n"
    "- chase_action: ONLY active pursuit/escape, not flashbacks with movement\n\n"
    "Categories:\n"
    "- epic_battle: intense fighting, action clash, impact effects, combat poses\n"
    "- tension: suspense, confrontation, anticipation, high stakes standoff\n"
    "- sadness: grief, loss, defeat, crying, tears, melancholy, nostalgia, regret\n"
    "- comedy: humor, funny reactions, slapstick, chibi/super-deformed faces\n"
    "- romance: love, intimacy, tender moments, blushing\n"
    "- horror: fear, dark atmosphere, monsters, gore, distorted faces\n"
    "- peaceful: calm, daily life, relaxed conversation, normal expressions\n"
    "- emotional_climax: triumphant speech, breakthrough moment, intense resolve, "
    "motivational words, team rallying, peak determination\n"
    "- mystery: enigmatic, questioning, investigation, hidden information\n"
    "- chase_action: pursuit, running, escape, fast movement, speed lines\n\n"
    "Reply with ONLY the category name."
)

# ── Narrative intent: what should the READER feel? ──
PROMPT_NARRATIVE = (
    "You are analyzing a manga page. Think about what EMOTION the author wants "
    "the reader to feel — not just what is visually happening.\n\n"
    "Categories:\n"
    "- epic_battle: the reader should feel the thrill of combat\n"
    "- tension: the reader should feel anxious, on edge, uneasy\n"
    "- sadness: the reader should feel sorrow, nostalgia, regret, loss\n"
    "- comedy: the reader should laugh\n"
    "- romance: the reader should feel warmth, love, tenderness\n"
    "- horror: the reader should feel scared, disturbed\n"
    "- peaceful: the reader should feel relaxed, at ease\n"
    "- emotional_climax: the reader should feel inspired, triumphant, a peak moment of determination\n"
    "- mystery: the reader should feel curious, intrigued\n"
    "- chase_action: the reader should feel the urgency of a pursuit\n\n"
    "Reply with ONLY the category name."
)

# ── Guided reasoning: step-by-step + key distinctions ──
PROMPT_GUIDED = (
    "Analyze this manga page step by step:\n"
    "1. What are the characters expressing? (faces, posture, gestures)\n"
    "2. What feeling does the author want the reader to experience?\n"
    "3. Classify as ONE of the categories below.\n\n"
    "Key distinctions:\n"
    "- sadness vs emotional_climax: sorrow/regret/nostalgia = sadness, triumph/determination = emotional_climax\n"
    "- tension vs epic_battle: anxious anticipation = tension, active combat = epic_battle\n"
    "- chase_action: ONLY for active pursuit/escape, not flashbacks with movement\n\n"
    "Categories: epic_battle, tension, sadness, comedy, romance, horror, "
    "peaceful, emotional_climax, mystery, chase_action\n\n"
    "Reply with ONLY the category name after your reasoning."
)

# ── Guided V2: step-by-step + distinctions + category descriptions ──
PROMPT_GUIDED_V2 = (
    "Analyze this manga page step by step:\n"
    "1. What are the characters expressing?\n"
    "2. What emotion does the author want the reader to feel?\n"
    "3. Classify as ONE category below.\n\n"
    "Key: sadness = sorrow/nostalgia (even with movement), "
    "emotional_climax = triumph/determination, "
    "chase_action = ONLY active pursuit\n\n"
    "Categories:\n"
    "- epic_battle: intense fighting, combat clash\n"
    "- tension: suspense, anticipation, high stakes\n"
    "- sadness: grief, loss, melancholy, nostalgia, regret\n"
    "- comedy: humor, slapstick, chibi faces\n"
    "- romance: love, tender moments, blushing\n"
    "- horror: fear, dark dread, monsters, gore\n"
    "- peaceful: calm, daily life, relaxed conversation\n"
    "- emotional_climax: triumphant speech, breakthrough, peak determination\n"
    "- mystery: enigmatic, investigation, hidden information\n"
    "- chase_action: pursuit, running, escape\n\n"
    "Reply with ONLY the category name."
)

# ── Guided V3: original guided + minimal hint for peaceful only ──
PROMPT_GUIDED_V3 = (
    "Analyze this manga page step by step:\n"
    "1. What are the characters expressing? (faces, posture, gestures)\n"
    "2. What feeling does the author want the reader to experience?\n"
    "3. Classify as ONE of the categories below.\n\n"
    "Key distinctions:\n"
    "- sadness vs emotional_climax: sorrow/regret/nostalgia = sadness, triumph/determination = emotional_climax\n"
    "- tension vs epic_battle: anxious anticipation = tension, active combat = epic_battle\n"
    "- chase_action: ONLY for active pursuit/escape, not flashbacks with movement\n\n"
    "Categories: epic_battle, tension, sadness, comedy, romance, horror, "
    "peaceful (calm daily life), emotional_climax, mystery, chase_action\n\n"
    "Reply with ONLY the category name after your reasoning."
)

# ── Combined: narrative intent + visual cues + distinctions ──
PROMPT_COMBINED = (
    "You are analyzing a manga page. Think about what EMOTION the author wants "
    "the reader to feel.\n\n"
    "Focus on: facial expressions, body language, visual effects, panel composition.\n\n"
    "Categories:\n"
    "- epic_battle: thrill of active combat, fighting clash, impact effects\n"
    "- tension: anxiety, unease, suspense, dread of what comes next\n"
    "- sadness: sorrow, nostalgia, regret, loneliness (even with bold text or movement)\n"
    "- comedy: humor, laughter, silly exaggerated reactions\n"
    "- romance: warmth, love, tender closeness, blushing\n"
    "- horror: fear, something disturbing, dark dread\n"
    "- peaceful: relaxation, daily life, calm atmosphere\n"
    "- emotional_climax: inspiration, triumph, peak determination, breakthrough moment\n"
    "- mystery: curiosity, intrigue, hidden meaning\n"
    "- chase_action: urgency of active pursuit or escape\n\n"
    "Key: sadness can have movement/bold text (flashbacks). "
    "emotional_climax must show triumph or determination, not just intensity.\n\n"
    "Reply with ONLY the category name."
)

# ── Two-pass hierarchical classification ──
# Pass 1: visual energy (4 choices — high accuracy)
# Pass 2: narrow sub-category with visual cues (2-3 choices)
#
# Key insight: sadness can be calm/contemplative (not just dark/crying),
# mystery is dark/enigmatic (not calm/questioning)
PROMPT_PASS1 = (
    "What is the overall atmosphere of this manga page?\n"
    "- action: intense, exciting, high energy\n"
    "- calm: quiet, gentle, reflective\n"
    "- dark: threatening, scary, ominous\n"
    "- funny: humorous, silly, comedic\n"
    "Answer with ONLY one word: action, calm, dark, funny"
)

# Pass 2 prompts per broad category — with visual cues
PROMPT_PASS2 = {
    "action": (
        "This manga page has high visual energy. Classify it more precisely.\n"
        "Focus on: facial expressions, body language, text style.\n"
        "- epic_battle: intense fighting, action clash, impact effects, combat poses\n"
        "- chase_action: pursuit, running, escape, fast movement, speed lines\n"
        "- emotional_climax: triumphant speech, breakthrough moment, intense resolve, "
        "motivational words, team rallying, peak determination\n"
        "Reply with ONLY the category name."
    ),
    "calm": (
        "This manga page has a calm/quiet atmosphere. Classify it more precisely.\n"
        "Focus on: facial expressions, body language, background mood.\n"
        "- peaceful: daily life, relaxed conversation, normal expressions, neutral mood\n"
        "- romance: love, intimacy, tender moments, blushing, closeness\n"
        "- sadness: melancholy, regret, loneliness, reflection on loss, contemplative grief\n"
        "Reply with ONLY the category name."
    ),
    "dark": (
        "This manga page has a dark/heavy atmosphere. Classify it more precisely.\n"
        "Focus on: facial expressions, body language, visual effects.\n"
        "- tension: suspense, confrontation, anticipation, high stakes standoff\n"
        "- horror: fear, dark atmosphere, monsters, gore, distorted faces\n"
        "- mystery: enigmatic, shadowy figures, investigation, hidden information, revelation\n"
        "Reply with ONLY the category name."
    ),
    "funny": "comedy",  # Only one option — no need for pass 2
}

PASS1_CATEGORIES = ["action", "calm", "dark", "funny"]

def parse_pass1(raw: str) -> str:
    """Parse broad category from pass 1 response."""
    import re
    clean = re.sub(r'<think>.*?</think>', '', raw, flags=re.DOTALL).strip().lower()
    for cat in PASS1_CATEGORIES:
        if cat in clean:
            return cat
    return f"?({clean[:20]})"

# For structured output: /no_think disables thinking phase so model can output JSON directly
PROMPT_STRUCTURED = (
    "/no_think\n"
    "Classify this manga/webtoon page mood. Choose the single best mood "
    "and rate its intensity from 1 (subtle) to 5 (overwhelming)."
)

# Structured output with explicit enum list in prompt (helps guide the model)
PROMPT_STRUCTURED_VERBOSE = (
    "/no_think\n"
    "Classify this manga/webtoon page mood as ONE of: epic_battle, tension, "
    "sadness, comedy, romance, horror, peaceful, emotional_climax, mystery, "
    "chase_action. Rate intensity from 1 (subtle) to 5 (overwhelming)."
)

# Thinking-in-JSON: let the model reason in the "thinking" field, then classify
PROMPT_STRUCT_THINK = (
    "Analyze this manga/webtoon page. First describe what you see in the 'thinking' field, "
    "then classify the mood as ONE of: epic_battle, tension, sadness, comedy, romance, "
    "horror, peaceful, emotional_climax, mystery, chase_action."
)

# ============================================================
# Perf monitor (simplified from test_models.py)
# ============================================================

def _get_gpu_stats():
    try:
        import pynvml
        handle = pynvml.nvmlDeviceGetHandleByIndex(0)
        mem = pynvml.nvmlDeviceGetMemoryInfo(handle)
        util = pynvml.nvmlDeviceGetUtilizationRates(handle)
        return mem.used / 1024**2, mem.total / 1024**2, util.gpu
    except Exception:
        return 0, 0, 0

class PerfMonitor:
    def __init__(self):
        self.samples = []
        self._running = False
        self._thread = None
        try:
            import pynvml
            pynvml.nvmlInit()
            self.has_gpu = True
        except Exception:
            self.has_gpu = False
        import psutil
        psutil.cpu_percent(interval=None)  # warm up

    def start(self):
        self.samples = []
        self._running = True
        self._thread = threading.Thread(target=self._loop, daemon=True)
        self._thread.start()

    def stop(self):
        self._running = False
        if self._thread:
            self._thread.join(timeout=1)
        if not self.samples:
            return {}
        vram_vals = [s.get("vram", 0) for s in self.samples]
        gpu_vals = [s.get("gpu", 0) for s in self.samples]
        return {
            "vram_peak_mb": max(vram_vals) if vram_vals else 0,
            "vram_total_mb": self.samples[0].get("vram_total", 0),
            "gpu_avg": sum(gpu_vals) / len(gpu_vals) if gpu_vals else 0,
        }

    def _loop(self):
        import psutil
        while self._running:
            s = {}
            if self.has_gpu:
                vram, total, gpu = _get_gpu_stats()
                s["vram"] = vram
                s["vram_total"] = total
                s["gpu"] = gpu
            self.samples.append(s)
            time.sleep(0.2)

# ============================================================
# Image resize helper
# ============================================================

def resize_image(img_path: str, max_size: int) -> str:
    """Resize image to max_size on longest edge. Returns path to temp resized file."""
    img = Image.open(img_path)
    w, h = img.size

    if max(w, h) <= max_size:
        return img_path  # already small enough

    if w > h:
        new_w = max_size
        new_h = int(h * max_size / w)
    else:
        new_h = max_size
        new_w = int(w * max_size / h)

    resized = img.resize((new_w, new_h), Image.LANCZOS)

    # Save to temp dir
    temp_dir = Path("temp_resized")
    temp_dir.mkdir(exist_ok=True)
    out_path = temp_dir / f"{max_size}_{Path(img_path).name}"

    # Convert to JPEG for consistency (smaller file)
    if resized.mode != "RGB":
        resized = resized.convert("RGB")
    out_path = out_path.with_suffix(".jpg")
    resized.save(str(out_path), "JPEG", quality=90)
    return str(out_path)


def get_image_info(img_path: str) -> str:
    """Return WxH and file size."""
    img = Image.open(img_path)
    size_kb = os.path.getsize(img_path) / 1024
    return f"{img.size[0]}x{img.size[1]} ({size_kb:.0f}KB)"

# ============================================================
# Parse response
# ============================================================

def parse_mood(raw: str) -> str:
    """Extract mood from raw response (handles JSON, text, thinking tags)."""
    import re

    # Strip thinking tags if present
    clean = re.sub(r'<think>.*?</think>', '', raw, flags=re.DOTALL).strip()
    if not clean:
        clean = raw.strip()

    # Try JSON parse
    try:
        data = json.loads(clean)
        if isinstance(data, dict):
            return data.get("mood", "?")
    except (json.JSONDecodeError, ValueError):
        pass

    # Find JSON in text
    m = re.search(r'\{[^}]+\}', clean)
    if m:
        try:
            return json.loads(m.group()).get("mood", "?")
        except (json.JSONDecodeError, ValueError):
            pass

    # Find mood keyword
    lower = clean.lower()
    for mood in MOODS:
        if mood in lower:
            return mood

    return f"?({clean[:30]})"


# ============================================================
# Configs
# ============================================================

CONFIGS = {
    # === Resize tests (done) ===
    "baseline": {
        "label": "Baseline (no resize)",
        "prompt": PROMPT_SHORT,
        "num_predict": 5000,
        "resize": None,
        "structured": False,
    },
    "resize_672": {
        "label": "Resize 672px",
        "prompt": PROMPT_SHORT,
        "num_predict": 5000,
        "resize": 672,
        "structured": False,
    },
    # === num_ctx tests — how low can we go? ===
    "ctx_2048": {
        "label": "672px + num_ctx 2048",
        "prompt": PROMPT_SHORT,
        "num_predict": 5000,
        "resize": 672,
        "structured": False,
        "num_ctx": 2048,
    },
    "ctx_4096": {
        "label": "672px + num_ctx 4096",
        "prompt": PROMPT_SHORT,
        "num_predict": 5000,
        "resize": 672,
        "structured": False,
        "num_ctx": 4096,
    },
    "ctx_8192": {
        "label": "672px + num_ctx 8192",
        "prompt": PROMPT_SHORT,
        "num_predict": 5000,
        "resize": 672,
        "structured": False,
        "num_ctx": 8192,
    },
    # === Process priority: lower Ollama priority ===
    "lowprio": {
        "label": "672px + low priority",
        "prompt": PROMPT_SHORT,
        "num_predict": 5000,
        "resize": 672,
        "structured": False,
        "low_priority": True,
    },
    # === Combined: best num_ctx + low priority ===
    "combined": {
        "label": "672px + ctx4096 + lowprio",
        "prompt": PROMPT_SHORT,
        "num_predict": 5000,
        "resize": 672,
        "structured": False,
        "num_ctx": 4096,
        "low_priority": True,
    },
    # === Accuracy optimization ===
    "temp0": {
        "label": "672px + temp 0",
        "prompt": PROMPT_SHORT,
        "num_predict": 5000,
        "resize": 672,
        "structured": False,
        "temperature": 0.0,
    },
    "visual": {
        "label": "672px + visual prompt",
        "prompt": PROMPT_VISUAL,
        "num_predict": 5000,
        "resize": 672,
        "structured": False,
    },
    "visual_temp0": {
        "label": "672px + visual + temp 0",
        "prompt": PROMPT_VISUAL,
        "num_predict": 5000,
        "resize": 672,
        "structured": False,
        "temperature": 0.0,
    },
    "vote3": {
        "label": "672px + vote 3x",
        "prompt": PROMPT_SHORT,
        "num_predict": 5000,
        "resize": 672,
        "structured": False,
        "vote": 3,
    },
    "vote3_visual": {
        "label": "672px + visual + vote 3x",
        "prompt": PROMPT_VISUAL,
        "num_predict": 5000,
        "resize": 672,
        "structured": False,
        "vote": 3,
    },
    "vote3_visual_temp0": {
        "label": "672px + visual + vote 3x + temp 0",
        "prompt": PROMPT_VISUAL,
        "num_predict": 5000,
        "resize": 672,
        "structured": False,
        "vote": 3,
        "temperature": 0.0,
    },
    # === Visual V2 (hybrid) ===
    "visual_v2": {
        "label": "672px + visual v2 (hybrid)",
        "prompt": PROMPT_VISUAL_V2,
        "num_predict": 5000,
        "resize": 672,
        "structured": False,
    },
    "visual_v2_temp0": {
        "label": "672px + visual v2 + temp 0",
        "prompt": PROMPT_VISUAL_V2,
        "num_predict": 5000,
        "resize": 672,
        "structured": False,
        "temperature": 0.0,
    },
    "guided_temp0": {
        "label": "672px + guided + temp 0",
        "prompt": PROMPT_GUIDED,
        "num_predict": 10000,
        "resize": 672,
        "structured": False,
        "temperature": 0.0,
    },
    "guided_v2": {
        "label": "672px + guided v2 (step-by-step + descriptions)",
        "prompt": PROMPT_GUIDED_V2,
        "num_predict": 5000,
        "resize": 672,
        "structured": False,
    },
    "guided_v2_temp0": {
        "label": "672px + guided v2 + temp 0",
        "prompt": PROMPT_GUIDED_V2,
        "num_predict": 8000,
        "resize": 672,
        "structured": False,
        "temperature": 0.0,
    },
    "guided_v3_temp0": {
        "label": "672px + guided v3 (minimal hint) + temp 0",
        "prompt": PROMPT_GUIDED_V3,
        "num_predict": 10000,
        "resize": 672,
        "structured": False,
        "temperature": 0.0,
    },
    # === Narrative / Guided / Combined ===
    "narrative": {
        "label": "672px + narrative intent",
        "prompt": PROMPT_NARRATIVE,
        "num_predict": 10000,
        "resize": 672,
        "structured": False,
    },
    "guided": {
        "label": "672px + guided reasoning",
        "prompt": PROMPT_GUIDED,
        "num_predict": 10000,
        "resize": 672,
        "structured": False,
    },
    "combined": {
        "label": "672px + combined (narrative+visual+distinctions)",
        "prompt": PROMPT_COMBINED,
        "num_predict": 10000,
        "resize": 672,
        "structured": False,
    },
    # === Two-pass hierarchical classification ===
    "two_pass": {
        "label": "672px + two-pass",
        "prompt": PROMPT_PASS1,  # Not used directly, two_pass flag handles it
        "num_predict": 5000,
        "resize": 672,
        "structured": False,
        "two_pass": True,
    },
    "two_pass_visual": {
        "label": "672px + two-pass + visual fallback",
        "prompt": PROMPT_VISUAL,  # Used as fallback if pass1 fails
        "num_predict": 5000,
        "resize": 672,
        "structured": False,
        "two_pass": True,
    },
    # === Structured output (archived) ===
    "struct_think": {
        "label": "Struct think-in-JSON",
        "prompt": PROMPT_STRUCT_THINK,
        "num_predict": 5000,
        "resize": None,
        "structured": "thinking",
    },
    "struct_nothink_5k": {
        "label": "Struct /no_think 5k tokens",
        "prompt": PROMPT_STRUCTURED_VERBOSE,
        "num_predict": 5000,
        "resize": None,
        "structured": True,
    },
}

DEFAULT_MODEL = "qwen3-vl:2b"
MODEL = DEFAULT_MODEL

# ============================================================
# Process priority helper
# ============================================================

def set_ollama_priority(low: bool) -> bool:
    """Set Ollama process priority. Returns True if successful."""
    import psutil
    target_names = {"ollama_llama_server.exe", "ollama.exe", "ollama_llama_server", "ollama"}
    found = False
    for proc in psutil.process_iter(["name", "pid"]):
        try:
            if proc.info["name"] and proc.info["name"].lower() in target_names:
                if low:
                    proc.nice(psutil.BELOW_NORMAL_PRIORITY_CLASS)
                else:
                    proc.nice(psutil.NORMAL_PRIORITY_CLASS)
                console.print(f"  [dim]Set {proc.info['name']} (PID {proc.info['pid']}) → "
                              f"{'BELOW_NORMAL' if low else 'NORMAL'}[/dim]")
                found = True
        except (psutil.NoSuchProcess, psutil.AccessDenied, AttributeError):
            pass
    return found


# ============================================================
# Run one config
# ============================================================

def run_config(config_name: str, config: dict, images: list[str], perf: PerfMonitor) -> list[dict]:
    """Run one configuration on all images. Returns list of result dicts."""
    label = config["label"]
    console.print(f"\n[bold cyan]━━━ {label} ━━━[/bold cyan]")

    # Apply process priority if needed
    if config.get("low_priority"):
        if not set_ollama_priority(low=True):
            console.print("  [yellow]Could not find Ollama process for priority change[/yellow]")

    results = []
    perf.start()

    for img_path in images:
        name = Path(img_path).name

        # Resize if needed
        actual_path = img_path
        if config["resize"]:
            actual_path = resize_image(img_path, config["resize"])
            if actual_path != img_path:
                info = get_image_info(actual_path)
                # Only show resize info for first image
                if not results:
                    console.print(f"  [dim]Resized to {config['resize']}px → {info}[/dim]")

        # Build Ollama call
        temperature = config.get("temperature", 0.1)
        options = {
            "temperature": temperature,
            "num_predict": config["num_predict"],
        }
        if config.get("num_ctx"):
            options["num_ctx"] = config["num_ctx"]

        chat_kwargs = {
            "model": MODEL,
            "messages": [{
                "role": "user",
                "content": config["prompt"],
                "images": [actual_path],
            }],
            "options": options,
        }

        if config["structured"] == "thinking":
            chat_kwargs["format"] = MOOD_SCHEMA_THINKING
        elif config["structured"]:
            chat_kwargs["format"] = MOOD_SCHEMA

        # Two-pass hierarchical classification
        two_pass = config.get("two_pass", False)

        # Voting: run inference N times, take majority
        vote_count = config.get("vote", 1)

        t0 = time.time()
        try:
            if two_pass:
                # Pass 1: broad category
                pass1_kwargs = {
                    "model": MODEL,
                    "messages": [{"role": "user", "content": PROMPT_PASS1, "images": [actual_path]}],
                    "options": options,
                }
                resp1 = ollama.chat(**pass1_kwargs)
                raw = resp1["message"]["content"]
                broad = parse_pass1(raw)

                if broad.startswith("?"):
                    mood = broad
                elif PROMPT_PASS2.get(broad) == "comedy" or broad == "funny":
                    mood = "comedy"
                else:
                    # Pass 2: narrow sub-category
                    pass2_prompt = PROMPT_PASS2.get(broad, PROMPT_PASS2["dark"])
                    pass2_kwargs = {
                        "model": MODEL,
                        "messages": [{"role": "user", "content": pass2_prompt, "images": [actual_path]}],
                        "options": options,
                    }
                    resp2 = ollama.chat(**pass2_kwargs)
                    raw = resp2["message"]["content"]
                    mood = parse_mood(raw)

                elapsed = (time.time() - t0) * 1000
                # Show pass details
                console.print(f"    [dim]pass1: {broad}[/dim]")

            elif vote_count > 1:
                votes = []
                for v in range(vote_count):
                    resp = ollama.chat(**chat_kwargs)
                    raw = resp["message"]["content"]
                    votes.append(parse_mood(raw))
                elapsed = (time.time() - t0) * 1000

                # Majority vote
                from collections import Counter
                vote_counts = Counter(votes)
                mood = vote_counts.most_common(1)[0][0]
                vote_detail = ", ".join(votes)

                # Show vote breakdown if not unanimous
                if len(set(votes)) > 1:
                    console.print(f"    [dim]votes: {vote_detail} → {mood}[/dim]")
            else:
                resp = ollama.chat(**chat_kwargs)
                elapsed = (time.time() - t0) * 1000
                raw = resp["message"]["content"]

                if config["structured"] == "thinking":
                    try:
                        parsed = MoodResultThinking.model_validate_json(raw)
                        mood = parsed.mood
                    except Exception:
                        mood = parse_mood(raw)
                        if mood.startswith("?"):
                            preview = raw[:120].replace('\n', '\\n') if raw.strip() else "(EMPTY)"
                            console.print(f"    [dim red]RAW: {preview}[/dim red]")
                elif config["structured"]:
                    try:
                        parsed = MoodResult.model_validate_json(raw)
                        mood = parsed.mood
                    except Exception:
                        mood = parse_mood(raw)
                        if mood.startswith("?"):
                            preview = raw[:120].replace('\n', '\\n') if raw.strip() else "(EMPTY)"
                            console.print(f"    [dim red]RAW: {preview}[/dim red]")
                else:
                    mood = parse_mood(raw)

        except Exception as e:
            elapsed = (time.time() - t0) * 1000
            mood = f"ERR({str(e)[:30]})"
            raw = str(e)

        # Check correctness
        expected = GROUND_TRUTH.get(name, "?")
        alts = ACCEPTABLE_ALT.get(name, [])
        correct = mood == expected or mood in alts

        result = {
            "config": config_name,
            "image": name,
            "mood": mood,
            "expected": expected,
            "correct": correct,
            "time_ms": round(elapsed, 1),
            "raw": raw if mood.startswith("?") or mood.startswith("ERR") else "",
        }
        results.append(result)

        # Print inline
        status = "[green]OK[/green]" if correct else "[red]MISS[/red]"
        console.print(f"  {name:<12} {mood:<22} expected: {expected:<22} {status}  {elapsed:.0f}ms")

    perf_stats = perf.stop()

    # Score
    correct_count = sum(1 for r in results if r["correct"])
    total = len(results)
    avg_time = sum(r["time_ms"] for r in results) / total if total else 0

    console.print(f"\n  [bold]Score: {correct_count}/{total} ({correct_count/total*100:.0f}%)[/bold]  "
                  f"Avg: {avg_time:.0f}ms  "
                  f"Total: {sum(r['time_ms'] for r in results)/1000:.1f}s")

    if perf_stats.get("vram_peak_mb"):
        vram_pct = perf_stats["vram_peak_mb"] / perf_stats["vram_total_mb"] * 100
        console.print(f"  VRAM: {perf_stats['vram_peak_mb']:.0f} MB ({vram_pct:.0f}%)  "
                      f"GPU avg: {perf_stats['gpu_avg']:.0f}%")

    # Restore normal priority
    if config.get("low_priority"):
        set_ollama_priority(low=False)

    return results, perf_stats


# ============================================================
# Main
# ============================================================

def main():
    parser = argparse.ArgumentParser(description="Optimization benchmark for Qwen3-VL 2B")
    parser.add_argument("--model", type=str, default=DEFAULT_MODEL,
                        help=f"Ollama model to use (default: {DEFAULT_MODEL})")
    parser.add_argument("--config", nargs="+", choices=list(CONFIGS.keys()),
                        help="Run specific configs only")
    parser.add_argument("--runs", type=int, default=1,
                        help="Number of runs per config (for stable timing)")
    args = parser.parse_args()

    global MODEL
    MODEL = args.model

    console.print(Panel.fit(
        "[bold]Manga Mood AI — Optimization Benchmark[/bold]\n"
        f"Model: {MODEL} | Configs: {len(CONFIGS)}",
        border_style="cyan",
    ))

    # Collect images (exclude subdirs used by other benchmarks)
    exclude_dirs = {"bluelock-sequence"}
    images = []
    for ext in ("*.jpg", "*.jpeg", "*.png", "*.webp"):
        for p in Path("test-images").rglob(ext):
            if not any(part in exclude_dirs for part in p.parts):
                images.append(str(p))
    images.sort()
    console.print(f"\nFound [bold]{len(images)}[/bold] test images")

    # Show original image sizes
    console.print("\n[dim]Original image sizes:[/dim]")
    for img in images:
        console.print(f"  [dim]{Path(img).name}: {get_image_info(img)}[/dim]")

    # Check model
    try:
        ollama.show(MODEL)
        console.print(f"\n[green]Model {MODEL} ready[/green]")
    except Exception:
        console.print(f"\n[red]Model {MODEL} not found. Run: ollama pull {MODEL}[/red]")
        return

    configs_to_run = args.config or list(CONFIGS.keys())
    all_results = {}
    all_perf = {}

    for config_name in configs_to_run:
        config = CONFIGS[config_name]

        if args.runs > 1:
            # Multi-run: average timing
            all_run_results = []
            for run in range(args.runs):
                console.print(f"\n[dim]-- Run {run+1}/{args.runs} --[/dim]")
                perf = PerfMonitor()
                results, perf_stats = run_config(config_name, config, images, perf)
                all_run_results.append(results)

            # Use last run's moods, average the times
            final_results = all_run_results[-1]
            for i, r in enumerate(final_results):
                times = [all_run_results[run][i]["time_ms"] for run in range(args.runs)]
                r["time_ms"] = sum(times) / len(times)
                r["time_min"] = min(times)
                r["time_max"] = max(times)

            all_results[config_name] = final_results
            all_perf[config_name] = perf_stats
        else:
            perf = PerfMonitor()
            results, perf_stats = run_config(config_name, config, images, perf)
            all_results[config_name] = results
            all_perf[config_name] = perf_stats

    # ── Summary table ──
    console.print("\n")
    table = Table(
        title="Optimization Comparison — Qwen3-VL 2B",
        box=box.ROUNDED,
        show_lines=True,
    )
    table.add_column("Config", style="bold", min_width=28)
    table.add_column("Score", justify="center", min_width=10)
    table.add_column("Avg Time", justify="right")
    table.add_column("Total Time", justify="right")
    table.add_column("VRAM", justify="right")
    table.add_column("GPU", justify="right")
    table.add_column("Misses", style="dim")

    for config_name in configs_to_run:
        results = all_results[config_name]
        perf_stats = all_perf.get(config_name, {})

        correct = sum(1 for r in results if r["correct"])
        total = len(results)
        avg_time = sum(r["time_ms"] for r in results) / total
        total_time = sum(r["time_ms"] for r in results) / 1000
        misses = [r["image"] for r in results if not r["correct"]]

        score_str = f"{correct}/{total} ({correct/total*100:.0f}%)"
        vram_str = ""
        gpu_str = ""
        if perf_stats.get("vram_peak_mb"):
            vram_pct = perf_stats["vram_peak_mb"] / perf_stats["vram_total_mb"] * 100
            vram_str = f"{perf_stats['vram_peak_mb']:.0f}MB ({vram_pct:.0f}%)"
            gpu_str = f"{perf_stats['gpu_avg']:.0f}%"

        # Color score
        if correct >= 12:
            score_style = "bold green"
        elif correct >= 10:
            score_style = "bold yellow"
        else:
            score_style = "bold red"

        table.add_row(
            CONFIGS[config_name]["label"],
            f"[{score_style}]{score_str}[/{score_style}]",
            f"{avg_time:.0f}ms",
            f"{total_time:.1f}s",
            vram_str,
            gpu_str,
            ", ".join(misses) if misses else "—",
        )

    console.print(table)

    # ── Per-image comparison ──
    console.print("\n")
    detail_table = Table(
        title="Per-Image Detail",
        box=box.ROUNDED,
        show_lines=True,
    )
    detail_table.add_column("Image", style="dim", max_width=12)
    detail_table.add_column("Expected", style="bold")

    for config_name in configs_to_run:
        detail_table.add_column(CONFIGS[config_name]["label"][:20], min_width=18)

    for img in images:
        name = Path(img).name
        expected = GROUND_TRUTH.get(name, "?")
        row = [name, expected]

        for config_name in configs_to_run:
            r = next((r for r in all_results[config_name] if r["image"] == name), None)
            if r:
                mood = r["mood"]
                ms = r["time_ms"]
                if r["correct"]:
                    row.append(f"[green]{mood}[/green] ({ms:.0f}ms)")
                else:
                    row.append(f"[red]{mood}[/red] ({ms:.0f}ms)")
            else:
                row.append("—")

        detail_table.add_row(*row)

    console.print(detail_table)

    # Cleanup temp files
    temp_dir = Path("temp_resized")
    if temp_dir.exists():
        import shutil
        shutil.rmtree(temp_dir, ignore_errors=True)
        console.print("\n[dim]Cleaned up temp resized images[/dim]")

    # Save results
    results_path = Path("results/optimizations.json")
    results_path.parent.mkdir(exist_ok=True)
    save_data = {
        config_name: {
            "label": CONFIGS[config_name]["label"],
            "results": all_results[config_name],
            "perf": all_perf.get(config_name, {}),
        }
        for config_name in configs_to_run
    }
    with open(results_path, "w", encoding="utf-8") as f:
        json.dump(save_data, f, indent=2, ensure_ascii=False)
    console.print(f"[dim]Results saved to {results_path}[/dim]")


if __name__ == "__main__":
    main()
