// Shared by fixtures.ts and fuzz.ts
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import QRCode from "qrcode";
import { prepareZXingModule, readBarcodes } from "zxing-wasm/reader";
import {
  AlphanumericEncoder,
  AlphanumericMode,
  MixedEncoder,
  NumericEncoder,
  NumericMode,
} from "../typescript/src/extras/encoders.ts";
import {
  buildCanvasData,
  ByteEncoder,
  ByteMode,
  type Ecl,
  FuqrError,
  generateWithEncoder,
  type Mask,
  Module,
} from "../typescript/src/fuqr.ts";

export type Mode = "numeric" | "alphanumeric" | "byte" | "mixed";
export const MODES: Mode[] = ["numeric", "alphanumeric", "byte", "mixed"];

export type Case = {
  mode: Mode;
  minVersion: number;
  maxVersion: number;
  minEcl: Ecl;
  maxEcl: Ecl;
  mask: Mask;
  content: string;
};

export type Result =
  | { error: string }
  | { version: number; ecl: Ecl; mask: Mask; matrix: Uint8Array };

export function makeEncoder(mode: Mode, content: string) {
  if (mode === "numeric") return new NumericEncoder(content);
  if (mode === "alphanumeric") return new AlphanumericEncoder(content);
  if (mode === "byte") return new ByteEncoder(content);
  return new MixedEncoder(content);
}

export function run({ mode, content, ...options }: Case): Result {
  try {
    return generateWithEncoder(makeEncoder(mode, content), options);
  } catch (err) {
    if (err instanceof FuqrError) return { error: err.code };
    throw err;
  }
}

prepareZXingModule({
  overrides: {
    wasmBinary: readFileSync(
      createRequire(import.meta.url).resolve("zxing-wasm/reader/zxing_reader.wasm"),
    ).buffer,
  },
  fireImmediately: true,
});

async function scanZxing(qr: { matrix: Uint8Array; version: number }) {
  const width = (qr.version * 4 + 17 + 2 * 4) * 3;
  const image = {
    data: buildCanvasData(qr, 4, 3),
    width,
    height: width,
    colorSpace: "srgb" as const,
  };
  const [result] = await readBarcodes(image, { formats: ["QRCode"] });
  if (result === undefined) throw new Error("zxing-wasm could not read the code");

  const extra = JSON.parse(result.extra) as { DataMask: number; Version: string; ECLevel: string };
  return {
    content: new TextDecoder("utf-8", { fatal: true }).decode(result.bytes),
    version: Number(extra.Version),
    ecl: "LMQH".indexOf(extra.ECLevel),
    mask: extra.DataMask,
  };
}

function nodeQrcode(c: Case, segments: QRCode.QRCodeSegment[] | string) {
  return QRCode.create(segments, {
    version: c.minVersion,
    errorCorrectionLevel: (["L", "M", "Q", "H"] as const)[c.minEcl],
    maskPattern: c.mask,
  });
}

// node-qrcode picks its own mixed segments, which may be longer than ours
function nodeQrcodeBitLen(c: Case) {
  const modes = { Numeric: NumericMode, Alphanumeric: AlphanumericMode, Byte: ByteMode };
  return nodeQrcode(c, c.content).segments.reduce((bits, segment) => {
    const mode = modes[segment.mode.id as keyof typeof modes];
    return bits + mode.segLen(segment.data.length, c.minVersion);
  }, 0);
}

// node-qrcode's matrix, or null if it doesn't fit
function nodeQrcodeMatrix(c: Case) {
  let segments: QRCode.QRCodeSegment[] | string;
  if (c.mode === "mixed") segments = c.content;
  else if (c.mode === "byte")
    segments = [{ mode: c.mode, data: new TextEncoder().encode(c.content) }];
  else segments = [{ mode: c.mode, data: c.content }];
  try {
    return nodeQrcode(c, segments).modules.data;
  } catch {
    return null;
  }
}

// Returns where the dark modules first differ, or "" if they match
function compareModules(width: number, ours: Uint8Array | null, theirs: Uint8Array | null) {
  if (ours == null || theirs == null) {
    if (ours == theirs) return "";
    return `fuqr fit = ${ours != null}, node-qrcode fit = ${theirs != null}`;
  }
  for (let i = 0; i < ours.length; i++) {
    if ((ours[i] & Module.ON) !== theirs[i])
      return `differs at ${i % width},${Math.floor(i / width)}`;
  }
  return "";
}

