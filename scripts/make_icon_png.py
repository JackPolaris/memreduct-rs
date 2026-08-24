#!/usr/bin/env python3
"""Generate the app icon PNGs directly (no SVG renderer needed).

The upstream `清理.svg` cannot be rendered by libvips/sharp (renders blank),
which is why `tauri icon` produced transparent PNGs and the exe icon showed as
a green/black placeholder. This script draws the brand icon procedurally:
a purple rounded-circle background with a white "memory modules" glyph.
"""
import struct
import zlib

ACCENT = (0x67, 0x6E, 0xBB)  # #676EBB brand purple


def png_chunk(tag: bytes, data: bytes) -> bytes:
    c = tag + data
    return struct.pack(">I", len(data)) + c + struct.pack(">I", zlib.crc32(c) & 0xFFFFFFFF)


def write_png(path: str, size: int) -> None:
    cx = cy = (size - 1) / 2.0
    radius = size * 0.47
    rows = []
    for y in range(size):
        row = bytearray()
        for x in range(size):
            dx = x - cx
            dy = y - cy
            d = (dx * dx + dy * dy) ** 0.5

            if d <= radius:
                # Inside the purple circle.
                # Draw three white "memory stick" bars in the centre.
                in_bar = False
                bar_half = size * 0.09
                bar_len = size * 0.30
                gap = size * 0.11
                if abs(dx) <= bar_len:
                    for off in (-gap, 0.0, gap):
                        if abs(dy - off) <= bar_half:
                            in_bar = True
                            break
                r, g, b = (255, 255, 255) if in_bar else ACCENT
                a = 255
            else:
                r = g = b = a = 0

            row += bytes((r, g, b, a))
        rows.append(bytes(row))

    raw = b"".join(b"\x00" + row for row in rows)
    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    png = (
        b"\x89PNG\r\n\x1a\n"
        + png_chunk(b"IHDR", ihdr)
        + png_chunk(b"IDAT", zlib.compress(raw, 9))
        + png_chunk(b"IEND", b"")
    )
    with open(path, "wb") as f:
        f.write(png)
    print(f"wrote {path} ({len(png)} bytes)")


if __name__ == "__main__":
    import os

    out = "src-tauri/icons"
    for name, size in [
        ("32x32.png", 32),
        ("128x128.png", 128),
        ("128x128@2x.png", 256),
        ("256x256.png", 256),
        ("icon.png", 512),
    ]:
        write_png(os.path.join(out, name), size)
    print("done")
