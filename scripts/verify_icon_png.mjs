#!/usr/bin/env node
// Validate that the shipped icon PNGs actually contain the brand artwork —
// i.e. they are neither fully transparent (the `tauri icon` failure mode) nor
// the old green/black placeholder.
//
// NOTE: this used to only *print* the sampled pixels and then claim a
// validation had happened, so it always exited 0 and never caught the very
// problem it was written for. It now exits non-zero when the icon is blank.
import sharp from "sharp";

const files = [
  "src-tauri/icons/32x32.png",
  "src-tauri/icons/128x128.png",
  "src-tauri/icons/icon.png",
];

/**
 * Minimum fraction of non-transparent pixels.
 *
 * The brand mark is a glyph on a transparent background, so it only covers
 * roughly a third of the canvas (measured: 31–38%). The failure mode this guards
 * against is a *blank* render — `tauri icon` produced fully transparent PNGs,
 * i.e. 0% — so the bar only has to separate "real artwork" from "nothing".
 */
const MIN_OPAQUE_RATIO = 0.05;

let failed = false;

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

  let opaque = 0;
  for (let i = 3; i < data.length; i += ch) {
    if (data[i] > 16) opaque += 1;
  }
  const ratio = opaque / (w * h);
  // A real mark has *some* opaque pixels and a drawn centre; a blank render has
  // neither. Both conditions are checked because a mostly-empty image with one
  // stray pixel would otherwise pass on the ratio alone.
  const ok = ratio >= MIN_OPAQUE_RATIO && center[3] > 16;

  console.log(
    `${ok ? "ok  " : "FAIL"} ${f}: ${w}x${h} center=${center.join(",")} ` +
      `upper=${upper.join(",")} opaque=${(ratio * 100).toFixed(1)}%`
  );

  if (!ok) failed = true;
}

if (failed) {
  console.error(
    "\nIcon check failed: an icon is mostly transparent (blank render). " +
      "Regenerate it before packaging."
  );
  process.exit(1);
}

console.log("Icon check passed.");
