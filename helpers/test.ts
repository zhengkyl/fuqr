// Test suites, checked against a port by harness.ts.
//
// Usage: node helpers/test.ts <adapter command...>
// FUZZ_SEED replays a run and FUZZ_ROUNDS scales the random suites. FUZZ_FULL=1
// also runs exhaustive strings at versions 10 and 27, which is much slower.
import {
  BYTE,
  close,
  DATA_CODEWORDS,
  type Encoder,
  ENCODERS,
  group,
  type Mode,
  prefixes,
  run,
  segmentBits,
  utf8,
} from "./harness.ts";

const SEED = Number(process.env.FUZZ_SEED ?? Math.floor(Math.random() * 2 ** 32));
const ROUNDS = Number(process.env.FUZZ_ROUNDS ?? 300);
const FULL = process.env.FUZZ_FULL === "1";

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
    byte: "Hello, wörld! 日本語のテキスト 🎉 https://example.com/?q=code&n=42\n",
    mixed: "https://example.com/2470295/MANUALS/525322511#step-3?Q=ABC%20DEF&é=日本0000A00a",
  };
  for (const encoder of ENCODERS) {
    const text = [...texts[encoder]];
    const chars = Array.from({ length: 7090 }, (_, i) => text[i % text.length]);
    const bits = prefixBits(encoder, chars);
    for (let version = 1; version <= 40; version++) {
      for (let ecl = 0; ecl < 4; ecl++) {
        const len = longest(bits, version, 8 * DATA_CODEWORDS[version][ecl]);
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
    const capacity = 8 * DATA_CODEWORDS[version][minEcl];
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

close();

// ---- Helpers for generating cases ----

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

// Runs of one pool, mostly short since those decide when to switch
function randomChars(random: ReturnType<typeof makeRandom>, encoder: Encoder, minBytes: number) {
  // Chars by the cheapest mode they fit in
  const pools = [[..."0123456789"], [..."ABCXYZ $%*+-./:"], [..."abcxyz!?\n~éß日本🎉"]];
  const top = encoder === "mixed" ? 2 : ENCODERS.indexOf(encoder);
  const chars: string[] = [];
  let bytes = 0;
  while (bytes < minBytes) {
    const pool = pools[random(0, top)];
    const len = random(0, 3) === 0 ? random(1, 60) : random(1, 12);
    for (let i = 0; i < len; i++) {
      chars.push(pool[random(0, pool.length - 1)]);
      bytes += utf8(chars.at(-1)!).length;
    }
  }
  return chars;
}
