import { NUM_BYTES, NUM_EC_BYTES } from "./constants.ts";
import { ByteEncoder, type Encoder } from "./encoder.ts";
import { NoopFixer, type Fixer } from "./fixer.ts";
import { buildMatrix } from "./matrix.ts";
import { MAX_VERSION, QrError, type Ecl, type Mask, type Version } from "./types.ts";

export { NUM_BLOCKS, NUM_BYTES, NUM_DATA_MODULES, NUM_EC_BYTES } from "./constants.ts";
export { buildGeneratorMatrix } from "./ecc.ts";
export { AlphanumericEncoder, ByteEncoder, NumericEncoder } from "./encoder.ts";
export type { Encoder } from "./encoder.ts";
export { NoopFixer, PixelArtFixer } from "./fixer.ts";
export type { Fixer } from "./fixer.ts";
export { buildMatrix, iterateMostlyDataModules } from "./matrix.ts";
export { imageToLogo, resizeStencil } from "./stencil.ts";
export type { Logo, Stencil } from "./stencil.ts";
export { MAX_VERSION, Module, QrError } from "./types.ts";
export type { Ecl, Mask, QrErrorCode, Version } from "./types.ts";

// Find the minimum version and ECL where the content fits, adjusted by the fixer.
export function findVersionEcl(
  encoder: Encoder,
  fixer: Fixer,
  options: { minVersion?: Version; minEcl?: Ecl } = {},
): { version: Version; ecl: Ecl } {
  let { minVersion = 1, minEcl = 0 } = options;

  while (minVersion <= MAX_VERSION) {
    const reqBytes = Math.ceil(encoder.bitLen(minVersion) / 8);
    if (reqBytes <= NUM_BYTES[minVersion] - NUM_EC_BYTES[minVersion][minEcl]) break;
    minVersion++;
  }
  if (minVersion > MAX_VERSION) throw new QrError("TEXT_TOO_LONG");

  return fixer.fixVersionEcl(minVersion, minEcl, encoder);
}

// Convenience: encode text as bytes, find minimum version/ECL, and build the matrix.
export function generate(
  text: string,
  options: { minVersion?: Version; minEcl?: Ecl; mask?: Mask; fixer?: Fixer } = {},
) {
  const encoder = new ByteEncoder(text);
  const fixer = options.fixer ?? new NoopFixer();
  const { version, ecl } = findVersionEcl(encoder, fixer, options);
  return buildMatrix(encoder, fixer, version, ecl, options.mask);
}
