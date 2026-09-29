// Runs test.ts suites against the adapter command in argv. See adapter.ts/adapter.rs
import { Resvg } from "@resvg/resvg-js";
import { spawn } from "node:child_process";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { createInterface } from "node:readline";
import QRCode from "qrcode";
import { prepareZXingModule, readBarcodes } from "zxing-wasm/reader";

// node-qrcode's internal modules have no types, and require gives any
const require = createRequire(import.meta.url);
const ecCodes = require("qrcode/lib/core/error-correction-code.js");
const ecLevels = require("qrcode/lib/core/error-correction-level.js");
const utils = require("qrcode/lib/core/utils.js");

const ECLS = ["L", "M", "Q", "H"] as const;
export const DATA_CODEWORDS: number[][] = Array.from({ length: 41 }, (_, i) =>
  ECLS.map(
    (ecl) => utils.getSymbolTotalCodewords(i) - ecCodes.getTotalCodewordsCount(i, ecLevels[ecl]),
  ),
);

const SEGMENT_CLASSES = ["numeric", "alphanumeric", "byte"].map((mode) =>
  require(`qrcode/lib/core/${mode}-data.js`),
);

// fromArray drops empty segments, so use built segments as is
const Segments = require("qrcode/lib/core/segments.js");
Segments.fromArray = (built: unknown) => built;

// FNV-1a 64 as two 32 bit halves. Prime is 2^40 + 0x1b3.
function fnv1a64(bytes: Uint8Array) {
  let hi = 0xcbf29ce4;
  let lo = 0x84222325;
  for (const byte of bytes) {
    lo = (lo ^ byte) >>> 0;
    const low = lo * 0x1b3;
    hi = (Math.imul(hi, 0x1b3) + Math.floor(low / 2 ** 32) + (lo << 8)) >>> 0;
    lo = low >>> 0;
  }
  return hi.toString(16).padStart(8, "0") + lo.toString(16).padStart(8, "0");
}

// ---- Independent spec reference for encoders ----

const NUMERIC = 0;
const ALPHANUMERIC = 1;
export const BYTE = 2;
export type Mode = 0 | 1 | 2;
type Segment = { mode: Mode; len: number };

export const ENCODERS = ["numeric", "alphanumeric", "byte", "mixed"] as const;
export type Encoder = (typeof ENCODERS)[number];

const MODE_OF = Array.from({ length: 256 }, (_, byte): Mode => {
  if (0x30 <= byte && byte <= 0x39) return NUMERIC;
  const b45 = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";
  if (b45.includes(String.fromCharCode(byte))) return ALPHANUMERIC;
  return BYTE;
});

export const group = (version: number) => (version <= 9 ? 0 : version <= 26 ? 1 : 2);

// ISO 18004 Table 3, by mode then version group
const CCI_LENS = [
  [10, 12, 14],
  [9, 11, 13],
  [8, 16, 16],
];
export function segmentBits(mode: Mode, len: number, version: number) {
  const header = 4 + CCI_LENS[mode][group(version)];
  if (mode === NUMERIC) return header + 10 * Math.floor(len / 3) + [0, 4, 7][len % 3];
  if (mode === ALPHANUMERIC) return header + 11 * Math.floor(len / 2) + 6 * (len % 2);
  return header + 8 * len;
}

type Best = { bits: number; count: number; start: number; mode: Mode };

// Best segmentation of every prefix of bytes
// Fewest bits win, then fewest segments, then the longest last segment.
export function prefixes(bytes: Uint8Array, version: number) {
  const best: Best[] = [{ bits: 0, count: 0, start: 0, mode: BYTE }];
  let byteStart = 0;
  for (let end = 1; end <= bytes.length; end++) {
    let b: Best = { bits: Infinity, count: Infinity, start: Infinity, mode: BYTE };
    const consider = (start: number, mode: Mode) => {
      const bits = best[start].bits + segmentBits(mode, end - start, version);
      const count = best[start].count + 1;
      if (
        bits < b.bits ||
        (bits === b.bits && (count < b.count || (count === b.count && start < b.start)))
      ) {
        b = { bits, count, start, mode };
      }
    };
    for (const mode of [NUMERIC, ALPHANUMERIC] as Mode[]) {
      for (let start = end - 1; start >= 0 && MODE_OF[bytes[start]] <= mode; start--) {
        consider(start, mode);
      }
    }
    // Byte segments cost 8 bits per byte, so every end shares the best start
    // by fewest bits - 8 * start, then fewest segments, then earliest start
    const key = (start: number) => best[start].bits - 8 * start;
    const last = end - 1;
    if (
      key(last) < key(byteStart) ||
      (key(last) === key(byteStart) && best[last].count < best[byteStart].count)
    ) {
      byteStart = last;
    }
    consider(byteStart, BYTE);
    best.push(b);
  }
  return best;
}

