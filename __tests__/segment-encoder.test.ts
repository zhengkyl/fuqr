import QRCode from "qrcode";
import { describe, expect, test } from "vitest";
import {
  AlphanumericMode,
  NumericMode,
  SegmentEncoder,
} from "../typescript/src/extras/encoders.ts";
import {
  ByteMode,
  type Ecl,
  FuqrError,
  generateWithEncoder,
  type Mode,
  Module,
} from "../typescript/src/fuqr.ts";
import { VERSIONS } from "./data.ts";

const NAMES = new Map<Mode, "numeric" | "alphanumeric" | "byte">([
  [NumericMode, "numeric"],
  [AlphanumericMode, "alphanumeric"],
  [ByteMode, "byte"],
]);

// Deliberately not the shortest segmentations, as [mode, end] pairs
const CASES: [string, [Mode, number][]][] = [
  [
    "123",
    [
      [NumericMode, 1],
      [AlphanumericMode, 2],
      [ByteMode, 3],
    ],
  ],
  [
    "ABC123abc",
    [
      [AlphanumericMode, 4],
      [NumericMode, 6],
      [ByteMode, 9],
    ],
  ],
  [
    "1A2B34",
    [
      [ByteMode, 2],
      [AlphanumericMode, 4],
      [NumericMode, 6],
    ],
  ],
  [
    "héllo 42 WORLD",
    [
      [ByteMode, 6],
      [AlphanumericMode, 8],
      [NumericMode, 9],
      [AlphanumericMode, 15],
    ],
  ],
];

function split(ends: [Mode, number][]) {
  return ends.map(([mode, end], i) => ({ mode, start: i === 0 ? 0 : ends[i - 1][1], end }));
}

describe("SegmentEncoder", () => {
  test.each(VERSIONS)("matches node-qrcode version %i", (version) => {
    for (const [content, ends] of CASES) {
      const bytes = new TextEncoder().encode(content);
      const segments = split(ends);
      for (const ecl of [0, 3] as Ecl[]) {
        const where = `${JSON.stringify(content)} v${version} ecl${ecl}`;
        const options = {
          version,
          errorCorrectionLevel: (["L", "H"] as const)[ecl && 1],
          maskPattern: 2 as const,
        };
        const nodeSegments = segments.map(({ mode, start, end }) => {
          const data = bytes.subarray(start, end);
          const name = NAMES.get(mode)!;
          return name === "byte"
            ? { mode: name, data }
            : { mode: name, data: new TextDecoder().decode(data) };
        });

        let fuqr;
        try {
          fuqr = generateWithEncoder(new SegmentEncoder(bytes, segments), {
            minVersion: version,
            maxVersion: version,
            minEcl: ecl,
            maxEcl: ecl,
          }).matrix;
        } catch (err) {
          if (!(err instanceof FuqrError && err.code === "TEXT_TOO_LONG")) throw err;
          expect(() => QRCode.create(nodeSegments, options), where).toThrow();
          continue;
        }

        const nodeQrcode = QRCode.create(nodeSegments, options).modules.data;
        expect(
          fuqr.every((m, i) => (m & Module.ON) === nodeQrcode[i]),
          where,
        ).toBe(true);
      }
    }
  });

  test("bitLen matches encode", () => {
    for (const [content, ends] of CASES) {
      const bytes = new TextEncoder().encode(content);
      const encoder = new SegmentEncoder(bytes, split(ends));
      for (const version of [1, 10, 27]) {
        let pushed = 0;
        encoder.encode(version, (_, len) => (pushed += len));
        expect(pushed).toBe(encoder.bitLen(version));
      }
    }
  });

  test("rejects invalid segments", () => {
    const bytes = new TextEncoder().encode("12ab");
    const make = (segments: { mode: Mode; start: number; end: number }[]) => () =>
      new SegmentEncoder(bytes, segments);
    expect(make([{ mode: NumericMode, start: 0, end: 3 }])).toThrow(FuqrError);
    expect(make([{ mode: AlphanumericMode, start: 0, end: 4 }])).toThrow(FuqrError);
    expect(make([{ mode: ByteMode, start: 2, end: 5 }])).toThrow(FuqrError);
    expect(make([{ mode: ByteMode, start: 3, end: 2 }])).toThrow(FuqrError);
    expect(make([{ mode: NumericMode, start: 0, end: 2 }])).not.toThrow();
  });
});
