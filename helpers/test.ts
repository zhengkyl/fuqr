// Test harness. See corresponding adapter.ts/adapter.rs
//
// Usage: node helpers/test.ts <adapter command...>
// FUZZ_SEED replays a run and FUZZ_ROUNDS scales the random suites. FUZZ_FULL=1
// also runs exhaustive strings at versions 10 and 27, which is much slower.
import { Resvg } from "@resvg/resvg-js";
import { spawn } from "node:child_process";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { createInterface } from "node:readline";
import QRCode from "qrcode";
import { prepareZXingModule, readBarcodes } from "zxing-wasm/reader";

const require = createRequire(import.meta.url);

const SEED = Number(process.env.FUZZ_SEED ?? Math.floor(Math.random() * 2 ** 32));
const ROUNDS = Number(process.env.FUZZ_ROUNDS ?? 300);
const FULL = process.env.FUZZ_FULL === "1";

// [encoder, minVersion, maxVersion, minEcl, maxEcl, mask, content]
type Case = [Encoder, number, number, number, number, number, string];

const utf8 = (text: string) => new TextEncoder().encode(text);

// ---- Independent spec reference for encoders ----

const NUMERIC = 0;
const ALPHANUMERIC = 1;
const BYTE = 2;
type Mode = 0 | 1 | 2;
type Segment = { mode: Mode; len: number };

const ENCODERS = ["numeric", "alphanumeric", "byte", "mixed"] as const;
type Encoder = (typeof ENCODERS)[number];

const MODE_OF = Array.from({ length: 256 }, (_, byte): Mode => {
  if (0x30 <= byte && byte <= 0x39) return NUMERIC;
  const b45 = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";
  if (b45.includes(String.fromCharCode(byte))) return ALPHANUMERIC;
  return BYTE;
});

// ISO 18004 Table 3, by mode then version group
const CCI_LENS = [
  [10, 12, 14],
  [9, 11, 13],
  [8, 16, 16],
];

const group = (version: number) => (version <= 9 ? 0 : version <= 26 ? 1 : 2);

function segmentBits(mode: Mode, len: number, version: number) {
  const header = 4 + CCI_LENS[mode][group(version)];
  if (mode === NUMERIC) return header + 10 * Math.floor(len / 3) + [0, 4, 7][len % 3];
  if (mode === ALPHANUMERIC) return header + 11 * Math.floor(len / 2) + 6 * (len % 2);
  return header + 8 * len;
}

type Best = { bits: number; count: number; start: number; mode: Mode };

