// Reads a code back two ways: by hand, and with a real scanner. Rendering goes
// through the library's own renderer, so the scanner sees what a camera would.
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { expect } from "vitest";
import { prepareZXingModule, readBarcodes } from "zxing-wasm/reader";
import { buildCanvasData, type Ecl, type Mask } from "../typescript/src/fuqr.ts";
import { examine, type Qr } from "./examine.ts";

prepareZXingModule({
  overrides: {
    wasmBinary: readFileSync(
      createRequire(import.meta.url).resolve("zxing-wasm/reader/zxing_reader.wasm"),
    ).buffer,
  },
  fireImmediately: true,
});

const SCALE = 3;
const MARGIN = 4;

function render(qr: Qr) {
  const width = (qr.version * 4 + 17 + 2 * MARGIN) * SCALE;
  const data = buildCanvasData(qr, MARGIN, SCALE);
  if (data.length !== width * width * 4) {
    throw new Error(`rendered ${data.length} bytes, expected ${width * width * 4}`);
  }
  return { data, width, height: width, colorSpace: "srgb" as const };
}

// zxing-cpp, the reference implementation. It reports the version, ecl and mask
// it read, so nothing here has to reach for the library's own tables.
async function zxing(qr: Qr) {
  const [result] = await readBarcodes(render(qr), { formats: ["QRCode"] });
  if (result === undefined) throw new Error("zxing-wasm could not read the code");

  const extra = JSON.parse(result.extra) as { DataMask: number; Version: string; ECLevel: string };
  return {
    // Byte segments carry utf8 without an eci header, which zxing reports as is.
    content: new TextDecoder("utf-8", { fatal: true }).decode(result.bytes),
    version: Number(extra.Version),
    ecl: "LMQH".indexOf(extra.ECLevel) as Ecl,
    mask: extra.DataMask as Mask,
  };
}

export async function scan(qr: Qr & { ecl: Ecl; mask: Mask }, content: string) {
  const details = { version: qr.version, ecl: qr.ecl, mask: qr.mask };

  expect(examine(qr), "manual").toMatchObject({ ...details, content });

  // zxing-cpp reports no result at all for an empty symbol.
  if (content !== "") expect(await zxing(qr), "zxing-wasm").toEqual({ ...details, content });
}