// Segments an encoder must emit, or null if it rejects the content.
// Empty content is one empty segment, in byte mode for the mixed encoder.
function expectedSegments(encoder: Encoder, bytes: Uint8Array, version: number): Segment[] | null {
  if (encoder !== "mixed") {
    const mode = ENCODERS.indexOf(encoder) as Mode;
    if (bytes.some((b) => MODE_OF[b] > mode)) return null;
    return [{ mode, len: bytes.length }];
  }
  if (bytes.length === 0) return [{ mode: BYTE, len: 0 }];
  const best = prefixes(bytes, version);
  const segments: Segment[] = [];
  for (let end = bytes.length; end > 0; end = best[end].start) {
    segments.unshift({ mode: best[end].mode, len: end - best[end].start });
  }
  return segments;
}

// ---- Expected answers, from the reference and node-qrcode ----

// [encoder, minVersion, maxVersion, minEcl, maxEcl, mask, content]
type Case = [Encoder, number, number, number, number, number, string];
type Qr = { modules: Uint8Array; width: number; version: number; ecl: number; mask: number };
export const utf8 = (text: string) => new TextEncoder().encode(text);

function expected([encoder, minVersion, maxVersion, minEcl, maxEcl, mask, content]: Case): {
  answer: string;
  qr?: Qr;
} {
  const bytes = utf8(content);
  // Segmentation only changes with char count indicator lengths
  const cache: (Segment[] | null)[] = [];
  const segmentsAt = (v: number) => (cache[group(v)] ??= expectedSegments(encoder, bytes, v));
  if (segmentsAt(1) === null) return { answer: "ENCODING_FAILURE" };

  // First version that fits, then the highest ecl that still fits
  const fits = (v: number, ecl: number) =>
    segmentsAt(v)!.reduce((bits, { mode, len }) => bits + segmentBits(mode, len, v), 0) <=
    8 * DATA_CODEWORDS[v][ecl];
  let version = minVersion;
  while (version <= maxVersion && !fits(version, minEcl)) version++;
  if (version > maxVersion) return { answer: "CONTENT_TOO_LONG" };
  let ecl = minEcl;
  while (ecl < maxEcl && fits(version, ecl + 1)) ecl++;

  let start = 0;
  const built = segmentsAt(version)!.map(({ mode, len }) => {
    const data = bytes.subarray(start, (start += len));
    return new SEGMENT_CLASSES[mode](mode === BYTE ? data : String.fromCharCode(...data));
  });
  const qr = QRCode.create(built, {
    version,
    errorCorrectionLevel: ECLS[ecl],
    maskPattern: mask as QRCode.QRCodeMaskPattern,
  });
  const { data: modules, size: width } = qr.modules;
  const answer = `${version}-${ecl}-${mask}-${fnv1a64(modules)}`;
  return { answer, qr: { modules, width, version, ecl, mask } };
}

prepareZXingModule({
  overrides: {
    wasmBinary: readFileSync(new URL(import.meta.resolve("zxing-wasm/reader/zxing_reader.wasm")))
      .buffer,
  },
  fireImmediately: true,
});

// Throws unless zxing decodes the matrix from expected() back to its inputs.
async function assertScans(content: string, qr: Qr | undefined) {
  // zxing can't read empty content
  if (qr === undefined || content === "") return;
  const { modules, width } = qr;
  const scale = 3;
  const size = (width + 8) * scale;
  const data = new Uint8ClampedArray(size * size * 4).fill(255);
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const mx = Math.floor(x / scale) - 4;
      const my = Math.floor(y / scale) - 4;
      if (mx < 0 || my < 0 || mx >= width || my >= width || !modules[my * width + mx]) continue;
      data.fill(0, (y * size + x) * 4, (y * size + x) * 4 + 3);
    }
  }
  const image = { data, width: size, height: size, colorSpace: "srgb" as const };
  const [result] = await readBarcodes(image, { formats: ["QRCode"] });
  if (result === undefined) throw new Error(`reference is wrong for ${JSON.stringify(content)}`);

  const extra = JSON.parse(result.extra) as { DataMask: number; Version: string; ECLevel: string };
  const scanned = {
    bytes: [...result.bytes],
    version: Number(extra.Version),
    ecl: ECLS.indexOf(extra.ECLevel as (typeof ECLS)[number]),
    mask: extra.DataMask,
  };
  const want = { bytes: [...utf8(content)], version: qr.version, ecl: qr.ecl, mask: qr.mask };
  if (JSON.stringify(scanned) !== JSON.stringify(want)) {
    throw new Error(`reference is wrong for ${JSON.stringify(content)}`);
  }
}