// Best segmentation of every prefix of bytes
// Fewest bits win, then fewest segments, then the longest last segment.
function prefixes(bytes: Uint8Array, version: number) {
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

const utils = require("qrcode/lib/core/utils.js") as {
  getSymbolTotalCodewords(version: number): number;
};
const ecCodes = require("qrcode/lib/core/error-correction-code.js") as {
  getTotalCodewordsCount(version: number, ecl: unknown): number;
};
const ecLevels = require("qrcode/lib/core/error-correction-level.js") as Record<string, unknown>;

const ECLS = ["L", "M", "Q", "H"] as const;
// Data codewords of each version and ecl
const CODEWORDS = Array.from({ length: 40 }, (_, i) =>
  ECLS.map(
    (ecl) =>
      utils.getSymbolTotalCodewords(i + 1) - ecCodes.getTotalCodewordsCount(i + 1, ecLevels[ecl]),
  ),
);

const SEGMENT_CLASSES = [
  "qrcode/lib/core/numeric-data.js",
  "qrcode/lib/core/alphanumeric-data.js",
  "qrcode/lib/core/byte-data.js",
].map((path) => require(path) as new (data: string | Uint8Array) => object);

// fromArray rejects empty numeric and alphanumeric data, so pass built segments
const segmentsModule = require("qrcode/lib/core/segments.js") as { fromArray: unknown };
segmentsModule.fromArray = (segments: unknown) => segments;

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

type Qr = { modules: Uint8Array; width: number; version: number; ecl: number; mask: number };

function expected([encoder, minVersion, maxVersion, minEcl, maxEcl, mask, content]: Case): {
  answer: string;
  qr?: Qr;
} {
  const bytes = utf8(content);
  // Segmentation only changes with char count indicator lengths
  const cache: (Segment[] | null)[] = [];
  const segmentsAt = (v: number) => (cache[group(v)] ??= expectedSegments(encoder, bytes, v));
  if (segmentsAt(1) === null) return { answer: "INVALID_ENCODING" };

  // First version that fits, then the highest ecl that still fits
  const fits = (v: number, ecl: number) =>
    segmentsAt(v)!.reduce((bits, { mode, len }) => bits + segmentBits(mode, len, v), 0) <=
    8 * CODEWORDS[v - 1][ecl];
  let version = minVersion;
  while (version <= maxVersion && !fits(version, minEcl)) version++;
  if (version > maxVersion) return { answer: "TEXT_TOO_LONG" };
  let ecl = minEcl;
  while (ecl < maxEcl && fits(version, ecl + 1)) ecl++;

  let start = 0;
  const built = segmentsAt(version)!.map(({ mode, len }) => {
    const data = bytes.subarray(start, (start += len));
    return new SEGMENT_CLASSES[mode](mode === BYTE ? data : String.fromCharCode(...data));
  });
  const qr = QRCode.create(built as unknown as QRCode.QRCodeSegment[], {
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
    wasmBinary: readFileSync(require.resolve("zxing-wasm/reader/zxing_reader.wasm")).buffer,
  },
  fireImmediately: true,
});

// Throws unless zxing reads an expected matrix back. Checks the reference and
// node-qrcode rather than the port, so only curated suites use it.
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

// ---- Suites ----

// [margin, scale, attributes]
const SVG_OPTIONS: [number, number | null, string][] = [
  [0, 1, ""],
  [2, null, 'class="qr"'],
  [1, 2, ""],
  [4, 3, ""],
];

// scan checks expected matrices with zxing.
// svg checks the rendered svg
async function run(name: string, { scan = false, svg = false }, cases: () => Iterable<Case>) {
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

// Best bits of each code point prefix of chars, per char count indicator group
function prefixBits(encoder: Encoder, chars: string[]) {
  const bytes = utf8(chars.join(""));
  const ends = [0];
  for (const c of chars) ends.push(ends.at(-1)! + utf8(c).length);

  return [1, 10, 27].map((version) => {
    if (encoder !== "mixed") {
      const mode = ENCODERS.indexOf(encoder) as Mode;
      return ends.map((end) => segmentBits(mode, end, version));
    }
    const best = prefixes(bytes, version);
    return ends.map((end) => (end === 0 ? segmentBits(BYTE, 0, version) : best[end].bits));
  });
}

// Longest prefix of chars that fits capacity bits at version
function longest(bits: number[][], version: number, capacity: number) {
  let len = 0;
  while (bits[group(version)][len + 1] <= capacity) len++;
  return len;
}

// mulberry32
function makeRandom(seed: number) {
  const next = () => {
    seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return (t ^ (t >>> 14)) >>> 0;
  };
  return (min: number, max: number) => min + (next() % (max - min + 1));
}

// Chars by the cheapest mode they fit in
const POOLS = [[..."0123456789"], [..."ABCXYZ $%*+-./:"], [..."abcxyz!?\n~éß日本🎉"]];

// Runs of one pool, mostly short since those decide when to switch
function randomChars(random: ReturnType<typeof makeRandom>, encoder: Encoder, minBytes: number) {
  const top = encoder === "mixed" ? 2 : ENCODERS.indexOf(encoder);
  const chars: string[] = [];
  let bytes = 0;
  while (bytes < minBytes) {
    const pool = POOLS[random(0, top)];
    const len = random(0, 3) === 0 ? random(1, 60) : random(1, 12);
    for (let i = 0; i < len; i++) {
      chars.push(pool[random(0, pool.length - 1)]);
      bytes += utf8(chars.at(-1)!).length;
    }
  }
  return chars;
}

console.log(`FUZZ_SEED=${SEED} FUZZ_ROUNDS=${ROUNDS} FUZZ_FULL=${FULL ? 1 : 0}`);

await run("literals", { scan: true }, function* () {
  const literals: Record<Encoder, string[]> = {
    numeric: [
      "",
      "0",
      "42",
      "123",
      "1234",
      "12345",
      "8675309",
      "00000000000000000000",
      "12a",
      "١٢",
    ],
    alphanumeric: [
      "",
      "A",
      "AB",
      "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:",
      "HTTPS://EXAMPLE.COM/VERIFY/ABCD-EFGH-IJKL-MNOP",
      "https://example.com",
      "Hello",
    ],
    byte: [
      "",
      "a",
      "Hello, World!",
      "tab\there\nnewline",
      "日本語のテキストです",
      "🎉🚀 party 👍🏽",
      // First and last code points of each UTF-8 length, around surrogates
      "\u{7f}\u{80}\u{7ff}\u{800}\u{d7ff}\u{e000}\u{ffff}\u{10000}\u{10ffff}",
      "WIFI:T:WPA;S:MyNetwork;P:correct horse battery staple;;",
    ],
    mixed: [
      "",
      // 7 numeric + 2 alphanumeric beats 9 alphanumeric
      "0000000A0",
      // Tied bits, so fewer segments win
      "a000",
      // Tied bits and segments, so the longer last segment wins
      "0000A00a",
      "ABC123456DEF",
      "ABCDEFGH1234567890IJKLMNOP",
      "abc12345678def",
      "abcHELLOWORLD1234def",
      "https://example.com/products/12345678",
      "https://example.com/t/0123456789012345678901234567890123456789",
    ],
  };
  for (const encoder of ENCODERS) {
    for (const [i, content] of literals[encoder].entries()) {
      yield [encoder, 1, 40, 0, 3, i % 8, content];
    }
  }
});

// Longest cycled text that fits each version and ecl, then one char more
await run("boundaries", { scan: true, svg: true }, function* () {
  // Periods share no factor with block lengths, so every block differs
  const texts: Record<Encoder, string> = {
    numeric: "3141592653589793238462643383279502884197169399375105820974944",
    alphanumeric: "THE QUICK BROWN FOX JUMPS OVER 13 LAZY DOGS $%*+-./: 24680 ZYX",
    byte: "Hello, wörld! 日本語のテキスト 🎉 https://example.com/?q=fuqr&n=42\n",
    mixed: "https://example.com/2470295/MANUALS/525322511#step-3?Q=ABC%20DEF&é=日本0000A00a",
  };
  for (const encoder of ENCODERS) {
    const text = [...texts[encoder]];
    const chars = Array.from({ length: 7090 }, (_, i) => text[i % text.length]);
    const bits = prefixBits(encoder, chars);
    for (let version = 1; version <= 40; version++) {
      for (let ecl = 0; ecl < 4; ecl++) {
        const len = longest(bits, version, 8 * CODEWORDS[version - 1][ecl]);
        const mask = (version + ecl) % 8;
        for (const l of [len, len + 1]) {
          yield [encoder, version, version, ecl, 3, mask, chars.slice(0, l).join("")];
        }
      }
    }
  }
});

// All short strings, which decide when switching modes pays off
await run("exhaustive", {}, function* () {
  const strings = (alphabet: string[], maxLen: number) => {
    let layer = [""];
    const all = [...layer];
    for (let len = 1; len <= maxLen; len++) {
      layer = layer.flatMap((s) => alphabet.map((c) => s + c));
      all.push(...layer);
    }
    return all;
  };
  const contents = [...strings(["0", "A", "a"], 10), ...strings(["7", ":", " ", "b", "é"], 5)];
  for (const version of FULL ? [1, 10, 27] : [1]) {
    for (const [i, content] of contents.entries()) {
      yield ["mixed", version, version, 0, 3, i % 8, content];
    }
  }
});

// Random content with full or random option ranges
await run("random", {}, function* () {
  const random = makeRandom(SEED);
  for (const encoder of ENCODERS) {
    for (let round = 0; round < ROUNDS; round++) {
      const content = randomChars(random, encoder, random(0, 300)).join("");
      const minVersion = random(0, 1) ? 1 : random(1, 40);
      const maxVersion = random(0, 1) ? 40 : random(minVersion, 40);
      const minEcl = random(0, 3);
      const maxEcl = random(minEcl, 3);
      yield [encoder, minVersion, maxVersion, minEcl, maxEcl, random(0, 7), content];
    }
  }
});

// Longest prefix of random content that fits a random version and ecl, then
// one char more
await run("random boundaries", {}, function* () {
  const random = makeRandom(SEED);
  for (let round = 0; round < ROUNDS / 10; round++) {
    const version = random(1, 40);
    const minEcl = random(0, 3);
    const capacity = 8 * CODEWORDS[version - 1][minEcl];
    // Dense enough content fits a third of capacity bits in chars
    const chars = randomChars(random, "mixed", Math.ceil(capacity / 3));
    const len = longest(prefixBits("mixed", chars), version, capacity);
    const mask = random(0, 7);
    for (const l of [len, len + 1]) {
      const content = chars.slice(0, l).join("");
      yield ["mixed", version, version, minEcl, 3, mask, content];
      yield ["mixed", 1, version, minEcl, minEcl, mask, content];
    }
  }
});

child.stdin.end();
