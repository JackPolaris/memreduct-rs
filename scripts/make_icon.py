"""Generate a simple green rounded-square application icon (ICO + PNG).

Used only to satisfy Tauri's resource build; a real branded icon can replace it.
Run from the project root: `python scripts/make_icon.py`.
"""
import os
import struct
import sys

W = H = 32
SIZE = 256


def png_from_rgba(rows):
    """rows: list of list of (r,g,b,a). Produce a PNG byte string."""
    import zlib

    def chunk(t, d):
        c = t + d
        return struct.pack(">I", len(d)) + c + struct.pack(">I", zlib.crc32(c) & 0xFFFFFFFF)

    h = len(rows)
    w = len(rows[0])
    raw = b""
    for row in rows:
        raw += b"\x00" + b"".join(struct.pack("BBBB", *p) for p in row)
    ihdr = struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b"")


def rgba_grid(sz):
    rows = []
    for y in range(sz):
        row = []
        for x in range(sz):
            edge = max(4, sz // 8)
            inside = True
            if x < edge and y < edge:
                inside = False
            elif x >= sz - edge and y < edge:
                inside = False
            elif x < edge and y >= sz - edge:
                inside = False
            elif x >= sz - edge and y >= sz - edge:
                inside = False
            if inside:
                row.append((0x00, 0x80, 0x40, 0xFF))
            else:
                row.append((0, 0, 0, 0))
        rows.append(row)
    return rows


def write_png(path, sz):
    rows = rgba_grid(sz)
    png = png_from_rgba(rows)
    with open(path, "wb") as f:
        f.write(png)


def write_ico(path, sz=32):
    rows = rgba_grid(sz)
    bih = struct.pack("<IiiHHIIiiII", 40, sz, sz, 1, 32, 0, 0, 0, 0, 0, 0)
    # BGRA order in file
    xor = b"".join(struct.pack("<BBBB", b, g, r, a) for row in rows for (r, g, b, a) in row)
    andmask = b"\x00" * (sz * ((sz // 8) + (4 if sz % 8 else 0)))
    img = bih + xor + andmask
    header = struct.pack("<HHH", 0, 1, 1)
    entry = struct.pack("<BBBBHHII", sz, sz, 0, 0, 1, 32, len(img), 22)
    with open(path, "wb") as f:
        f.write(header + entry + img)


def main():
    out = os.path.join("src-tauri", "icons")
    os.makedirs(out, exist_ok=True)
    write_png(os.path.join(out, "32x32.png"), 32)
    write_png(os.path.join(out, "128x128.png"), 128)
    write_png(os.path.join(out, "128x128@2x.png"), 256)
    write_ico(os.path.join(out, "icon.ico"))
    print("icons written to", out)


if __name__ == "__main__":
    sys.exit(main())
