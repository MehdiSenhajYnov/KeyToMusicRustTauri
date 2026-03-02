"""Quick debug: see raw Qwen3-VL 2B responses."""
import ollama
import time
from pathlib import Path

test_images = [
    "test-images/manga/1.jpg",   # DBZ epic battle
    "test-images/manga/6.jpg",   # romance kiss
    "test-images/manga/8.jpg",   # horror
    "test-images/manga/7.jpg",   # sadness
    "test-images/webtoon/9.png", # horror webtoon
]

PROMPTS = {
    "no_think": """/no_think
Classify this manga/webtoon page mood as ONE of: epic_battle, tension, sadness, comedy, romance, horror, peaceful, emotional_climax, mystery, chase_action.
Reply ONLY with JSON: {"mood": "...", "intensity": 1-5, "reason": "<10 words>"}""",

    "simple": """What mood does this manga page convey? Pick one: epic_battle, tension, sadness, comedy, romance, horror, peaceful, emotional_climax, mystery, chase_action. Just say the mood word.""",

    "describe_first": """Describe what you see in this manga/comic image in one sentence, then classify the mood as one of: epic_battle, tension, sadness, comedy, romance, horror, peaceful, emotional_climax.""",
}

for img in test_images:
    if not Path(img).exists():
        continue
    print(f"\n{'='*60}")
    print(f"IMAGE: {img}")

    for label, prompt in PROMPTS.items():
        start = time.time()
        response = ollama.chat(
            model="qwen3-vl:2b",
            messages=[{"role": "user", "content": prompt, "images": [img]}],
            options={"temperature": 0.1, "num_predict": 500},
        )
        elapsed = (time.time() - start) * 1000

        raw = response["message"]["content"]
        # Show first 200 chars
        display = raw[:200].replace('\n', ' ') if raw else "(EMPTY)"
        print(f"  [{label:>16}] {elapsed:>6.0f}ms | {display}")

    print(f"{'='*60}")
