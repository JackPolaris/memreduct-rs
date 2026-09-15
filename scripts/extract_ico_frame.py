#!/usr/bin/env python3
"""Inspect the ICO at src-tauri/icons/icon.ico.

Prints the frame table and extracts the largest embedded PNG frame for manual
inspection.

The checked-in version of this script hard-coded an absolute
``/tmp/ico_frame.png`` output path, so it crashed with `FileNotFoundError` on
Windows — the only platform this project builds for. The frame is now written to
the system temporary directory (or to an explicit path given as argv[1]).
"""
import os
import struct
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ICO = os.path.join(ROOT, "src-tauri", "icons", "icon.ico")


def main() -> int:
    with open(ICO, "rb") as fh:
        data = fh.read()

    if len(data) < 6:
        print(f"not an ICO file: {ICO}", file=sys.stderr)
        return 1

    _reserved, typ, count = struct.unpack("<HHH", data[:6])
    print(f"ICONDIR: type={typ} count={count} bytes={len(data)}")

    off = 6
    entries = []
    for _ in range(count):
        w, h, _colors, _res, _planes, _bpp, size, imgoff = struct.unpack(
            "<BBBBHHII", data[off : off + 16]
        )
        # A stored 0 means 256 pixels.
        entries.append((w or 256, h or 256, imgoff, size))
        off += 16

    if not entries:
        print("no frames in ICO", file=sys.stderr)
        return 1

    w, h, imgoff, size = max(entries, key=lambda e: e[0] * e[1])
    png = data[imgoff : imgoff + size]
    print(f"largest frame: {w}x{h} size={size} magic={png[:4].hex()}")

    out = (
        sys.argv[1]
        if len(sys.argv) > 1
        else os.path.join(tempfile.gettempdir(), "ico_frame.png")
    )
    with open(out, "wb") as fh:
        fh.write(png)
    print(f"saved {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
