import { type Version } from "../typescript/src/fuqr.ts";

// Boundaries: alignment patterns (2, 7), version info (7),
// char count indicator bit len (10, 27), max ecc blocks (40)
const FAST_VERSIONS = [1, 2, 6, 7, 9, 10, 13, 26, 27, 40] as Version[];
const ALL_VERSIONS = Array.from({ length: 40 }, (_, i) => (i + 1) as Version);
export const VERSIONS = process.env.FUQR_FAST ? FAST_VERSIONS : ALL_VERSIONS;

const NUMERIC = [
  "0",
  "42",
  "007",
  "1234",
  "86753",
  "000000",
  "8675309",
  "0123456789",
  "9".repeat(31),
  "0".repeat(32),
  "5".repeat(33),
  "8675309".repeat(20),
  "0123456789".repeat(30),
];

const ALPHANUMERIC = [
  "A",
  " ",
  "$%*+-./:",
  "HELLO",
  "FUQR 42",
  "AB CD EF",
  "HTTPS://EXAMPLE.COM",
  "HTTPS://EXAMPLE.COM/PATH $%*+-./:",
  "ORDER 4200 $9.99 SHIP-TO 90210",
  "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:",
  "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:".repeat(10),
  "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:".repeat(100),
];

const BYTE = [
  "a",
  "hello",
  "Hello, World!",
  "  tab\there\nnewline",
  "https://example.com/some/long/path?with=query&more=params#anchor",
  "héllo wörld, grüße",
  "日本語のテキストです",
  "\u{1f389}\u{1f680} party \u{1f44d}\u{1f3fd}",
  "id 42 — ünïcödé \u{1f4e6} END",
  "Order #42 costs $9.99 — ship to 90210 \u{1f4e6}",
  "https://example.com/2470295/manuals/525322511#step-3",
  "https://example.com/2470295/manuals/525322511#step-3".repeat(30),
];

function mulberry32(seed: number) {
  return () => {
    seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const DIGITS = [[0x30, 0x39]]; // numeric mode

const ALPHANUMERIC_ONLY = [
  [0x41, 0x5a], // upper case
  [0x20, 0x20], // space
  [0x24, 0x25], // $%
  [0x2a, 0x2b], // *+
  [0x2d, 0x2f], // -./
  [0x3a, 0x3a], // :
];

const BYTE_ONLY = [
  [0x61, 0x7a], // lower case
  [0x21, 0x23], // punctuation alphanumeric mode leaves out
  [0xa0, 0x24f], // two byte
  [0x3040, 0x9fff], // three byte
  [0x1f300, 0x1f6ff], // four byte
];

// Each string draws from a few ranges, so runs form and the mixed encoder has
// to decide where to switch modes.
function randomContents(count: number, seed: number, ranges: number[][]) {
  const random = mulberry32(seed);
  return Array.from({ length: count }, () => {
    const picks = [0, 0, 0].map(() => ranges[Math.floor(random() * ranges.length)]);
    let content = "";
    for (let i = 1 + Math.floor(random() * 60); i > 0; i--) {
      const [lo, hi] = picks[Math.floor(random() * picks.length)];
      content += String.fromCodePoint(lo + Math.floor(random() * (hi - lo + 1)));
    }
    return content;
  });
}

export const NUMERIC_CONTENTS = [...NUMERIC, ...randomContents(8, 20260806, DIGITS)];

export const ALPHANUMERIC_CONTENTS = [
  ...NUMERIC,
  ...ALPHANUMERIC,
  ...randomContents(8, 20260807, [...DIGITS, ...ALPHANUMERIC_ONLY]),
];

export const BYTE_CONTENTS = [
  ...NUMERIC,
  ...ALPHANUMERIC,
  ...BYTE,
  ...randomContents(16, 20260808, [...DIGITS, ...ALPHANUMERIC_ONLY, ...BYTE_ONLY]),
];
