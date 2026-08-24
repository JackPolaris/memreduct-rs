#!/usr/bin/env node
// Render `清理.svg` with resvg (handles the arc paths that libvips/sharp
// renders blank) and generate the app icon PNGs + validate centre color.
import { Resvg } from "@resvg/resvg-js";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.join(__dirname, "..");
const SRC = path.join(ROOT, "清理.svg");
const OUT = path.join(ROOT, "src-tauri", "icons");

const svg = fs.readFileSync(SRC);
if (!svg) {
  console.error("missing 清理.svg");
  process.exit(1);
}

const sizes = [
  ["32x32.png", 32],
  ["128x128.png", 128],
  ["128x128@2x.png", 256],
  ["256x256.png", 256],
  ["icon.png", 512],
];

for (const [name, size] of sizes) {
  const resvg = new Resvg(svg, { fitTo: { mode: "width", value: size } });
  const png = resvg.render().asPng();
  const outPath = path.join(OUT, name);
  fs.writeFileSync(outPath, png);
  console.log(`wrote ${name} (${png.length} bytes)`);
}

console.log("done");
