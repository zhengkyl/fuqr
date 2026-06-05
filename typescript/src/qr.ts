import { NUM_CODEWORDS, NUM_EC_CODEWORDS } from "./constants.js";
import { type Encoder } from "./encoder.js";
import type { Fixer } from "./fixer.js";
import { MAX_VERSION, QrError, type Ecl, type Version } from "./types.js";

export { NUM_BLOCKS, NUM_CODEWORDS, NUM_DATA_MODULES, NUM_EC_CODEWORDS } from "./constants.js";
export { buildGeneratorMatrix, gf256MatrixInvert } from "./ecc.js";
export { AlphanumericEncoder, ByteEncoder, NumericEncoder } from "./encoder.js";
export type { Encoder } from "./encoder.js";
export { LogoFixer as ExactLogoStamper, PixelArtFixer as PixelArtStamper } from "./fixer.js";
export type { Fixer as Stamper } from "./fixer.js";
export {
  buildMatrix,
  generateCodewordMatrix,
  iterateMostlyDataModules as traverseDataBits,
} from "./matrix.js";
export { imageToLogo, resizeStencil } from "./stencil.js";
export type { Logo, Stencil } from "./stencil.js";
export { MAX_VERSION, Module, QrError } from "./types.js";
export type { Ecl, Mask, QrErrorCode, Version } from "./types.js";

// Find the minimum version and maximum ECL at that version that fits the encoder's data.
// export function findVersionEcl(
//   encoder: Encoder,
//   options: {
//     minVersion?: Version;
//     minEcl?: Ecl;
//     strictVersion?: boolean;
//     strictEcl?: boolean;
//   } = {},
// ): { version: Version; ecl: Ecl } {
//   let { minVersion = 1, minEcl = 0, strictVersion = false, strictEcl = false } = options;

//   while (minVersion <= MAX_VERSION) {
//     const reqCw = Math.ceil(encoder.bitLen(minVersion) / 8);
//     if (reqCw <= NUM_CODEWORDS[minVersion] - NUM_EC_CODEWORDS[minVersion][minEcl]) break;
//     if (strictVersion) throw new QrError("TEXT_TOO_LONG");
//     minVersion++;
//   }
//   if (minVersion > MAX_VERSION) throw new QrError("TEXT_TOO_LONG");

//   const reqCw = Math.ceil(encoder.bitLen(minVersion) / 8);
//   let maxEcl = minEcl;
//   if (!strictEcl) {
//     for (let e = 3; e > minEcl; e--) {
//       if (reqCw <= NUM_CODEWORDS[minVersion] - NUM_EC_CODEWORDS[minVersion][e]) {
//         maxEcl = e;
//         break;
//       }
//     }
//   }

//   return { version: minVersion as Version, ecl: maxEcl as Ecl };
// }

// Find the minimum version/ECL where both the encoder's data and the stamper fit.
export function findVersionEclWithStamp(
  encoder: Encoder,
  fixer: Fixer,
  options: { minVersion?: Version; minEcl?: Ecl } = {},
): { version: Version; ecl: Ecl } {
  let { minVersion = 1, minEcl = 0 } = options;

  while (minVersion <= MAX_VERSION) {
    const reqCw = Math.ceil(encoder.bitLen(minVersion) / 8);
    if (reqCw <= NUM_CODEWORDS[minVersion] - NUM_EC_CODEWORDS[minVersion][minEcl]) break;
    minVersion++;
  }
  if (minVersion > MAX_VERSION) throw new QrError("TEXT_TOO_LONG");

  const reqCw = Math.ceil(encoder.bitLen(minVersion) / 8);
  while (minVersion <= MAX_VERSION) {
    while (
      minEcl < 3 &&
      reqCw <= NUM_CODEWORDS[minVersion] - NUM_EC_CODEWORDS[minVersion][minEcl + 1]
    ) {
      minEcl++;
    }
    if (fixer.fits(minVersion as Version, minEcl as Ecl, reqCw)) {
      return { version: minVersion as Version, ecl: minEcl as Ecl };
    }
    minVersion++;
  }

  throw new QrError("LOGO_TOO_LARGE");
}

// Convenience: encode text as bytes, find minimum version/ECL, and build the matrix.
// export function generate(
//   text: string,
//   options: { minVersion?: Version; minEcl?: Ecl; mask?: Mask } = {},
// ) {
//   const encoder = new ByteEncoder(text);
//   const { version, ecl } = findVersionEcl(encoder, options);
//   return buildMatrix(encoder, version, ecl, options.mask);
// }
