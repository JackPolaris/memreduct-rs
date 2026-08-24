#!/usr/bin/env node
// Validate that the resvg-rendered icon is the brand purple, not blank/green.
import sharp from "sharp";

const files = ["src-tauri/icons/32x32.png", "src-tauri/icons/128x128.png"];

for (const f of files) {
  const { data, info } = await sharp(f).raw().toBuffer({ resolveWithObject: true });
  const ch = info.channels;
  const w = info.width;
  const h = info.height;
  const px = (x, y) => {
    const i = (y * w + x) * ch;
    return [data[i], data[i + 1], data[i + 2], data[i + 3]];
  };
  const center = px(Math.floor(w / 2), Math.floor(h / 2));
  const upper = px(Math.floor(w / 2), Math.floor(h * 0.45));
  console.log(`${f}: center=${center.join(",")} upper=${upper.join(",")}`);
}

console.log("expected purple center or white pattern, must NOT be 0,0,0,0 blank");
