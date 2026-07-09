import { encode, type DecodedPng } from "fast-png";

// Expand any fast-png result to 8 bit RGBA, row major, four bytes per pixel.
// Handles 1/2/4/8/16 bit depth, gray or rgb with optional alpha, and palettes.
export function toRgba(png: DecodedPng): { width: number; height: number; rgba: Uint8Array } {
  const { width, height, data, depth, channels, palette } = png;
  const rgba = new Uint8Array(width * height * 4);
  const max = (1 << depth) - 1;
  const scale = 255 / max;
  const perByte = 8 / depth; // sub byte depths pack this many samples per byte, msb first
  const rowBytes = Math.ceil(width / perByte);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const p = y * width + x;
      let r: number, g: number, b: number, a: number;
      if (depth < 8) {
        const byte = data[y * rowBytes + ((x / perByte) | 0)];
        const s = (byte >> ((perByte - 1 - (x % perByte)) * depth)) & max;
        if (palette) [r, g, b, a = 255] = palette[s]; // 4th entry present only with a tRNS chunk
        else (r = g = b = s * scale), (a = 255);
      } else if (palette) {
        [r, g, b, a = 255] = palette[data[p]];
      } else {
        const i = p * channels;
        if (channels >= 3) {
          [r, g, b] = [data[i] * scale, data[i + 1] * scale, data[i + 2] * scale];
          a = channels === 4 ? data[i + 3] * scale : 255;
        } else {
          r = g = b = data[i] * scale;
          a = channels === 2 ? data[i + 1] * scale : 255;
        }
      }
      const o = p * 4;
      rgba[o] = r;
      rgba[o + 1] = g;
      rgba[o + 2] = b;
      rgba[o + 3] = a;
    }
  }
  return { width, height, rgba };
}

// Per pixel luminance in 0..1, 1 white 0 black.
export function toGray(png: DecodedPng): { width: number; height: number; gray: Float64Array } {
  const { width, height, rgba } = toRgba(png);
  const gray = new Float64Array(width * height);
  for (let p = 0; p < gray.length; p++) {
    const i = p * 4;
    gray[p] = (0.299 * rgba[i] + 0.587 * rgba[i + 1] + 0.114 * rgba[i + 2]) / 255;
  }
  return { width, height, gray };
}

// Per pixel alpha 0..255, 255 opaque.
export function toAlpha(png: DecodedPng): { width: number; height: number; alpha: Uint8Array } {
  const { width, height, rgba } = toRgba(png);
  const alpha = new Uint8Array(width * height);
  for (let p = 0; p < alpha.length; p++) alpha[p] = rgba[p * 4 + 3];
  return { width, height, alpha };
}

// Encode row major RGBA (four bytes per pixel) as an 8 bit truecolor+alpha png.
export function encodePng(width: number, height: number, rgba: Uint8Array): Uint8Array {
  return encode({ width, height, data: rgba, channels: 4, depth: 8 });
}
