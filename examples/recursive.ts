import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { PixelArtPlugin } from "../typescript/src/extras/PixelArtPlugin.ts";
import { generate, Module, renderSvg } from "../typescript/src/fuqr.ts";

const margin = 2;
const pad = 8;

function recursive() {
  const prev = generate("INSIDE");
  const prevStride = prev.version * 4 + 17;
  const prevWidth = prevStride + 2 * margin;

  const width = prevWidth + 2 * pad;
  const stencil = new Uint8Array(width * width);

  for (let y = 0; y < prevWidth; y++) {
    for (let x = 0; x < prevWidth; x++) {
      // force the previous code and its quiet zone, at full weight
      const inQr = x >= margin && x < prevWidth - margin && y >= margin && y < prevWidth - margin;
      const bit = inQr ? prev.matrix[(y - margin) * prevStride + (x - margin)] & Module.ON : 0;
      stencil[(y + pad) * width + (x + pad)] = (127 << 1) | bit;
    }
  }

  return renderSvg(generate("OUTSIDE", {}, [new PixelArtPlugin(stencil)]));
}

writeFileSync(join(import.meta.dirname, "outputs/recursive.svg"), recursive());
