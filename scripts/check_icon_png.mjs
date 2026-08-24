import sharp from "sharp";

const f = "清理.png";
const { data, info } = await sharp(f).raw().toBuffer({ resolveWithObject: true });
const ch = info.channels;
const w = info.width;
const h = info.height;
const px = (x, y) => {
  const i = (y * w + x) * ch;
  return [data[i], data[i + 1], data[i + 2], data[i + 3]];
};
console.log(`size: ${w}x${h} channels=${ch}`);
console.log(`center(${Math.floor(w/2)},${Math.floor(h/2)}):`, px(Math.floor(w/2), Math.floor(h/2)).join(","));
console.log(`upper(${Math.floor(w/2)},${Math.floor(h*0.45)}):`, px(Math.floor(w/2), Math.floor(h*0.45)).join(","));
console.log(`corner(2,2):`, px(2, 2).join(","));
