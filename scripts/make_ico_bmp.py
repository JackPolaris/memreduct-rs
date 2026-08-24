#!/usr/bin/env python3
"""Generate src-tauri/icons/icon.ico as a BMP-embedded ICO (max compatibility).

Windows .ico files traditionally contain DIB/BMP data; PNG-embedded ICOs work
in Explorer but mis-render in some APIs (e.g. System.Drawing reads wrong
colours). This converts the resvg PNG frames into 32bpp BGRA DIB entries.
"""
import struct
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ICONS = ROOT / "src-tauri" / "icons"

sizes = [(32, "32x32.png"), (128, "128x128.png"), (256, "256x256.png")]
frames = []


def png_pixels(path: Path, size: int):
    """Decode RGBA pixel rows WITHOUT external deps — no, use a tiny PNG parser
    is too much. Instead we read the PNG with a raw IDAT decoder below."""
    data = path.read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n"

    pos = 8
    width = height = 0
    bit_depth = color_type = 0
    idat = b""

    while pos < len(data):
        length = struct.unpack(">I", data[pos:pos + 4])[0]
        typ = data[pos + 4:pos + 8]
        chunk = data[pos + 8:pos + 8 + length]
        pos += 12 + length

        if typ == b"IHDR":
            width, height, bit_depth, color_type = struct.unpack(">IIBB", chunk[:10])
        elif typ == b"IDAT":
            idat += chunk
        elif typ == b"IEND":
            break

    import zlib

    raw = zlib.decompress(idat)

    # Filter reconstruction (only 8-bit RGBA, non-interlaced is produced by resvg).
    bpp = 4
    stride = width * bpp
    out = bytearray()
    prev = bytearray(stride)
    pos = 0
    for _y in range(height):
        f = raw[pos]
        pos += 1
        line = bytearray(raw[pos:pos + stride])
        pos += stride
        if f == 0:
            pass
        elif f == 1:  # Sub
            for i in range(bpp, stride):
                line[i] = (line[i] + line[i - bpp]) & 0xFF
        elif f == 2:  # Up
            for i in range(stride):
                line[i] = (line[i] + prev[i]) & 0xFF
        elif f == 3:  # Average
            for i in range(stride):
                a = line[i - bpp] if i >= bpp else 0
                line[i] = (line[i] + ((a + prev[i]) >> 1)) & 0xFF
        elif f == 4:  # Paeth
            for i in range(stride):
                a = line[i - bpp] if i >= bpp else 0
                b = prev[i]
                c = prev[i - bpp] if i >= bpp else 0
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                pr = a if (pa <= pb and pa <= pc) else (b if pb <= pc else c)
                line[i] = (line[i] + pr) & 0xFF
        out += line
        prev = line

    return width, height, bytes(out)


for size, name in sizes:
    path = ICONS / name
    width, height, rgba = png_pixels(path, size)

    # BGRA rows (bottom-up for DIB).
    bgra = b""
    for y in range(height - 1, -1, -1):
        row_start = y * width * 4
        row = bytearray()
        for x in range(width):
            i = row_start + x * 4
            r, g, b, a = rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]
            row += bytes((b, g, r, a))
        bgra += bytes(row)

    # BITMAPINFOHEADER (40 bytes) + AND mask (1 bpp, rows padded to 4 bytes).
    bih = struct.pack("<IiiHHIIiiII", 40, width, height * 2, 1, 32, 0, 0, 0, 0, 0, 0)
    # AND mask: fully opaque → all zeros.
    and_stride = ((width + 31) // 32) * 4
    and_mask = b"\x00" * (and_stride * height)

    framedata = bih + bgra + and_mask
    frames.append((size, framedata))

# ICONDIR + ICONDIRENTRY (16 bytes each).
header = struct.pack("<HHH", 0, 1, len(frames))
offset = 6 + 16 * len(frames)
entries = b""
images = b""
for size, framedata in frames:
    b = size if size < 256 else 0
    entries += struct.pack("<BBBBHHII", b, b, 0, 0, 1, 32, len(framedata), offset)
    offset += len(framedata)
    images += framedata

out = ICONS / "icon.ico"
out.write_bytes(header + entries + images)
print(f"wrote {out} ({out.stat().st_size} bytes, {len(frames)} frames)")
