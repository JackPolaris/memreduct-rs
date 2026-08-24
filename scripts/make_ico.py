#!/usr/bin/env python3
"""Generate src-tauri/icons/icon.ico from existing PNG files (PNG-embedded ICO)."""
import struct
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ICONS = ROOT / "src-tauri" / "icons"

sizes = [
    (ICONS / "32x32.png", 32),
    (ICONS / "128x128.png", 128),
    (ICONS / "256x256.png", 256),
]

entries = []
images = []
for path, size in sizes:
    data = path.read_bytes()
    images.append(data)
    # ICONDIRENTRY: width(1) height(1) colors(1) reserved(1) planes(2) bpp(2) size(4) offset(4)
    b = size if size < 256 else 0  # 256 encoded as 0
    entries.append(struct.pack("<BBBBHHII", b, b, 0, 0, 1, 32, len(data), 0))

header = struct.pack("<HHH", 0, 1, len(sizes))

offset = 6 + 16 * len(sizes)
for i, entry in enumerate(entries):
    entries[i] = entry[:12] + struct.pack("<I", offset)
    offset += len(images[i])

out = ICONS / "icon.ico"
out.write_bytes(header + b"".join(entries) + b"".join(images))
print(f"wrote {out} ({out.stat().st_size} bytes)")
