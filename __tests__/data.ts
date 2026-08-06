// Content lists, grouped by the narrowest mode that can carry them. Lengths are
// picked to land on every remainder in the groupings the modes encode with:
// threes for numeric, pairs for alphanumeric, and byte boundaries throughout.
export const NUMERIC = [
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

export const ALPHANUMERIC = [
  "A",
  "42",
  " ",
  "$%*+-./:",
  "HELLO",
  "FUQR 42",
  "AB CD EF",
  "HTTPS://EXAMPLE.COM",
  "HTTPS://EXAMPLE.COM/PATH $%*+-./:",
  "ORDER 4200 $9.99 SHIP-TO 90210",
  "A1".repeat(15),
  "Z".repeat(64),
  "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:",
];

export const BYTE = [
  "",
  "a",
  "hello",
  "Hello, World!",
  "  tab\there\nnewline",
  "https://example.com/some/long/path?with=query&more=params#anchor",
  "https://example.com/2470295/manuals/525322511#step-3",
  "héllo wörld, grüße",
  "日本語のテキストです",
  "\u{1f389}\u{1f680} party \u{1f44d}\u{1f3fd}",
  "id 42 — ünïcödé \u{1f4e6} END",
  "Order #42 costs $9.99 — ship to 90210 \u{1f4e6}",
  "x".repeat(300),
  "z".repeat(1500),
];

function mulberry32(seed: number) {
  return () => {
    seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const CODE_POINT_RANGES = [
  [0x30, 0x39], // digits, numeric mode
  [0x41, 0x5a], // upper case, alphanumeric mode
  [0x61, 0x7a], // lower case, byte mode
  [0x20, 0x2f], // punctuation, straddles alphanumeric and byte
  [0xa0, 0x24f], // two byte
  [0x3040, 0x9fff], // three byte
  [0x1f300, 0x1f6ff], // four byte
];

// Each string draws from a few ranges, so runs form and the mixed encoder has
// to decide where to switch modes.
export function randomContents(count: number, seed: number) {
  const random = mulberry32(seed);
  return Array.from({ length: count }, () => {
    const picks = [0, 0, 0].map(() => CODE_POINT_RANGES[Math.floor(random() * 7)]);
    let content = "";
    for (let i = 1 + Math.floor(random() * 60); i > 0; i--) {
      const [lo, hi] = picks[Math.floor(random() * picks.length)];
      content += String.fromCodePoint(lo + Math.floor(random() * (hi - lo + 1)));
    }
    return content;
  });
}

export const RANDOM = randomContents(16, 20260806);
