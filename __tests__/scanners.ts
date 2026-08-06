import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { prepareZXingModule, readBarcodes } from "zxing-wasm/reader";
import { buildCanvasData, type Ecl, type Mask } from "../typescript/src/fuqr.ts";

export type Qr = { matrix: Uint8Array; version: number };

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

export async function scanZxing(qr: Qr) {
  const [result] = await readBarcodes(render(qr), { formats: ["QRCode"] });
  if (result === undefined) throw new Error("zxing-wasm could not read the code");

  const extra = JSON.parse(result.extra) as { DataMask: number; Version: string; ECLevel: string };
  return {
    content: new TextDecoder("utf-8", { fatal: true }).decode(result.bytes),
    version: Number(extra.Version),
    ecl: "LMQH".indexOf(extra.ECLevel) as Ecl,
    mask: extra.DataMask as Mask,
  };
}
