import { writeFileSync } from "node:fs";
import { MixedEncoder } from "../typescript/src/extras/encoders.ts";
import { type Ecl, type Mask, NUM_DATA_BITS, NUM_EC_BYTES } from "../typescript/src/fuqr.ts";
import {
  type Case,
  check,
  describe,
  type Encoder,
  type Mode,
  MODES,
  type Result,
  run,
} from "./common.ts";

const FIXTURES_DIR = new URL("fixtures/", import.meta.url);

const ALL = { minVersion: 1, maxVersion: 40, minEcl: 0, maxEcl: 3 } as const;

// Version and min ecl pinned, so the next ecl can't take content that overflows.
// Mask cycles so every ecl meets every mask.
const pinned = (encoder: Encoder, version: number, ecl: number) => ({
  encoder,
  minVersion: version,
  maxVersion: version,
  minEcl: ecl as Ecl,
  maxEcl: 3 as Ecl,
  mask: ((version + ecl) % 8) as Mask,
});

const FILLER: Record<Mode, string> = {
  numeric: "0123456789",
  alphanumeric: "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:",
  // only 1 byte codepoints to fill arbitrary length
  byte: `HTTPS://abc.defghi.jkl/m?no=pq&rs=tuv#wxyz %+-_$!,;"@^*[](){}\n`,
};

// Runs of byte, alphanumeric and numeric chars, long enough to be their own
// segments where possible. All three modes are tried first.
const MIXED_UNITS: string[] = [];
for (const b of [1, 2, 3, 4, 0]) {
  for (const a of [12, 13, 14, 15, 16, 1, 2, 3, 4, 5, 6, 0]) {
    for (let d = 4; d <= 20; d++) {
      MIXED_UNITS.push(
        "abcd".slice(0, b) + "ABCDEFGHIJKLMNOP".slice(0, a) + "0123456789".repeat(2).slice(0, d),
      );
    }
  }
}

const cycle = (unit: string, len: number) =>
  unit.repeat(Math.ceil(len / unit.length)).slice(0, len);

const dataBits = (version: number, ecl: number) =>
  ((NUM_DATA_BITS[version] >> 3) - NUM_EC_BYTES[version][ecl]) * 8;

function longest(fits: (len: number) => boolean) {
  let lo = 0; // inclusive
  let hi = 8000; // exclusive
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if (fits(mid)) lo = mid;
    else hi = mid - 1;
  }
  return lo;
}

function boundaries(mode: Mode) {
  const cases: Case[] = [];
  for (let version = 1; version <= 40; version++) {
    for (let ecl = 0; ecl < 4; ecl++) {
      const options = pinned(mode, version, ecl);
      const bits = dataBits(version, ecl);
      const len = longest((len) => MODES[mode].segLen(version, len) <= bits);
      const content = cycle(FILLER[mode], len);
      cases.push({ ...options, content }, { ...options, content: content + "0" });
    }
  }
  return cases;
}

// Mixed content with bitLen exactly bits. Its boundary is a mix of segment
// lengths rather than a char count, so trying units with different runs
// reaches any bit count, including those single modes can't.
function exactMixed(version: number, bits: number) {
  for (const unit of MIXED_UNITS) {
    const bitLen = (len: number) => new MixedEncoder(cycle(unit, len)).bitLen(version);
    const len = longest((len) => bitLen(len) <= bits);
    if (bitLen(len) === bits) return cycle(unit, len);
  }
  throw new Error(`no mixed content has ${bits} bits at version ${version}`);
}

// Same as boundaries, but by exact bit count, plus a cut short terminator
// with 1 to 3 bits left, which single modes can't reach
function mixedBoundaries() {
  const cases: Case[] = [];
  for (let version = 1; version <= 40; version++) {
    for (let ecl = 0; ecl < 4; ecl++) {
      const options = pinned("mixed", version, ecl);
      const bits = dataBits(version, ecl);
      cases.push(
        { ...options, content: exactMixed(version, bits) },
        { ...options, content: exactMixed(version, bits + 1) },
        { ...options, content: exactMixed(version, bits - 1 - ((version + ecl) % 3)) },
      );
    }
  }
  return cases;
}

function toCase(encoder: Encoder) {
  return (content: string, i: number) => ({
    encoder,
    content,
    ...ALL,
    mask: (i % 8) as Mask,
  });
}

