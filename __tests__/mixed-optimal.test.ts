import { describe, expect, test } from "vitest";
import { AlphanumericEncoder, MixedEncoder } from "../typescript/src/extras/encoders.ts";
import { BYTE_CONTENTS } from "./data.ts";

// Version groups with different char count indicator lengths
const VERSIONS = [1, 9, 10, 26, 27, 40];

function modeOf(byte: number) {
  if (0x30 <= byte && byte <= 0x39) return 0;
  if (AlphanumericEncoder.byteToB45(byte) !== 255) return 1;
  return 2;
}

function segmentBits(mode: number, len: number, version: number) {
  const cciDiff = (version > 9 ? 2 : 0) + (version > 26 ? 2 : 0);
  if (mode === 0) return 4 + 10 + cciDiff + Math.ceil((len * 10) / 3);
  if (mode === 1) return 4 + 9 + cciDiff + Math.ceil((len * 11) / 2);
  return 4 + (version < 10 ? 8 : 16) + len * 8;
}

// Obviously correct O(n^2) search over every possible last segment
function shortestBitLen(content: string, version: number) {
  const bytes = new TextEncoder().encode(content);
  const n = bytes.length;
  if (n === 0) return segmentBits(2, 0, version);

  const best = new Array(n + 1).fill(Infinity);
  best[0] = 0;
  for (let end = 1; end <= n; end++) {
    for (let mode = 0; mode < 3; mode++) {
      for (let start = end - 1; start >= 0 && modeOf(bytes[start]) <= mode; start--) {
        best[end] = Math.min(best[end], best[start] + segmentBits(mode, end - start, version));
      }
    }
  }
  return best[n];
}

// Returns a description of what is wrong, or "" if nothing
function check(content: string, version: number) {
  const where = `${JSON.stringify(content)} v${version}`;
  const encoder = new MixedEncoder(content);
  const bits = encoder.bitLen(version);
  const shortest = shortestBitLen(content, version);
  if (bits !== shortest) return `${where} bitLen ${bits}, shortest ${shortest}`;

  // Segments are contiguous, valid, and add up to bitLen
  const { bytes, segments } = encoder;
  let pos = 0;
  let total = 0;
  for (let i = 0; i < segments.length; i++) {
    const { mode, start, end } = segments[i];
    if (start !== pos) return `${where} segment ${i} starts at ${start}, expected ${pos}`;
    if (i > 0 && mode === segments[i - 1].mode) return `${where} segment ${i} repeats mode`;
    for (let j = start; j < end; j++) {
      if (modeOf(bytes[j]) > mode) return `${where} byte ${j} invalid in mode ${mode}`;
    }
    total += segmentBits(mode, end - start, version);
    pos = end;
  }
  if (pos !== bytes.length) return `${where} segments end at ${pos}`;
  if (total !== bits) return `${where} segments total ${total}, bitLen ${bits}`;

  let pushed = 0;
  encoder.encode(version, (_, len) => (pushed += len));
  if (pushed !== bits) return `${where} pushed ${pushed}, bitLen ${bits}`;

  return "";
}

function checkAll(contents: Iterable<string>, version: number) {
  const errors = [];
  for (const content of contents) {
    const error = check(content, version);
    if (error !== "") errors.push(error);
  }
  expect(errors.slice(0, 10)).toEqual([]);
}

// All strings of length up to maxLen made from alphabet
function* strings(alphabet: string[], maxLen: number): Generator<string> {
  let layer = [""];
  for (let len = 1; len <= maxLen; len++) {
    layer = layer.flatMap((s) => alphabet.map((c) => s + c));
    yield* layer;
  }
}

describe("mixed mode is optimal", () => {
  test("regressions", () => {
    // 7 numeric + 2 alphanumeric beats 9 alphanumeric
    expect(new MixedEncoder("0000000A0").bitLen(1)).toBe(62);
    expect(new MixedEncoder("").bitLen(1)).toBe(12);
  });

  test.each(VERSIONS)(
    "exhaustive short strings version %i",
    (version) => {
      checkAll(strings(["0", "A", "a"], 10), version);
      checkAll(strings(["7", ":", " ", "b", "é"], 5), version);
    },
    60_000,
  );

  test.each(VERSIONS)("test data version %i", (version) => {
    checkAll(BYTE_CONTENTS, version);
  });

  test("reuses segments across versions", () => {
    const encoder = new MixedEncoder("https://example.com/2470295/manuals/525322511#step-3");
    const small = encoder.bitLen(1);
    const smallSegments = encoder.segments;
    const large = encoder.bitLen(10);
    expect(large).toBeGreaterThan(small);
    expect(encoder.bitLen(9)).toBe(small);
    expect(encoder.segments).toBe(smallSegments);
  });
});
