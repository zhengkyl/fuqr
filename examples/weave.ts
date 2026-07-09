import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { generate, Module } from "../typescript/src/fuqr.ts";

const { matrix, version } = generate("https://github.com/zhengkyl/fuqr");
const stride = version * 4 + 17;
const margin = 2;
const scale = 11;

// gap is in sub-units (1..unit)
function weave(blackGap: number, whiteGap: number): string {
  const rect = (x: number, y: number, w: number, h: number) => `M${x} ${y}h${w}v${h}h-${w}z`;

  const lo = -margin;
  const hi = stride + 2 * margin;
  const side = (stride + 2 * margin) * scale;
  const origin = lo * scale;
  const co = scale - 2 * blackGap; // thread thickness

  let bot = "";
  for (let y = lo; y < hi; y++) bot += rect(origin, y * scale + blackGap, side, co);

  const cco = scale - 2 * whiteGap;
  let mid = "";
  for (let x = lo; x < hi; x++) mid += rect(x * scale + whiteGap, origin, cco, side);

  // Finders have no gap, so they cover their cell solid and break the run.
  let top = "";
  for (let y = lo; y < hi; y++) {
    let run = 0;
    const flush = (end: number) => {
      if (run > 0) top += rect((end - run) * scale, y * scale + blackGap, run * scale, co);
      run = 0;
    };
    for (let x = lo; x < hi; x++) {
      const inQr = x >= 0 && x < stride && y >= 0 && y < stride;
      const module = inQr ? matrix[y * stride + x] : 0;
      if (module & Module.ON && !(module & Module.FINDER)) {
        run++;
      } else {
        flush(x);
      }
    }
    flush(hi);
  }

  const finder = (x: number, y: number) => {
    const seven = 7 * scale;
    const five = 5 * scale;
    const three = 3 * scale;

    mid += `M${x + scale},${y + scale}h${five}v${five}h-${five}z`;
    top += `M${x},${y}h${seven}v${seven}h-${seven}zM${x + scale},${y + scale}v${five}h${five}v-${five}zM${x + 2 * scale},${y + 2 * scale}h${three}v${three}h-${three}z`;
  };

  finder(0, 0);
  finder((stride - 7) * scale, 0);
  finder(0, (stride - 7) * scale);

  return [
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${origin} ${origin} ${side} ${side}" width="300" height="300">`,
    `<rect x="${origin}" y="${origin}" width="${side}" height="${side}" fill="#65abec"/>`,
    `<path d="${bot}" fill="#000"/>`,
    `<path d="${mid}" fill="#fff"/>`,
    `<path d="${top}" fill="#000"/>`,
    `</svg>`,
  ].join("\n");
}

writeFileSync(join(import.meta.dirname, "outputs/weave.svg"), weave(3, 3));
