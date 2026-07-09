// A version 6 code turned on its side with a wordmark banner across it. The
// banner is 7 modules tall; its width follows the image aspect and overhangs.
import { decode as decodePng } from "fast-png";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { FittedLogoPlugin } from "../typescript/src/extras/FittedLogoPlugin.ts";
import { generate, Module } from "../typescript/src/fuqr.ts";
import { encodePng, toRgba } from "./helpers/png.ts";

const wordmark = toRgba(decodePng(readFileSync(join(import.meta.dirname, "inputs/wordmark.png"))));

const stride = 6 * 4 + 17; // version 6
const barH = 7; // banner height in modules, after the 90 degree turn
const barW = Math.round((barH * wordmark.width) / wordmark.height); // width from the aspect

// Unrotated the logo is tall, so the stencil is the banner turned 90 degrees
// ccw to clear the right modules under the rotated code.
const naturalWidth = wordmark.height;
const naturalHeight = wordmark.width;
const alpha = new Uint8Array(naturalWidth * naturalHeight);
for (let sy = 0; sy < naturalHeight; sy++) {
  for (let sx = 0; sx < naturalWidth; sx++) {
    alpha[sy * naturalWidth + sx] =
      wordmark.rgba[(sx * wordmark.width + (wordmark.width - 1 - sy)) * 4 + 3];
  }
}

const size = barH / stride;
const { matrix } = generate(
  "https://kylezhe.ng/writes/crafting-qr-codes",
  {
    minVersion: 6,
    maxVersion: 6,
  },
  [
    new FittedLogoPlugin(
      {
        dataUrl: "",
        naturalWidth,
        naturalHeight,
        stencil: { width: naturalWidth, height: naturalHeight, data: alpha },
      },
      size,
      1,
      1,
    ),
  ],
);

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

// draw the matrix rotated 90 degrees clockwise
for (let my = 0; my < stride; my++) {
  for (let mx = 0; mx < stride; mx++) {
    if (!(matrix[my * stride + mx] & Module.ON)) continue;
    const x0 = (stride - 1 - my + margin) * scale;
    const y0 = (mx + margin) * scale;
    for (let dy = 0; dy < scale; dy++) {
      for (let dx = 0; dx < scale; dx++) set(x0 + dx, y0 + dy, 0, 0, 0);
    }
  }
}

// draw the banner upright, centered, overhanging the sides
const bpw = barW * scale;
const bph = barH * scale;
const bpx = (Math.round((stride - barW) / 2) + margin) * scale;
const bpy = (Math.round((stride - barH) / 2) + margin) * scale;
for (let py = 0; py < bph; py++) {
  for (let px = 0; px < bpw; px++) {
    const sx = Math.floor((px / bpw) * wordmark.width);
    const sy = Math.floor((py / bph) * wordmark.height);
    const si = (sy * wordmark.width + sx) * 4;
    if (wordmark.rgba[si + 3] === 0) continue; // transparent background
    set(bpx + px, bpy + py, wordmark.rgba[si], wordmark.rgba[si + 1], wordmark.rgba[si + 2]);
  }
}

writeFileSync(join(import.meta.dirname, "outputs/wordmark.png"), encodePng(dim, dim, rgba));
