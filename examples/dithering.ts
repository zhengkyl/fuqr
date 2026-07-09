import { decode as decodePng } from "fast-png";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { generate, Module } from "../typescript/src/fuqr.ts";
import { encodePng, toGray } from "./helpers/png.ts";

const { matrix, version } = generate("https://github.com/zhengkyl/fuqr", {
  minVersion: 3,
  maxVersion: 3,
});
const stride = version * 4 + 17;
const margin = 2;

const scale = 9;
const size = (stride + 2 * margin) * scale;

const rgba = new Uint8Array(size * size * 4).fill(255);

function set(x: number, y: number, r: number, g: number, b: number) {
  const i = (y * size + x) * 4;
  rgba[i] = r;
  rgba[i + 1] = g;
  rgba[i + 2] = b;
}

function setRect(
  x: number,
  y: number,
  width: number,
  height: number,
  r: number,
  g: number,
  b: number,
) {
  for (let j = y; j < y + height; j++) {
    for (let i = x; i < x + width; i++) {
      set(i, j, r, g, b);
    }
  }
}

// Layer 1: the spiral downsampled so each pixel is one qr dot wide, then
// ordered dithered. dither pulls pure black/white toward mid gray before the
// bayer compare, so even flat regions always land on a pattern. 0 is a hard
// threshold, 1 is a 50% checkerboard everywhere.
const dither = 0.3;
const bayer = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];
const spiral = toGray(decodePng(readFileSync(join(import.meta.dirname, "inputs/spiral.png"))));

const cell = scale / 3;
const grid = stride * cell; // 99x99, one cell per dot
for (let gy = 0; gy < grid; gy++) {
  for (let gx = 0; gx < grid; gx++) {
    // average source darkness over the cell, then dither against a threshold
    const x0 = Math.floor((gx * spiral.width) / grid);
    const x1 = Math.floor(((gx + 1) * spiral.width) / grid);
    const y0 = Math.floor((gy * spiral.height) / grid);
    const y1 = Math.floor(((gy + 1) * spiral.height) / grid);
    let sum = 0;
    for (let sy = y0; sy < y1; sy++) {
      for (let sx = x0; sx < x1; sx++) {
        sum += 1 - spiral.gray[sy * spiral.width + sx];
      }
    }
    const dark = sum / ((x1 - x0) * (y1 - y0));
    const level = 0.5 + (dark - 0.5) * (1 - dither);
    const threshold = (bayer[(gy % 4) * 4 + (gx % 4)] + 0.5) / 16;
    if (level <= threshold) continue;

    for (let dy = 0; dy < cell; dy++) {
      for (let dx = 0; dx < cell; dx++) {
        set(gx * cell + dx + margin * scale, gy * cell + dy + margin * scale, 0, 0, 0);
      }
    }
  }
}

const inset = (scale - cell) / 2;

for (let my = 0; my < stride; my++) {
  for (let mx = 0; mx < stride; mx++) {
    const module = matrix[my * stride + mx];

    const x0 = (mx + margin) * scale;
    const y0 = (my + margin) * scale;

    if (module === 0) {
      setRect(x0, y0, scale, scale, 255, 255, 255);
      continue;
    }
    if (module & Module.FINDER || module & Module.ALIGNMENT) {
      if (module & Module.ON) setRect(x0, y0, scale, scale, 0, 0, 0);
      else setRect(x0, y0, scale, scale, 255, 255, 255);
      continue;
    }

    for (let dy = 0; dy < cell; dy++) {
      for (let dx = 0; dx < cell; dx++) {
        if (module & Module.ON) set(inset + x0 + dx, inset + y0 + dy, 0, 0, 0);
        else set(inset + x0 + dx, inset + y0 + dy, 255, 255, 255);
      }
    }
  }
}

writeFileSync(join(import.meta.dirname, "outputs/dithering.png"), encodePng(size, size, rgba));