const CASES: Record<Encoder, Case[]> = {
  numeric: [
    ...[
      "",
      "0",
      "42",
      "123",
      "1234",
      "12345",
      "8675309",
      "00000000000000000000",
      // Invalid
      "12a",
      "١٢",
    ].map(toCase("numeric")),
    ...boundaries("numeric"),
  ],
  alphanumeric: [
    ...[
      // Alphanumeric packs 2 chars, with 0 or 1 left over
      "",
      "A",
      "AB",
      "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:",
      // Uppercase URLs fit alphanumeric
      "HTTPS://EXAMPLE.COM",
      "HTTPS://EXAMPLE.COM/ABC/123",
      "HTTP://WWW.EXAMPLE.COM/P/0042",
      "HTTPS://EXAMPLE.COM/VERIFY/ABCD-EFGH-IJKL-MNOP",
      // Invalid
      "https://example.com",
      "Hello",
    ].map(toCase("alphanumeric")),
    ...boundaries("alphanumeric"),
  ],
  byte: [
    ...[
      // 1, 2, 3 and 4 byte UTF-8
      "",
      "a",
      "Hello, World!",
      "tab\there\nnewline",
      "héllo wörld, grüße",
      "日本語のテキストです",
      "🎉🚀 party 👍🏽",
      // First and last code points of each UTF-8 length, around surrogates
      "\u{7f}\u{80}\u{7ff}\u{800}\u{d7ff}\u{e000}\u{ffff}\u{10000}\u{10ffff}",
      "https://example.com",
      "https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=42s",
      "WIFI:T:WPA;S:MyNetwork;P:correct horse battery staple;;",
    ].map(toCase("byte")),
    ...boundaries("byte"),
  ],
  mixed: [
    ...[
      "",
      // 7 numeric + 2 alphanumeric beats 9 alphanumeric
      "0000000A0",
      // Numeric and alphanumeric tie before switching to byte
      "A0000000a",
      // Runs short and long enough to switch
      "ABC123DEF",
      "ABC123456DEF",
      "ABCDEFGH1234567890IJKLMNOP",
      "abc123def",
      "abc12345678def",
      "abcHELLOdef",
      "abcHELLOWORLD1234def",
      // URLs
      "https://example.com/products/12345678",
      "https://example.com/2470295/manuals/525322511#step-3",
      "https://example.com/t/0123456789012345678901234567890123456789",
    ].map(toCase("mixed")),
    ...mixedBoundaries(),
  ],
};

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

function formatResult(result: Result) {
  if ("error" in result) return result.error;
  return `${result.version}-${result.ecl}-${result.mask}-${fnv1a64(result.matrix)}`;
}

// Writes long ASCII content made of a repeated unit as "unit"*N
function formatContent(content: string) {
  if (content.length < 32 || /[^\x00-\x7f]/.test(content)) return JSON.stringify(content);
  for (let period = 1; period <= 64 && period * 2 <= content.length; period++) {
    const unit = content.slice(0, period);
    if (unit.repeat(Math.ceil(content.length / period)).startsWith(content)) {
      return `${JSON.stringify(unit)}*${content.length}`;
    }
  }
  return JSON.stringify(content);
}

function formatFixture(c: Case, result: Result) {
  const options = `${c.minVersion} ${c.maxVersion} ${c.minEcl} ${c.maxEcl} ${c.mask}`;
  return `${options} ${formatResult(result)} ${formatContent(c.content)}`;
}

// Checks every case before writing any file
const files = new Map<Encoder, string[]>();
const errors = [];
for (const [encoder, cases] of Object.entries(CASES) as [Encoder, Case[]][]) {
  const lines = [];
  // Other options giving the same result for a content add nothing
  const seen = new Set<string>();
  for (const c of cases) {
    const result = run(c);
    const key = `${c.content} ${formatResult(result)}`;
    if (seen.has(key)) continue;
    seen.add(key);

    const error = await check(c, result);
    if (error !== "") errors.push(`${describe(c)} ${error}`);
    lines.push(formatFixture(c, result));
  }
  files.set(encoder, lines);
}

if (errors.length > 0) {
  console.error(errors.slice(0, 20).join("\n"));
  console.error(`${errors.length} bad cases, fixtures not written`);
  process.exit(1);
}
for (const [encoder, lines] of files) {
  writeFileSync(new URL(`${encoder}.txt`, FIXTURES_DIR), lines.join("\n") + "\n");
  console.log(`wrote ${lines.length} fixtures to ${encoder}.txt`);
}
