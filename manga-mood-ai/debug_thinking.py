"""Compare Qwen3-VL 2B: thinking ON vs OFF on all 13 images."""
import ollama
import json
import re
import time
from pathlib import Path
from rich.console import Console
from rich.table import Table
from rich import box

console = Console()

PROMPT_NO_THINK = """/no_think
Classify this manga/webtoon page mood as ONE of: epic_battle, tension, sadness, comedy, romance, horror, peaceful, emotional_climax, mystery, chase_action.
Reply ONLY with JSON: {"mood": "...", "intensity": 1-5, "secondary_mood": "...", "reason": "<10 words>"}"""

PROMPT_THINK = """Classify this manga/webtoon page mood as ONE of: epic_battle, tension, sadness, comedy, romance, horror, peaceful, emotional_climax, mystery, chase_action.
Reply ONLY with JSON: {"mood": "...", "intensity": 1-5, "secondary_mood": "...", "reason": "<10 words>"}"""

# Simpler prompt = less thinking tokens wasted
PROMPT_THINK_SHORT = """What is the mood of this manga page? Pick ONE: epic_battle, tension, sadness, comedy, romance, horror, peaceful, emotional_climax, mystery, chase_action. Reply with just the mood word."""

MOODS = ["epic_battle", "tension", "sadness", "comedy", "romance", "horror", "peaceful", "emotional_climax", "mystery", "chase_action"]

def parse(raw: str) -> str:
    try:
        return json.loads(raw.strip()).get("mood", "?")
    except:
        pass
    m = re.search(r'\{[^}]+\}', raw)
    if m:
        try:
            return json.loads(m.group()).get("mood", "?")
        except:
            pass
    for mood in MOODS:
        if mood in raw.lower():
            return mood
    return "?" if not raw.strip() else f"?({raw[:40]})"

images = sorted(Path("test-images").rglob("*"))
images = [str(p) for p in images if p.suffix.lower() in (".jpg", ".jpeg", ".png", ".webp")]

table = Table(title="Qwen3-VL 2B: Think (2k) vs Think (5k) vs Think Short", box=box.ROUNDED, show_lines=True)
table.add_column("Image", style="dim", max_width=12)
table.add_column("Think 2k", style="bold")
table.add_column("Time", justify="right")
table.add_column("Think 5k", style="bold cyan")
table.add_column("Time", justify="right")
table.add_column("Short 5k", style="bold green")
table.add_column("Time", justify="right")

for img in images:
    name = Path(img).name

    # Think 2k (baseline from before)
    t0 = time.time()
    r1 = ollama.chat(
        model="qwen3-vl:2b",
        messages=[{"role": "user", "content": PROMPT_THINK, "images": [img]}],
        options={"temperature": 0.1, "num_predict": 2000},
    )
    t1 = (time.time() - t0) * 1000
    raw1 = r1["message"]["content"]
    mood1 = parse(raw1)

    # Think 5k (more room for thinking)
    t0 = time.time()
    r2 = ollama.chat(
        model="qwen3-vl:2b",
        messages=[{"role": "user", "content": PROMPT_THINK, "images": [img]}],
        options={"temperature": 0.1, "num_predict": 5000},
    )
    t2 = (time.time() - t0) * 1000
    raw2 = r2["message"]["content"]
    mood2 = parse(raw2)

    # Short prompt + 5k tokens
    t0 = time.time()
    r3 = ollama.chat(
        model="qwen3-vl:2b",
        messages=[{"role": "user", "content": PROMPT_THINK_SHORT, "images": [img]}],
        options={"temperature": 0.1, "num_predict": 5000},
    )
    t3 = (time.time() - t0) * 1000
    raw3 = r3["message"]["content"]
    mood3 = parse(raw3)

    table.add_row(name, mood1, f"{t1:.0f}ms", mood2, f"{t2:.0f}ms", mood3, f"{t3:.0f}ms")

    # Show raw output for debugging
    for label, raw, mood in [("Think 2k", raw1, mood1), ("Think 5k", raw2, mood2), ("Short 5k", raw3, mood3)]:
        preview = raw[:120].replace('\n', ' ') if raw.strip() else "(EMPTY)"
        if mood.startswith("?"):
            console.print(f"  [red]{label} RAW:[/red] {preview}")

console.print(table)
