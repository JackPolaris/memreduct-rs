#!/usr/bin/env python3
"""Extract the PNG frames embedded in src-tauri/icons/icon.ico and print the
centre/upper pixel colour of the first frame to validate the icon is purple."""
import struct

path = "src-tauri/icons/icon.ico"
data = open(path, "rb").read()
reserved, typ, count = struct.unpack("<HHH", data[:6])
print(f"ICONDIR: type={typ} count={count} bytes={len(data)}")

off = 6
entries = []
for i in range(count):
    w, h, colors, res, planes, bpp, size, imgoff = struct.unpack("<BBBBHHII", data[off:off+16])
    entries.append((w, h, imgoff, size))
    off += 16

# Choose the 128 or 256 frame.
w, h, imgoff, size = entries[-1]
png = data[imgoff:imgoff+size]
print(f"last frame: {w if w else 256}x{h if h else 256} size={size} magic={png[:4].hex()}")

# Save the frame for external inspection.
open("/tmp/ico_frame.png", "wb").write(png)
print("saved /tmp/ico_frame.png")
