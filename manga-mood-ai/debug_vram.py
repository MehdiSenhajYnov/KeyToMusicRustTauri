"""Test Qwen2.5-VL 7B with limited GPU layers to avoid freezing.
num_gpu controls how many model layers go on GPU vs CPU.
Lower = less VRAM, slower but won't freeze your PC."""

import ollama
import time
from pathlib import Path
from rich.console import Console
from rich.table import Table
from rich import box

console = Console()

# 3 diverse test images
test_images = [
    "test-images/manga/1.jpg",    # epic_battle
    "test-images/manga/6.jpg",    # romance
    "test-images/manga/8.jpg",    # horror
]

PROMPT = """Analyze this manga/webtoon page and classify its mood/atmosphere.

Choose ONE primary mood from this list:
- epic_battle, tension, sadness, comedy, romance, horror, peaceful, emotional_climax, mystery, chase_action

Respond ONLY with JSON: {"mood": "...", "intensity": 1-5, "reason": "<10 words>"}"""

# Test different num_gpu values
# 0 = full CPU, 15 = half GPU, 99 = full GPU (default)
GPU_CONFIGS = [
    (0,  "CPU only (0 layers)"),
    (10, "10 GPU layers (~light)"),
    (20, "20 GPU layers (~half)"),
    (99, "Full GPU (default)"),
]

table = Table(title="Qwen2.5-VL 7B — GPU Layer Limits", box=box.ROUNDED, show_lines=True)
table.add_column("Config", style="bold", min_width=25)
table.add_column("1.jpg (battle)", min_width=15)
table.add_column("6.jpg (romance)", min_width=15)
table.add_column("8.jpg (horror)", min_width=15)
table.add_column("Avg Time", justify="right")

for num_gpu, label in GPU_CONFIGS:
    console.print(f"\n[cyan]Testing: {label}...[/cyan]")
    moods = []
    times = []

    for img in test_images:
        if not Path(img).exists():
            moods.append("?")
            times.append(0)
            continue

        t0 = time.time()
        try:
            r = ollama.chat(
                model="qwen2.5vl:7b",
                messages=[{"role": "user", "content": PROMPT, "images": [img]}],
                options={
                    "temperature": 0.1,
                    "num_predict": 150,
                    "num_gpu": num_gpu,
                },
            )
            elapsed = (time.time() - t0) * 1000
            raw = r["message"]["content"]

            import json, re
            try:
                mood = json.loads(raw.strip()).get("mood", "?")
            except:
                m = re.search(r'\{[^}]+\}', raw)
                mood = json.loads(m.group()).get("mood", "?") if m else "?"

            moods.append(mood)
            times.append(elapsed)
            console.print(f"  {Path(img).name}: {mood} ({elapsed:.0f}ms)")

        except Exception as e:
            elapsed = (time.time() - t0) * 1000
            moods.append(f"ERR")
            times.append(elapsed)
            console.print(f"  [red]{Path(img).name}: ERROR ({e})[/red]")

    avg = sum(times) / len(times) if times else 0
    table.add_row(
        label,
        f"{moods[0]} ({times[0]:.0f}ms)" if len(moods) > 0 else "?",
        f"{moods[1]} ({times[1]:.0f}ms)" if len(moods) > 1 else "?",
        f"{moods[2]} ({times[2]:.0f}ms)" if len(moods) > 2 else "?",
        f"{avg:.0f}ms",
    )

console.print("\n")
console.print(table)
console.print("\n[dim]Lower num_gpu = less VRAM used = PC won't freeze, but slower inference.[/dim]")
