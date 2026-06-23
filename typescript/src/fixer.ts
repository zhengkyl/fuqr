import { MASK_FUNC, NUM_BLOCKS, NUM_BYTES, NUM_EC_BYTES } from "./constants.ts";
import { buildGeneratorMatrix, generatorPolynomial, gf256Solve, remainder } from "./ecc.ts";
import type { Encoder } from "./encoder.ts";
import { iterateMostlyDataModules, visitAlignmentPatterns, visitTimingPatterns } from "./matrix.ts";
import { Module, type Ecl, type Mask, type Version } from "./types.ts";

// Consistent variable names
//
// A group of eight QR data modules is a byte/symbol/codeword. Prefer "byte".
// Message and error correction bytes make up a block/codeword. Prefer "block".
// Error correction can be shortened to "ec".
//
// Data refers to the collective blocks which make up the entire QR data section.
// |--------------------- data ----------------------|
// |---------------- block ----------------| ...etc
// |---------- message ----------|-- ec --|
// |--- content ---|-- padding --|
//
// Content consists of one or more sections with a header and binary data.
// The last content section in a message ends with a terminator, space permitting.

export interface Fixer {
  // Called once content fits at (version, ecl). May raise version/ecl as needed.
  fixVersionEcl(version: Version, ecl: Ecl, encoder: Encoder): { version: Version; ecl: Ecl };
  // Mutates the function pattern template matrix before data placement.
  fixMatrix(matrix: Uint8Array, version: Version): void;
  // Mutates messageBytes, preserving the first c content bytes.
  // Stores any deliberately broken bytes for fixInterleaved.
  fixMessage(
    messageBytes: Uint8Array,
    c: number,
    matrix: Uint8Array,
    version: Version,
    ecl: Ecl,
    mask: Mask,
  ): void;
  // Mutates the final interleaved bytes, applying breaks within the ec budget.
  fixInterleaved(interleaved: Uint8Array): void;
}

// Forced bits of the interleaved byte at index.
type Break = { index: number; mask: number; value: number };

export class NoopFixer implements Fixer {
  fixVersionEcl(version: Version, ecl: Ecl) {
    return { version, ecl };
  }
  fixMatrix() {}
  fixMessage() {}
  fixInterleaved() {}
}

// export class LogoFixer implements Fixer {
//   stencil: Stencil;
//   size: number;
//   naturalWidth: number;
//   naturalHeight: number;
//   gap: number;

//   constructor(logo: Logo, size: number, gap: number = 1) {
//     this.stencil = logo.stencil;
//     this.size = size;
//     this.naturalWidth = logo.naturalWidth;
//     this.naturalHeight = logo.naturalHeight;
//     this.gap = gap;
//   }
//   fits(version: Version, ecl: Ecl, reqCw: number): boolean {
//     const qrWidth = version * 4 + 17;
//     const logoWidth = Math.round(this.size * qrWidth);
//     const logoHeight = Math.round((logoWidth * this.naturalHeight) / this.naturalWidth);
//     const x0 = Math.round((qrWidth - logoWidth) / 2);
//     const y0 = Math.round((qrWidth - logoHeight) / 2);

//     const { matrix, threshold } = generateCodewordMatrix(version, ecl);
//     const blocks = NUM_BLOCKS[version][ecl];
//     const blockErrors = new Uint16Array(blocks);
//     const seen = new Set<number>();
//     const coverage = resizeStencil(this.stencil, logoWidth, logoHeight).data;
//     const gap = this.gap;

//     for (let y = Math.max(0, y0 - gap); y < Math.min(qrWidth, y0 + logoHeight + gap); y++) {
//       for (let x = Math.max(0, x0 - gap); x < Math.min(qrWidth, x0 + logoWidth + gap); x++) {
//         const lx = x - x0,
//           ly = y - y0;
//         const cxMin = Math.max(0, lx - gap),
//           cxMax = Math.min(logoWidth - 1, lx + gap);
//         const cyMin = Math.max(0, ly - gap),
//           cyMax = Math.min(logoHeight - 1, ly + gap);
//         if (cxMin > cxMax || cyMin > cyMax) continue;
//         let isCovered = false;
//         outerCheck: for (let cy = cyMin; cy <= cyMax; cy++) {
//           for (let cx = cxMin; cx <= cxMax; cx++) {
//             if (coverage[cy * logoWidth + cx]) {
//               isCovered = true;
//               break outerCheck;
//             }
//           }
//         }
//         if (!isCovered) continue;
//         const val = matrix[y * qrWidth + x];
//         if (val === 0 || seen.has(val)) continue;
//         seen.add(val);
//         blockErrors[(val - 1) >> 8]++;
//       }
//     }

