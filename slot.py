"""
slot.py — Switch between KeyToMusic data slots.

Usage:
    py slot.py                  List all slots
    py slot.py <name>           Switch to slot (creates if new)
    py slot.py --erase <name>   Delete a slot
"""

import sys
import os
import shutil
from pathlib import Path

APPDATA = Path(os.environ["APPDATA"])
DATA_DIR = APPDATA / "KeyToMusic"
SLOTS_DIR = APPDATA / "KeyToMusic_slots"
CURRENT_FILE = SLOTS_DIR / ".current"

# User data only — bin/ and logs/ stay untouched
USER_DATA = ["config.json", "profiles", "cache", "imported_sounds", "discovery"]


def get_current():
    if CURRENT_FILE.exists():
        name = CURRENT_FILE.read_text().strip()
        return name or None
    return None


def set_current(name):
    SLOTS_DIR.mkdir(parents=True, exist_ok=True)
    CURRENT_FILE.write_text(name)


def has_user_data():
    return any((DATA_DIR / item).exists() for item in USER_DATA)


def save_slot(name):
    slot = SLOTS_DIR / name
    slot.mkdir(parents=True, exist_ok=True)
    for item in USER_DATA:
        src = DATA_DIR / item
        if src.exists():
            dst = slot / item
            if dst.exists():
                shutil.rmtree(dst) if dst.is_dir() else dst.unlink()
            shutil.move(str(src), str(dst))


def load_slot(name):
    slot = SLOTS_DIR / name
    if not slot.exists():
        return
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    for item in USER_DATA:
        src = slot / item
        if src.exists():
            dst = DATA_DIR / item
            if dst.exists():
                shutil.rmtree(dst) if dst.is_dir() else dst.unlink()
            shutil.move(str(src), str(dst))
    # Clean up empty slot dir
    if slot.exists() and not any(slot.iterdir()):
        slot.rmdir()


def list_slots():
    current = get_current()
    saved = sorted(d.name for d in SLOTS_DIR.iterdir() if d.is_dir()) if SLOTS_DIR.exists() else []

    # Current active slot isn't in SLOTS_DIR (its data is live), so add it
    all_names = sorted(set(saved + ([current] if current else [])))

    if not all_names:
        print("No slots yet. Use: py slot.py <name>")
        return

    print("Slots:")
    for name in all_names:
        marker = "  <-- active" if name == current else ""
        print(f"  {name}{marker}")


def switch_to(target):
    current = get_current()

    if current == target:
        print(f"Already on '{target}'.")
        return

    # First time: unnamed data exists, ask for a name
    if current is None and has_user_data():
        print("Current data has no slot name.")
        name = input("Name for current data: ").strip()
        if not name:
            print("Aborted.")
            return
        if name == target:
            print(f"Can't save as '{name}' — that's the slot you're switching to.")
            return
        save_slot(name)
        print(f"  Saved current data as '{name}'")
    elif current:
        save_slot(current)
        print(f"  Saved '{current}'")

    # Load target
    slot = SLOTS_DIR / target
    if slot.exists() and any(slot.iterdir()):
        load_slot(target)
        print(f"  Loaded '{target}'")
    else:
        print(f"  Created '{target}' (empty)")

    set_current(target)
    print(f"Active: {target}")


def erase(name):
    current = get_current()
    if name == current:
        print(f"Can't erase '{name}' — it's active. Switch to another slot first.")
        return
    slot = SLOTS_DIR / name
    if slot.exists():
        shutil.rmtree(slot)
        print(f"Erased '{name}'.")
    else:
        print(f"Slot '{name}' not found.")


def main():
    args = sys.argv[1:]

    if not args:
        list_slots()
        return

    if args[0] in ("--erase", "-e"):
        if len(args) < 2:
            print("Usage: py slot.py --erase <name>")
            return
        erase(args[1])
        return

    if args[0] in ("--list", "-l"):
        list_slots()
        return

    switch_to(args[0])


if __name__ == "__main__":
    main()
