// Checks against fixtures in the repo root. See helpers/fixtures.ts there for
// the format.
import { readdirSync, readFileSync } from "node:fs";
import { expect, test } from "vitest";
import { AlphanumericEncoder, MixedEncoder, NumericEncoder } from "../src/extras/encoders.ts";
import { ByteEncoder, FuqrError, type GenerateOptions, generateWithEncoder } from "../src/fuqr.ts";

const FIXTURES_DIR = new URL("../../helpers/fixtures/", import.meta.url);

// "unit"*N means unit cycled to N chars
function parseContent(field: string) {
  const match = /^(".*")\*(\d+)$/.exec(field);
  if (match == null) return JSON.parse(field) as string;
  const unit = JSON.parse(match[1]) as string;
  const len = Number(match[2]);
  return unit.repeat(Math.ceil(len / unit.length)).slice(0, len);
}

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

const ENCODERS = {
  numeric: NumericEncoder,
  alphanumeric: AlphanumericEncoder,
  byte: ByteEncoder,
  mixed: MixedEncoder,
};

function run(mode: string, content: string, fields: string[]) {
  const [minVersion, maxVersion, minEcl, maxEcl, mask] = fields.map(Number);
  const options = { minVersion, maxVersion, minEcl, maxEcl, mask } as GenerateOptions;
  try {
    const encoder = new ENCODERS[mode as keyof typeof ENCODERS](content);
    const qr = generateWithEncoder(encoder, options);
    return `${qr.version}-${qr.ecl}-${qr.mask}-${fnv1a64(qr.matrix)}`;
  } catch (err) {
    if (err instanceof FuqrError) return err.code;
    throw err;
  }
}

const names = readdirSync(FIXTURES_DIR);

test.each(names)(
  "%s",
  (name) => {
    const mode = name.replace(/\.txt$/, "");
    const lines = readFileSync(new URL(name, FIXTURES_DIR), "utf8").trimEnd().split("\n");
    const errors = [];
    for (const [i, line] of lines.entries()) {
      const fields = line.split(" ");
      const expected = fields[5];
      const actual = run(mode, parseContent(fields.slice(6).join(" ")), fields.slice(0, 5));
      if (actual !== expected) errors.push(`${name}:${i + 1}: expected ${expected}, got ${actual}`);
    }
    expect(errors.slice(0, 10)).toEqual([]);
  },
  60_000,
);
