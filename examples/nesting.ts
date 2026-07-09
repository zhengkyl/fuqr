import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildSvgPath, generate, Module } from "../typescript/src/fuqr.ts";

const margin = 2;

function nesting(): string {
  const near = generate("NEAR");
  const far = generate("FAR");
  const stride = near.version * 4 + 17;

  const scale = 9;
  const unit = 3;
  const gap = (scale - unit) / 2;

  let innerBlack = "";
  let innerWhite = "";
  for (let y = 0; y < stride; y++) {
    for (let x = 0; x < stride; x++) {
      const outerOn = far.matrix[y * stride + x] & Module.ON;
      const innerOn = near.matrix[y * stride + x] & Module.ON;
      if (outerOn === innerOn) continue;

      const d = `M${x * scale + gap} ${y * scale + gap}h${unit}v${unit}h-${unit}z`;
      if (innerOn) innerBlack += d;
      else innerWhite += d;
    }
  }

  const side = (stride + 2 * margin) * scale;
  const origin = -margin * scale;
  return [
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${origin} ${origin} ${side} ${side}" width="300" height="300">`,
    `<rect x="${origin}" y="${origin}" width="${side}" height="${side}" fill="#fff"/>`,
    `<path d="${buildSvgPath(far, 0, scale)}"/>`,
    `<path d="${innerWhite}" fill="#fff"/>`,
    `<path d="${innerBlack}"/>`,
    `</svg>`,
  ].join("\n");
}

writeFileSync(join(import.meta.dirname, "outputs/nesting.svg"), nesting());