// ---- SVG rendering ----

// Whether svg renders exactly the dark modules, scaled up
function svgMatches({ modules, width }: Qr, margin: number, scale: number, svg: string) {
  const size = (width + 2 * margin) * scale;
  const rgba = new Resvg(svg).render().pixels;
  if (rgba.length !== size * size * 4) return false;
  for (let i = 0; i < size * size; i++) {
    const x = Math.floor((i % size) / scale) - margin;
    const y = Math.floor(i / size / scale) - margin;
    const dark = x >= 0 && y >= 0 && x < width && y < width && modules[y * width + x] === 1;
    if (rgba[i * 4] < 128 !== dark) return false;
  }
  return true;
}

// ---- Adapter process ----

const command = process.argv.slice(2);
if (command[0] === "--") command.shift();
const child = spawn(command[0], command.slice(1), { stdio: ["pipe", "pipe", "inherit"] });
const waiting: ((answer: string) => void)[] = [];
createInterface({ input: child.stdout }).on("line", (line) => {
  waiting.shift()!(JSON.parse(line) as string);
});
child.on("exit", (code) => {
  if (waiting.length === 0) return;
  console.error(`adapter exited with ${code}`);
  process.exit(1);
});

// svg is [margin, size, attributes] to also get the port's svg
function ask(request: Case, svg: [number, string | null, string] | null) {
  return new Promise<string>((resolve) => {
    waiting.push(resolve);
    child.stdin.write(JSON.stringify([...request, svg]) + "\n");
  });
}

// ---- Running cases ----

// [margin, scale, attributes]
const SVG_OPTIONS: [number, number | null, string][] = [
  [0, 1, ""],
  [2, null, 'class="qr"'],
  [1, 2, ""],
  [4, 3, ""],
];

// scan checks expected matrices with zxing.
// svg checks the rendered svg
export async function run(
  name: string,
  { scan = false, svg = false },
  cases: () => Iterable<Case>,
) {
  const failures: string[] = [];
  const pending: Promise<void>[] = [];
  let count = 0;
  let rendered = 0;
  for (const request of cases()) {
    count++;
    const want = expected(request);
    if (scan) await assertScans(request[6], want.qr);
    let options: [number, string | null, string] | null = null;
    let check = (answer: string) => answer === want.answer;
    const { qr } = want;
    if (svg && qr !== undefined) {
      const [margin, scale, attributes] = SVG_OPTIONS[rendered++ % SVG_OPTIONS.length];
      const size = scale === null ? null : String((qr.width + 2 * margin) * scale);
      options = [margin, size, attributes];
      check = (answer) => {
        const space = answer.indexOf(" ");
        const image = answer.slice(space + 1);
        return answer.slice(0, space) === want.answer && svgMatches(qr, margin, scale ?? 1, image);
      };
    }
    pending.push(
      ask(request, options).then((answer) => {
        if (check(answer)) return;
        // Long content is shortened, since FUZZ_SEED reproduces it in full
        const chars = [...request[6]];
        const content = chars.length > 80 ? chars.slice(0, 80).join("") + "…" : request[6];
        failures.push(`  ${JSON.stringify([...request.slice(0, 6), content])}`);
      }),
    );
    // Bounds memory while keeping the pipe full
    if (pending.length >= 1000) await Promise.all(pending.splice(0));
  }
  await Promise.all(pending);

  if (failures.length === 0) {
    console.log(`${name}: ${count} ok`);
    return;
  }
  process.exitCode = 1;
  console.log(`${name}: ${failures.length} of ${count} failed`);
  console.log(failures.slice(0, 5).join("\n"));
}

export const close = () => child.stdin.end();
