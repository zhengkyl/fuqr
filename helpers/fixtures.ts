import { writeFileSync } from "node:fs";
import { type Ecl, type Mask, NUM_DATA_BITS, NUM_EC_BYTES } from "../typescript/src/fuqr.ts";
import {
  type Case,
  check,
  describe,
  makeEncoder,
  type Mode,
  MODES,
  type Result,
  run,
} from "./common.ts";

const FIXTURES_DIR = new URL("fixtures/", import.meta.url);

const ALL = { minVersion: 1, maxVersion: 40, minEcl: 0, maxEcl: 3 } as const;

const UNITS = {
  numeric: "0123456789",
  alphanumeric: "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:",
  byte: "The quick brown fox jumps over the lazy dog. ",
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

// Longest content cycling unit with bitLen at most bits, by binary search.
// bitLen never shrinks as content grows, even for mixed, since dropping a
// char from an optimal encoding never makes it longer.
function longest(mode: Mode, unit: string, version: number, bits: number) {
  const fits = (len: number) => makeEncoder(mode, cycle(unit, len)).bitLen(version) <= bits;
  let lo = 0;
  let hi = 8000;
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if (fits(mid)) lo = mid;
    else hi = mid - 1;
  }
  return cycle(unit, lo);
}

// Mixed content with bitLen exactly bits. Its boundary is a mix of segment
// lengths rather than a char count, so trying units with different runs
// reaches any bit count, including those single modes can't.
function exactMixed(version: number, bits: number) {
  for (const unit of MIXED_UNITS) {
    const content = longest("mixed", unit, version, bits);
    if (makeEncoder("mixed", content).bitLen(version) === bits) return content;
  }
  throw new Error(`no mixed content has ${bits} bits at version ${version}`);
}

const dataBits = (version: number, ecl: number) =>
  ((NUM_DATA_BITS[version] >> 3) - NUM_EC_BYTES[version][ecl]) * 8;

// With version pinned and any ecl, each ecl gets the content too long for the
// next ecl, up to its own capacity. The largest and smallest of every version
// and ecl cover every block layout, padding and ecl selection. With default
// options, each version's largest and one more test version selection. Mask
// cycles so every ecl meets every mask.
function boundaries(mode: "numeric" | "alphanumeric" | "byte") {
  const cases: Case[] = [];
  for (let version = 1; version <= 40; version++) {
    const largest = (ecl: number) => longest(mode, UNITS[mode], version, dataBits(version, ecl));
    const justOver = (ecl: number) => cycle(UNITS[mode], largest(ecl).length + 1);

    const auto: Case = { mode, ...ALL, mask: (version % 8) as Mask, content: "" };
    cases.push({ ...auto, content: largest(0) }, { ...auto, content: justOver(0) });

    const pinned = { ...auto, minVersion: version, maxVersion: version };
    for (let ecl = 0; ecl < 4; ecl++) {
      cases.push(
        { ...pinned, content: largest(ecl) },
        { ...pinned, content: ecl === 3 ? "" : justOver(ecl + 1) },
      );
    }
  }
  return cases;
}

// Same as boundaries, but by exact bit count instead of char count, plus a
// cut short terminator with 1 to 3 bits left, which single modes can't reach.
function mixedBoundaries() {
  const cases: Case[] = [];
  for (let version = 1; version <= 40; version++) {
    const exact = (bits: number) => exactMixed(version, bits);

    const auto: Case = { mode: "mixed", ...ALL, mask: (version % 8) as Mask, content: "" };
    const full = dataBits(version, 0);
    cases.push({ ...auto, content: exact(full) }, { ...auto, content: exact(full + 1) });

    const pinned = { ...auto, minVersion: version, maxVersion: version };
    for (let ecl = 0; ecl < 4; ecl++) {
      const bits = dataBits(version, ecl);
      cases.push(
        { ...pinned, content: exact(bits) },
        { ...pinned, content: ecl === 3 ? "" : exact(dataBits(version, ecl + 1) + 1) },
        { ...pinned, content: exact(bits - 1 - ((version + ecl) % 3)) },
      );
    }
  }
  return cases;
}

function toCase(mode: Mode) {
  return (content: string, i: number) => ({
    mode,
    content,
    minVersion: 1,
    maxVersion: 40,
    minEcl: 0 as Ecl,
    maxEcl: 3 as Ecl,
    mask: (i % 8) as Mask,
  });
}

const CASES: Record<Mode, Case[]> = {
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
const files = new Map<Mode, string[]>();
const errors = [];
for (const mode of MODES) {
  const lines = [];
  // Other options giving the same result for a content add nothing
  const seen = new Set<string>();
  for (const c of CASES[mode]) {
    const result = run(c);
    const key = `${c.content} ${formatResult(result)}`;
    if (seen.has(key)) continue;
    seen.add(key);

    const error = await check(c, result);
    if (error !== "") errors.push(`${describe(c)} ${error}`);
    lines.push(formatFixture(c, result));
  }
  files.set(mode, lines);
}

if (errors.length > 0) {
  console.error(errors.slice(0, 20).join("\n"));
  console.error(`${errors.length} bad cases, fixtures not written`);
  process.exit(1);
}
for (const [mode, lines] of files) {
  writeFileSync(new URL(`${mode}.txt`, FIXTURES_DIR), lines.join("\n") + "\n");
  console.log(`wrote ${lines.length} fixtures to ${mode}.txt`);
}