// Returns what is wrong with the result, or "" if nothing
export async function check(c: Case, result: Result) {
  // zxing can't read empty content, even from node-qrcode
  if (!("error" in result) && c.content !== "") {
    const scanned = await scanZxing(result).catch((err: Error) => err.message);
    const { version, ecl, mask } = result;
    const expected = JSON.stringify({ content: c.content, version, ecl, mask });
    if (JSON.stringify(scanned) !== expected) return `zxing scanned ${JSON.stringify(scanned)}`;
  }

  // node-qrcode rejects empty content, and only pinned details are comparable
  if (c.content === "" || c.minVersion !== c.maxVersion || c.minEcl !== c.maxEcl) return "";
  const ours = "error" in result ? null : result.matrix;
  const theirs = nodeQrcodeMatrix(c);
  const difference = compareModules(c.minVersion * 4 + 17, ours, theirs);
  if (difference === "") return "";

  // Mixed mode may differ from node-qrcode when it finds a shorter encoding
  if (c.mode === "mixed" && ours != null) {
    const bits = makeEncoder(c.mode, c.content).bitLen(c.minVersion);
    if (theirs == null || bits <= nodeQrcodeBitLen(c)) return "";
  }
  return `node-qrcode ${difference}`;
}

export const describe = (c: Case) =>
  `${c.mode}: ${c.minVersion} ${c.minEcl} ${c.mask} ${JSON.stringify(c.content)}`;

const DIGITS = [[0x30, 0x39]];
const ALPHANUMERIC = [
  ...DIGITS,
  [0x41, 0x5a], // upper case
  [0x20, 0x20], // space
  [0x24, 0x25], // $%
  [0x2a, 0x2b], // *+
  [0x2d, 0x2f], // -./
  [0x3a, 0x3a], // :
];
const BYTE = [
  ...ALPHANUMERIC,
  [0x61, 0x7a], // lower case
  [0x21, 0x23], // punctuation alphanumeric mode leaves out
  [0xa0, 0x24f], // two byte
  [0x3040, 0x9fff], // three byte
  [0x1f300, 0x1f6ff], // four byte
];
const RANGES = { numeric: DIGITS, alphanumeric: ALPHANUMERIC, byte: BYTE, mixed: BYTE };

const LOWER = "abcdefghijklmnopqrstuvwxyz";
const UPPER = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGIT = "0123456789";

export function makeRandom(seed: number) {
  // mulberry32
  const random = () => {
    seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
  const int = (min: number, max: number) => min + Math.floor(random() * (max - min + 1));
  const pick = <T>(items: T[]) => items[int(0, items.length - 1)];
  const chars = (alphabet: string, min: number, max: number) =>
    Array.from({ length: int(min, max) }, () => pick([...alphabet])).join("");

  // Drawing from a few ranges makes runs, so mixed mode has to pick switches
  const content = (mode: Mode, maxLen: number) => {
    const ranges = [0, 0, 0].map(() => pick(RANGES[mode]));
    return Array.from({ length: int(0, maxLen) }, () => {
      const [lo, hi] = pick(ranges);
      return String.fromCodePoint(int(lo, hi));
    }).join("");
  };

  const segment = () =>
    pick([
      () => chars(LOWER, 2, 12),
      () => chars(DIGIT, 1, 16),
      () => chars(UPPER + DIGIT, 4, 16),
      () => chars(LOWER + DIGIT + "-_", 4, 20),
    ])();

  const url = () => {
    let url = pick(["https://", "http://", "HTTPS://"]) + chars(LOWER, 3, 12);
    url += pick([".com", ".org", ".io", ".co.uk"]);
    for (let i = int(0, 5); i > 0; i--) url += "/" + segment();
    if (random() < 0.5) {
      const params = Array.from({ length: int(1, 3) }, () => chars(LOWER, 1, 8) + "=" + segment());
      url += "?" + params.join("&");
    }
    if (random() < 0.2) url += "#" + segment();
    return url;
  };

  return { int, content, url };
}
