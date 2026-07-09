import { decode as decodePng } from "fast-png";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { PixelArtPlugin } from "../typescript/src/extras/PixelArtPlugin.ts";
import { buildMatrix, ByteEncoder, Module } from "../typescript/src/fuqr.ts";
import { encodeGif } from "./helpers/gif.ts";
import { toAlpha } from "./helpers/png.ts";

const version = 6;
const qrWidth = version * 4 + 17;
const imgWidth = qrWidth;
const imgHeight = ((imgWidth * 3) / 4) | 1; // 4:3 frames, forced odd
const padTop = (qrWidth - imgHeight) / 2 - 1;
const numFrames = 11;

function horseStencil(frame: number): Uint8Array {
  const file = readFileSync(
    join(import.meta.dirname, `inputs/horse/horse_${String(frame).padStart(2, "0")}.png`),
  );
  const png = toAlpha(decodePng(file));

  // nearest neighbor resize to the qr image area
  const img = new Uint8Array(imgWidth * imgHeight);
  for (let y = 0; y < imgHeight; y++) {
    for (let x = 0; x < imgWidth; x++) {
      const sx = Math.floor(((x + 0.5) * png.width) / imgWidth);
      const sy = Math.floor(((y + 0.5) * png.height) / imgHeight);
      img[y * imgWidth + x] = png.alpha[sy * png.width + sx];
    }
  }

  // horse pixels are forced on, pixels touching the horse are forced off
  // so the silhouette keeps a white outline, everything else is free
  const stencil = new Uint8Array(qrWidth * qrWidth);
  for (let y = 0; y < imgHeight; y++) {
    for (let x = 0; x < imgWidth; x++) {
      let value = 0;
      if (img[y * imgWidth + x] > 127) {
        value = (127 << 1) | 1;
      } else {
        neighbors: for (let dy = -1; dy <= 1; dy++) {
          for (let dx = -1; dx <= 1; dx++) {
            const nx = x + dx;
            const ny = y + dy;
            if (nx < 0 || nx >= imgWidth || ny < 0 || ny >= imgHeight) continue;
            if (img[ny * imgWidth + nx] > 127) {
              value = 127 << 1;
              break neighbors;
            }
          }
        }
      }
      if (value === 0) continue;
      // 90 degree ccw rotation, the horse lies sideways in the qr code
      stencil[(qrWidth - 1 - x) * qrWidth + (y + padTop)] = value;
    }
  }
  return stencil;
}

const scale = 7;
const margin = 2;
const outWidth = (qrWidth + 2 * margin) * scale;

// One fixer reused across frames so its schedule spreads ec churn over time.
const fixer = new PixelArtPlugin(horseStencil(0));
const frames: Uint8Array[] = [];
for (let i = 0; i < numFrames; i++) {
  fixer.weightedStencil = horseStencil(i);
  const { matrix } = buildMatrix(
    new ByteEncoder("https://github.com/zhengkyl/fuqr"),
    { version, ecl: 0, mask: 0 },
    [fixer],
  );

  const frame = new Uint8Array(outWidth * outWidth); // 0 = white
  for (let my = 0; my < qrWidth; my++) {
    for (let mx = 0; mx < qrWidth; mx++) {
      if (!(matrix[my * qrWidth + mx] & Module.ON)) continue;
      // undo the rotation with a 90 degree cw rotation
      const sx = (qrWidth - 1 - my + margin) * scale;
      const sy = (mx + margin) * scale;
      for (let dy = 0; dy < scale; dy++) {
        frame.fill(1, (sy + dy) * outWidth + sx, (sy + dy) * outWidth + sx + scale);
      }
    }
  }
  frames.push(frame);
}

const gif = encodeGif(
  outWidth,
  outWidth,
  [
    [255, 255, 255],
    [0, 0, 0],
  ],
  frames,
  20,
);
writeFileSync(join(import.meta.dirname, "outputs/horse.gif"), gif);