//     for (let b = 0; b < blocks; b++) {
//       if (blockErrors[b] > threshold - 3) return false;
//     }
//     return true;
//   }
// }

// Each weightedStencil value is (weight << 1) | bit, indexed by module position.
// Weight 0 means don't care. Data module bits are pre-mask, function module bits are final.
export class PixelArtFixer implements Fixer {
  weightedStencil: Uint8Array;
  breaks: Break[] = [];

  constructor(weightedStencil: Uint8Array) {
    this.weightedStencil = weightedStencil;
  }

  fixVersionEcl(version: Version, ecl: Ecl) {
    return { version, ecl };
  }

  // draw over timing and all alignment patterns except the only used (bottom right)
  fixMatrix(matrix: Uint8Array, version: Version) {
    const weightedStencil = this.weightedStencil;
    const width = version * 4 + 17;
    const override = (x: number, y: number) => {
      const posIdx = y * width + x;
      const stencilVal = weightedStencil[posIdx];
      if (stencilVal >> 1 === 0) return;
      matrix[posIdx] = (matrix[posIdx] & ~Module.ON) | (stencilVal & Module.ON);
    };
    visitTimingPatterns(width, override);

    const sparedStart = width - 9;
    visitAlignmentPatterns(version, width, (x: number, y: number) => {
      if (x >= sparedStart && y >= sparedStart) return;
      override(x, y);
    });
  }

