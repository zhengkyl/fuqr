import { readFileSync } from "node:fs";
import { describe, expect, test } from "vitest";
import { AlphanumericEncoder, MixedEncoder } from "../src/extras/encoders.ts";

// Curated and random contents from the root fixtures, skipping long "unit"*N
const FIXTURE_CONTENTS = new Set(
  readFileSync(new URL("../../helpers/fixtures/mixed.txt", import.meta.url), "utf8")
    .trimEnd()
    .split("\n")
    .map((line) => line.split(" ").slice(6).join(" "))
    .filter((field) => field.endsWith('"'))
    .map((field) => JSON.parse(field) as string),
);

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
  const encoder = new MixedEncoder(content);
  const bits = encoder.bitLen(version);
  const shortest = shortestBitLen(content, version);
  let pushed = 0;
  encoder.encode(version, (_, len) => (pushed += len));
  if (bits === shortest && pushed === bits) return "";
  return `${JSON.stringify(content)} v${version} bitLen ${bits}, shortest ${shortest}, pushed ${pushed}`;
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
function strings(alphabet: string[], maxLen: number) {
  let layer = [""];
  const all = [];
  for (let len = 1; len <= maxLen; len++) {
    layer = layer.flatMap((s) => alphabet.map((c) => s + c));
    all.push(...layer);
  }
  return all;
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

  test.each(VERSIONS)("fixture contents version %i", (version) => {
    checkAll(FIXTURE_CONTENTS, version);
  });
});
