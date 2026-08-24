import sharp from "sharp";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.join(__dirname, "..");
const SRC = path.join(ROOT, "清理.png");
const OUT = path.join(ROOT, "src-tauri", "icons");

const sizes = [
  ["16x16.png", 16],
  ["32x32.png", 32],
  ["64x64.png", 64],
  ["128x128.png", 128],
  ["128x128@2x.png", 256],
  ["256x256.png", 256],
  ["icon.png", 512],
];

for (const [name, size] of sizes) {
  await sharp(SRC).resize(size, size).png().toFile(path.join(OUT, name));
  console.log(`wrote ${name}`);
}

console.log("done");
