#!/usr/bin/env python3
"""Seed ClipnPaste history.db with dummy content for marketing screenshots.

Usage:
  python3 scripts/seed-demo-history.py          # seed (backs up first)
  python3 scripts/seed-demo-history.py restore  # restore backup
"""

from __future__ import annotations

import base64
import io
import shutil
import sqlite3
import sys
import time
import uuid
from pathlib import Path

try:
    from PIL import Image, ImageDraw
except ImportError:
    Image = None  # type: ignore

DATA_DIR = Path.home() / ".local" / "share" / "clipnpaste"
DB_PATH = DATA_DIR / "history.db"
BACKUP_PATH = DATA_DIR / "history.db.bak-screenshot"


def make_demo_png() -> tuple[str, str]:
    """Return (full content data URL, preview data URL) for a colorful sample image."""
    if Image is None:
        # 1x1 PNG fallback
        raw = base64.b64decode(
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg=="
        )
        b64 = base64.b64encode(raw).decode("ascii")
        url = f"data:image/png;base64,{b64}"
        return url, url

    img = Image.new("RGBA", (320, 180), (15, 23, 42, 255))
    draw = ImageDraw.Draw(img)
    draw.rectangle([20, 20, 300, 160], fill=(56, 189, 248, 255), outline=(14, 165, 233, 255), width=3)
    draw.ellipse([100, 50, 220, 130], fill=(244, 63, 94, 255))
    draw.text((40, 30), "ClipnPaste", fill=(15, 23, 42, 255))
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    full_b64 = base64.b64encode(buf.getvalue()).decode("ascii")
    content = f"data:image/png;base64,{full_b64}"

    thumb = img.copy()
    thumb.thumbnail((240, 240))
    tbuf = io.BytesIO()
    thumb.save(tbuf, format="PNG")
    preview = f"data:image/png;base64,{base64.b64encode(tbuf.getvalue()).decode('ascii')}"
    return content, preview


def backup() -> None:
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    if DB_PATH.exists():
        # Never overwrite an existing screenshot backup (preserves real user data).
        if BACKUP_PATH.exists():
            print(f"Keeping existing backup at {BACKUP_PATH}")
        else:
            shutil.copy2(DB_PATH, BACKUP_PATH)
            print(f"Backed up to {BACKUP_PATH}")
    else:
        print("No existing DB; will create new")


def restore() -> None:
    if not BACKUP_PATH.exists():
        print(f"No backup at {BACKUP_PATH}", file=sys.stderr)
        sys.exit(1)
    shutil.copy2(BACKUP_PATH, DB_PATH)
    print(f"Restored {DB_PATH} from backup")


def seed() -> None:
    backup()
    conn = sqlite3.connect(DB_PATH)
    conn.execute(
        """
        CREATE TABLE IF NOT EXISTS items (
            id TEXT PRIMARY KEY,
            item_type TEXT NOT NULL,
            content TEXT NOT NULL,
            preview TEXT NOT NULL,
            pinned INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL
        )
        """
    )
    conn.execute("DELETE FROM items")

    now = int(time.time() * 1000)
    image_content, image_preview = make_demo_png()

    rows = [
        # pinned first in order if we sort by pinned desc, created_at desc
        (
            str(uuid.uuid4()),
            "text",
            "📌 Pinned: meeting notes for Friday standup",
            "📌 Pinned: meeting notes for Friday standup",
            1,
            now + 50,
        ),
        (
            str(uuid.uuid4()),
            "text",
            "Hello from ClipnPaste 👋\nWindows 11-style clipboard history for Linux.",
            "Hello from ClipnPaste 👋\nWindows 11-style clipboard history for Linux.",
            0,
            now + 40,
        ),
        (
            str(uuid.uuid4()),
            "text",
            "fn main() {\n    println!(\"clipnpaste\");\n}",
            "fn main() {\n    println!(\"clipnpaste\");\n}",
            0,
            now + 30,
        ),
        (
            str(uuid.uuid4()),
            "text",
            "https://github.com/LinkofHyrule89/ClipnPaste",
            "https://github.com/LinkofHyrule89/ClipnPaste",
            0,
            now + 20,
        ),
        (
            str(uuid.uuid4()),
            "image",
            image_content,
            image_preview,
            0,
            now + 10,
        ),
        (
            str(uuid.uuid4()),
            "text",
            "sudo apt install ./ClipnPaste_*_amd64.deb",
            "sudo apt install ./ClipnPaste_*_amd64.deb",
            0,
            now,
        ),
    ]

    conn.executemany(
        "INSERT INTO items (id, item_type, content, preview, pinned, created_at) VALUES (?,?,?,?,?,?)",
        rows,
    )
    conn.commit()
    conn.close()
    print(f"Seeded {len(rows)} demo items into {DB_PATH}")
    print("Restart clipnpaste or reopen Super+V so the UI reloads history.")


def main() -> None:
    if len(sys.argv) > 1 and sys.argv[1] == "restore":
        restore()
    else:
        seed()


if __name__ == "__main__":
    main()
