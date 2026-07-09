// A star logo dropped into the center of a QR. FittedLogoPlugin grows the
// version until the modules it clears stay inside the error correction budget.
import { decode as decodePng } from "fast-png";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { FittedLogoPlugin } from "../typescript/src/extras/FittedLogoPlugin.ts";
import { generate, Module } from "../typescript/src/fuqr.ts";
import { encodePng, toRgba } from "./helpers/png.ts";

const star = toRgba(decodePng(readFileSync(join(import.meta.dirname, "inputs/star.png"))));
const alpha = new Uint8Array(star.width * star.height);
for (let i = 0; i < alpha.length; i++) alpha[i] = star.rgba[i * 4 + 3];

const size = 0.57; // logo width as a fraction of the qr width

const { matrix, version } = generate("https://github.com/zhengkyl/fuqr", { maxVersion: 6 }, [
  new FittedLogoPlugin(
    {
      dataUrl: "",
      naturalWidth: star.width,
      naturalHeight: star.height,
      stencil: { width: star.width, height: star.height, data: alpha },
    },
    size,
    0,
  ),
]);

const stride = version * 4 + 17;
const margin = 2;
const scale = 7;
const dim = (stride + 2 * margin) * scale;

const rgba = new Uint8Array(dim * dim * 4).fill(255);

function set(x: number, y: number, r: number, g: number, b: number) {
  const i = (y * dim + x) * 4;
  rgba[i] = r;
  rgba[i + 1] = g;
  rgba[i + 2] = b;
}

for (let my = 0; my < stride; my++) {
  for (let mx = 0; mx < stride; mx++) {
    if (!(matrix[my * stride + mx] & Module.ON)) continue;
    const x0 = (mx + margin) * scale;
    const y0 = (my + margin) * scale;
    for (let dy = 0; dy < scale; dy++) {
      for (let dx = 0; dx < scale; dx++) set(x0 + dx, y0 + dy, 0, 0, 0);
    }
  }
}

// same placement the plugin reserves, then draw the star into it
const logoWidth = Math.round(size * stride);
const logoHeight = Math.round((logoWidth * star.height) / star.width);
const lpx = (Math.round((stride - logoWidth) / 2) + margin) * scale;
const lpy = (Math.round((stride - logoHeight) / 2) + margin) * scale;
const lpw = logoWidth * scale;
const lph = logoHeight * scale;
for (let py = 0; py < lph; py++) {
  for (let px = 0; px < lpw; px++) {
    const sx = Math.floor((px / lpw) * star.width);
    const sy = Math.floor((py / lph) * star.height);
    const si = (sy * star.width + sx) * 4;
    if (star.rgba[si + 3] === 0) continue; // transparent background
    set(lpx + px, lpy + py, star.rgba[si], star.rgba[si + 1], star.rgba[si + 2]);
  }
}

writeFileSync(join(import.meta.dirname, "outputs/nonsquare.png"), encodePng(dim, dim, rgba));