  fixMessage(
    messageBytes: Uint8Array,
    contentBytes: number,
    matrix: Uint8Array,
    version: Version,
    ecl: Ecl,
    mask: Mask,
  ) {
    const numBytes = NUM_BYTES[version];
    const numEcBytes = NUM_EC_BYTES[version][ecl];
    const numMessageBytes = numBytes - numEcBytes;
    const numBlocks = NUM_BLOCKS[version][ecl];
    const numG1Blocks = numBlocks - (numBytes % numBlocks);
    const messagePerG1 = Math.floor(numMessageBytes / numBlocks);
    const ecPerBlock = numEcBytes / numBlocks;

    const blockStart = (b: number) => b * messagePerG1 + Math.max(0, b - numG1Blocks);

    const cPerBlock = Array.from({ length: numBlocks }, () => 0);
    const pPerBlock = Array.from({ length: numBlocks }, () => 0);
    let remainingC = contentBytes;
    for (let b = 0; b < numBlocks; b++) {
      const capacity = b < numG1Blocks ? messagePerG1 : messagePerG1 + 1;
      const filled = Math.min(capacity, remainingC);
      remainingC -= filled;
      cPerBlock[b] = filled;
      pPerBlock[b] = capacity - filled;
    }

    // byte is the solve target, mask/value are the forced bits, weight is the
    // total weight of forced bits which don't already match.
    type Option = Break & { symbol: number; byte: number; weight: number };
    const fixPerBlock: Option[][] = Array.from({ length: numBlocks }, () => []);
    const breakPerBlock: Option[][] = Array.from({ length: numBlocks }, () => []);

    let bitIdx = 0;
    const targetBuffer = [0, 0, 0, 0, 0, 0, 0, 0];

    const weightedStencil = this.weightedStencil;
    const masker = MASK_FUNC[mask];
    const width = version * 4 + 17;
    iterateMostlyDataModules(width, (x, y) => {
      const posIdx = y * width + x;
      if (matrix[posIdx] !== 0) return;

      let stencilVal = weightedStencil[posIdx];
      if (stencilVal > 0) {
        stencilVal ^= +masker(x, y);
      }
      // msb order
      const msbBitPos = 7 - (bitIdx % 8);
      targetBuffer[msbBitPos] = stencilVal;

      const postByteIdx = bitIdx >> 3;
      if (msbBitPos === 0) {
        let block;
        let symbol;
        let isContent = false;
        let actualByte;

        if (postByteIdx < messagePerG1 * numBlocks) {
          block = postByteIdx % numBlocks;
          symbol = Math.floor(postByteIdx / numBlocks);
          actualByte = messageBytes[blockStart(block) + symbol];
          isContent = symbol < cPerBlock[block];
        } else if (postByteIdx < numMessageBytes) {
          block = (postByteIdx % numBlocks) + numG1Blocks;
          symbol = messagePerG1;
          actualByte = messageBytes[blockStart(block) + symbol];
          isContent = symbol < cPerBlock[block];
        } else {
          const ecIdx = postByteIdx - numMessageBytes;
          block = ecIdx % numBlocks;
          symbol =
            Math.floor(ecIdx / numBlocks) + (block < numG1Blocks ? messagePerG1 : messagePerG1 + 1);
          // ec bytes aren't known yet, only their forced bits matter
          actualByte = 0b1010_1010;
        }

        let forcedMask = 0;
        let forcedValue = 0;
        let weight = 0;
        for (let i = 0; i < 8; i++) {
          const target = targetBuffer[i];
          const targetWeight = target >> 1;
          if (targetWeight === 0) continue;
          forcedMask |= 1 << i;
          forcedValue |= (target & 1) << i;
          if (((actualByte >> i) & 1) !== (target & 1)) weight += targetWeight;
        }
        if (forcedMask !== 0) {
          const option = {
            symbol,
            index: postByteIdx,
            byte: (actualByte & ~forcedMask) | forcedValue,
            mask: forcedMask,
            value: forcedValue,
            weight,
          };
          if (isContent) {
            breakPerBlock[block].push(option);
          } else {
            fixPerBlock[block].push(option);
          }
        }
      }

      bitIdx++;
    });

    const breaks: Break[] = [];
    const breakLimit = Math.floor(ecPerBlock / 2) - 3;
    for (let b = 0; b < numBlocks; b++) {
      const fixable = pPerBlock[b];
      const fixOptions = fixPerBlock[b];
      const breakOptions = breakPerBlock[b];

      if (fixOptions.length > fixable) {
        fixOptions.sort((a, b) => b.weight - a.weight);
        breakOptions.push(...fixOptions.slice(fixable));
        fixOptions.length = fixable;
      }

      if (breakOptions.length > breakLimit) {
        breakOptions.sort((a, b) => b.weight - a.weight);
        breakOptions.length = Math.max(0, breakLimit);
      }
      for (const { index, mask, value } of breakOptions) {
        breaks.push({ index, mask, value });
      }
    }
    this.breaks = breaks;

    // A fix on a data column directly sets that byte, since those codeword
    // bytes are systematic. A fix on an ec column couples the unknown padding,
    // so only those require solving. G is systematic ([I | P]), so the system
    // collapses to one equation and one unknown padding byte per ec fix.
    const divisor = generatorPolynomial(ecPerBlock);
    let G = buildGeneratorMatrix(messagePerG1, ecPerBlock);
    for (let b = 0; b < numBlocks; b++) {
      if (b === numG1Blocks) {
        G = buildGeneratorMatrix(messagePerG1 + 1, ecPerBlock);
      }

      const p = pPerBlock[b];
      if (p === 0) continue;

      const fixOptions = fixPerBlock[b];
      if (fixOptions.length === 0) continue;

      const m = cPerBlock[b];
      const start = blockStart(b);

      const k = b < numG1Blocks ? messagePerG1 : messagePerG1 + 1;

      // Apply data column fixes directly, collect ec column fixes to solve.
      const ecFixes: Option[] = [];
      const taken = new Uint8Array(p);
      for (const option of fixOptions) {
        if (option.symbol < k) {
          messageBytes[start + option.symbol] = option.byte;
          taken[option.symbol - m] = 1;
        } else {
          ecFixes.push(option);
        }
      }

      const t = ecFixes.length;
      if (t === 0) continue;

      // Leave one free padding byte unknown per ec fix, the rest keep their
      // current value. Zero the unknowns so they drop out of the known parity.
      const knownData = messageBytes.slice(start, start + k);
      const unknownCols = new Uint8Array(t);
      let padding = 0;
      for (let i = 0; i < t; i++) {
        while (taken[padding]) padding++;
        unknownCols[i] = m + padding;
        knownData[m + padding] = 0;
        padding++;
      }

      // Each ec fix must supply the parity the known data doesn't already give.
      const knownEc = remainder(knownData, divisor);
      const rhs = new Uint8Array(t);
      for (let j = 0; j < t; j++) {
        rhs[j] = ecFixes[j].byte ^ knownEc[ecFixes[j].symbol - k];
      }

      // A[j][i] is unknown column i's contribution to ec fix j's parity byte.
      const A: Uint8Array[] = Array.from({ length: t }, (_, j) => {
        const row = new Uint8Array(t);
        for (let i = 0; i < t; i++) row[i] = G[unknownCols[i]][ecFixes[j].symbol];
        return row;
      });
      const solved = gf256Solve(A, rhs);
      for (let i = 0; i < t; i++) messageBytes[start + unknownCols[i]] = solved[i];
    }
  }

  // overwrite intentional broken bytes
  fixInterleaved(interleaved: Uint8Array) {
    for (const { index, mask, value } of this.breaks) {
      interleaved[index] = (interleaved[index] & ~mask) | value;
    }
  }
}
