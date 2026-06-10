import { NUM_BYTES, NUM_EC_BYTES } from "./constants.js";
import { ByteEncoder, type Encoder } from "./encoder.js";
import { NoopFixer, type Fixer } from "./fixer.js";
import { buildMatrix } from "./matrix.js";
import { MAX_VERSION, QrError, type Ecl, type Mask, type Version } from "./types.js";

export { NUM_BLOCKS, NUM_BYTES, NUM_DATA_MODULES, NUM_EC_BYTES } from "./constants.js";
export { buildGeneratorMatrix, gf256MatrixInvert } from "./ecc.js";
export { AlphanumericEncoder, ByteEncoder, NumericEncoder } from "./encoder.js";
export type { Encoder } from "./encoder.js";
export { NoopFixer, PixelArtFixer } from "./fixer.js";
export type { Fixer } from "./fixer.js";
export { buildMatrix, iterateMostlyDataModules } from "./matrix.js";
export { imageToLogo, resizeStencil } from "./stencil.js";
export type { Logo, Stencil } from "./stencil.js";
export { MAX_VERSION, Module, QrError } from "./types.js";
export type { Ecl, Mask, QrErrorCode, Version } from "./types.js";

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
