import { describe, expect, test } from "vitest";
import {
  ByteEncoder,
  type Ecl,
  type Encoder,
  FuqrError,
  generateWithEncoder,
  type Mask,
  NUM_DATA_BITS,
  NUM_EC_BYTES,
  type Version,
} from "../typescript/src/fuqr.ts";
import {
  AlphanumericEncoder,
  MixedEncoder,
  NumericEncoder,
} from "../typescript/src/extras/encoders.ts";
import { ALPHANUMERIC, BYTE, NUMERIC, RANDOM } from "./data.ts";
import { scan } from "./scan.ts";

const VERSIONS = Array.from({ length: 40 }, (_, i) => (i + 1) as Version);
const ECLS = [0, 1, 2, 3] as Ecl[];
const MASKS = [0, 1, 2, 3, 4, 5, 6, 7] as Mask[];

const GROUPS: [string, string[]][] = [
  ["numeric", NUMERIC],
  ["alphanumeric", ALPHANUMERIC],
  ["byte", BYTE],
  ["random", RANDOM],
];

// Every encoder that can carry this content, since narrower modes only accept
// their own character set but wider ones accept anything.
function encodersFor(content: string): (() => Encoder)[] {
  const encoders: (() => Encoder)[] = [
    () => new ByteEncoder(content),
    () => new MixedEncoder(content),
  ];
  if (/^[0-9]*$/.test(content)) {
    encoders.push(() => new NumericEncoder(content));
  }
  if (/^[0-9A-Z $%*+\-./:]*$/.test(content)) {
    encoders.push(() => new AlphanumericEncoder(content));
  }
  return encoders;
}

// Content that outgrows the requested version is not a failure, it just means
// that combination has nothing to test.
function generateOrSkip(encoder: Encoder, options: Record<string, number>) {
  try {
    return generateWithEncoder(encoder, options);
  } catch (error) {
    if (error instanceof FuqrError && error.code === "TEXT_TOO_LONG") return null;
    throw error;
  }
}

describe.each(GROUPS)("%s content", (_, contents) => {
  // Every entry, in every mode that can carry it, at every ecl.
  test.each(contents)("every mode and ecl: %j", async (content) => {
    for (const makeEncoder of encodersFor(content)) {
      for (const ecl of ECLS) {
        const qr = generateOrSkip(makeEncoder(), { minEcl: ecl, maxEcl: ecl });
        if (qr !== null) await scan(qr, content);
      }
    }
  });

  // Every version, cycling through the list and the ecls so the sweep stays
  // proportional to the version count rather than multiplying by it.
  test.each(VERSIONS)("version %i", async (version) => {
    const content = contents[version % contents.length];
    const ecl = ECLS[version % ECLS.length];

    for (const makeEncoder of encodersFor(content)) {
      const qr = generateOrSkip(makeEncoder(), {
        minVersion: version,
        maxVersion: version,
        minEcl: ecl,
        maxEcl: ecl,
      });
      if (qr !== null) await scan(qr, content);
    }
  });

  test.each(MASKS)("mask %i", async (mask) => {
    const content = contents[mask % contents.length];
    for (const makeEncoder of encodersFor(content)) {
      const qr = generateOrSkip(makeEncoder(), { mask });
      if (qr !== null) await scan(qr, content);
    }
  });
});

// Filling a version leaves no padding, so every codeword carries content and the
// character count indicator is pushed to its widest value.
test.each(VERSIONS)("version %i filled to capacity at every ecl", async (version) => {
  for (const ecl of ECLS) {
    const numBytes = NUM_DATA_BITS[version] >> 3;
    const capacity = numBytes - NUM_EC_BYTES[version][ecl] - (version < 10 ? 2 : 3);
    const content = "A".repeat(capacity);
    const options = { minVersion: version, maxVersion: version, minEcl: ecl, maxEcl: ecl };
    await scan(generateWithEncoder(new ByteEncoder(content), options), content);
  }
});

test("content too long for the version range", () => {
  expect(() =>
    generateWithEncoder(new ByteEncoder("x".repeat(100)), { maxVersion: 1 }),
  ).toThrowError(expect.objectContaining({ code: "TEXT_TOO_LONG" }) as Error);
});
