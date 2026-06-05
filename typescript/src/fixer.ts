import { MASK_FUNC, NUM_BLOCKS, NUM_CODEWORDS, NUM_EC_CODEWORDS } from "./constants.js";
import { buildGeneratorMatrix, gf256MatrixInvert, gf256Mul } from "./ecc.js";
import { bitIndexMatrix, generateCodewordMatrix, iterateMostlyDataModules } from "./matrix.js";
import type { Stencil } from "./stencil.js";
import type { Ecl, Mask, Version } from "./types.js";

export interface Fixer {
  fits(version: Version, ecl: Ecl, reqCw: number): boolean;
  fixPadding(): void;
  fixInterleaved(): void;
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
//   fixPadding(): void {
//     throw new Error("Method not implemented.");
//   }
//   fixInterleaved(): void {
//     throw new Error("Method not implemented.");
//   }
// }

export class PixelArtFixer implements Fixer {
  stencil: Stencil;

  constructor(stencil: Stencil) {
    this.stencil = stencil;
  }

  fits(version: Version, ecl: Ecl, reqCw: number): boolean {
    const totalCodewords = NUM_CODEWORDS[version];
    const ecCodewords = NUM_EC_CODEWORDS[version][ecl];
    const dataCodewords = totalCodewords - ecCodewords;
    const blocks = NUM_BLOCKS[version][ecl];
    const g1Blocks = blocks - (totalCodewords % blocks);
    const dataPerG1 = Math.floor(dataCodewords / blocks);
    const qrWidth = version * 4 + 17;

    const { matrix, threshold } = generateCodewordMatrix(version, ecl);
    const coveredPerBlock = new Uint16Array(blocks);
    const { data } = this.stencil;
    const seen = new Set<number>();
    for (let y = 0; y < qrWidth; y++) {
      for (let x = 0; x < qrWidth; x++) {
        if (data[y * qrWidth + x] === 0) continue;
        const val = matrix[y * qrWidth + x];
        if (val === 0 || seen.has(val)) continue;
        seen.add(val);
        coveredPerBlock[(val - 1) >> 8]++;
      }
    }

    // Distribute reqCw text codewords across blocks to find T per block.
    const textCwPerBlock = new Uint16Array(blocks);
    for (let c = 0; c < reqCw; c++) {
      const block = c < dataPerG1 * blocks ? c % blocks : g1Blocks + (c - dataPerG1 * blocks);
      textCwPerBlock[block]++;
    }
    for (let b = 0; b < blocks; b++) {
      const dataPerBlock = b < g1Blocks ? dataPerG1 : dataPerG1 + 1;
      if (coveredPerBlock[b] > dataPerBlock - textCwPerBlock[b] + threshold - 3) return false;
    }
    return true;
  }

  fixPadding(): void {
    throw new Error("Method not implemented.");
  }
  fixInterleaved(): void {
    throw new Error("Method not implemented.");
  }

  // byteArray: the full data codeword array already encoded by the caller
  //   (text + terminator + byte-align filled; padding positions still 0).
  // reqCw: number of codewords occupied by text+terminator (boundary between
  //   mandatory text bytes and free padding slots).
  // cwMatrix, threshold: from generateCodewordMatrix(version, ecl) — the same
  //   call the caller already made to build the QR matrix.
  // byteArray: data codewords already encoded by the caller (text+terminator
  //   filled; padding positions still 0). stamp() writes its solved padding
  //   values directly into this array.
  // reqCw: codewords occupied by text+terminator — boundary before free slots.
  // cwMatrix, threshold: from generateCodewordMatrix(version, ecl), which the
  //   caller already computed to build the QR matrix.
  // version, ecl: needed to look up block structure (NUM_BLOCKS, NUM_EC_CODEWORDS).
  stamp(
    matrix: Uint8Array,
    mask: Mask,
    preByteArray: Uint8Array,
    totalM: number,
    // cwMatrix: Uint16Array,
    // threshold: number,
    version: Version,
    ecl: Ecl,
  ): { paddingPerBlock: Uint8Array[]; broken: { index: number; value: number }[] } {
    const qrWidth = version * 4 + 17;
    const totalCodewords = NUM_CODEWORDS[version];
    const ecCodewords = NUM_EC_CODEWORDS[version][ecl];
    const dataCodewords = totalCodewords - ecCodewords;
    const blocks = NUM_BLOCKS[version][ecl];
    const g1Blocks = blocks - (totalCodewords % blocks);
    const dataPerG1 = Math.floor(dataCodewords / blocks);
    const eccPerBlock = ecCodewords / blocks;
    const center = (qrWidth - 1) / 2;

    const finalBitIndex = bitIndexMatrix(version, matrix);

    // const numToBlock = (c: number) => {
    //   return (c % blocks) + (c < dataPerG1 * blocks ? 0 : g1Blocks);
    // };

    // Single traversal in bit order (zigzag, MSB-first per codeword) builds:
    //   packedToModules: packed val → 8 module (x,y) positions in MSB-first order
    //   packedToInterleavedIdx: packed val → interleaved codeword index

    // Text codewords: first reqCw entries in interleaved order.
    // textCwsPerBlock[b] = their packed vals, in block-offset order.
    // packedToByteIdx: packed val → index into byteArray (to read encoded bytes).
    const mPerBlock = new Map<number, number>(
      Array.from({ length: blocks }, (_, index) => [index, 0]),
    );
    // const packedToByteIdx = new Map<number, number>();
    // Reconstruct interleaved-order packed vals for indices 0..reqCw-1
    // using the same block-distribution formula as the encoder.
    // const blockOff = new Uint8Array(blocks);
    for (let c = 0; c < totalM; c++) {
      const block = (c % blocks) + (c < dataPerG1 * blocks ? 0 : g1Blocks);
      // const v = ((block << 8) | blockOff[block]++) + 1;
      mPerBlock.set(block, mPerBlock.get(block)! + 1);
      // mPerBlock[block].push(v);
      // packedToByteIdx.set(v, c);
    }
    // const textValSet = new Set<number>(packedToByteIdx.keys());

    // Collect covered codewords per block with distance to QR center.
    const coveredByBlock: { val: number; dist: number }[][] = Array.from(
      { length: blocks },
      () => [],
    );

    // per block
    // find a differing symbols
    // rank by hamming dist (does mask affect this?)
    // change top p
    // overwrite top floor(r / 2) - 3

    const paddingPerBlock = Array.from({ length: blocks }, () => []);
    const fixPerBlock = Array.from({ length: blocks }, () => []);
    const breakPerBlock = Array.from({ length: blocks }, () => []);

    const { data, width } = this.stencil;

    function countOnes(num: number) {
      let count = 0;
      while (num > 0) {
        if (num & 1) count++;
        num >>= 1;
      }
      return count;
    }

    let targetByteBuffer = 0;
    let targetSetBuffer = 0;
    let bitIdx = 0;
    iterateMostlyDataModules(qrWidth, (x, y) => {
      const posIdx = y * qrWidth + x;
      if (matrix[posIdx] !== 0) return;

      targetByteBuffer <<= 1;
      targetByteBuffer += data[posIdx] & 1;

      targetSetBuffer <<= 1;
      targetSetBuffer += data[posIdx] > 0 ? 1 : 0;

      // byteArray[bitIdx >> 3] & ;
      // ranked padding fixable
      // ranked error fixable...

      if (bitIdx % 8 === 7) {
        const targetByte = targetByteBuffer & 0b1111_1111;
        const targetSet = targetSetBuffer & 0b1111_1111;

        const postByteIdx = bitIdx >> 3;
        if (postByteIdx < dataPerG1 * blocks) {
          const block = postByteIdx % blocks;
          const symbol = Math.floor(postByteIdx / blocks);

          const preByteIdx = block * dataPerG1 + Math.max(0, block - g1Blocks) + symbol;
          const actualByte = preByteArray[preByteIdx];

          const diff = (actualByte ^ targetByte) & targetSet;
          const hamming = countOnes(diff);
          if (preByteIdx < mPerBlock.get(block)!) {
            breakPerBlock[block].push({ val: actualByte ^ diff, dist: hamming });
          } else {
            fixPerBlock[block].push({ val: actualByte ^ diff, dist: hamming });
          }
        } else if (postByteIdx < dataCodewords) {
          const block = (postByteIdx % blocks) + g1Blocks;
          const symbol = dataPerG1;

          const preByteIdx = block * dataPerG1 + (block - g1Blocks) + symbol;
          const actualByte = preByteArray[preByteIdx];

          const diff = (actualByte ^ targetByte) & targetSet;
          const hamming = countOnes(diff);
          if (preByteIdx < mPerBlock.get(block)!) {
            breakPerBlock[block].push({ val: actualByte ^ diff, dist: hamming });
          } else {
            fixPerBlock[block].push({ val: actualByte ^ diff, dist: hamming });
          }
        } else {
          const eccIdx = postByteIdx - dataCodewords;
          const block = eccIdx % blocks;
          const symbol = Math.floor(eccIdx / blocks);

          const hamming = countOnes(targetSet) / 2;

          // ecc is not known, so use *stable* arbitrary value + average hamming
          const val = (0b1010_1010 & ~targetSet) | (targetByte & targetSet);
          fixPerBlock[block].push({ val, dist: hamming });
        }
      }

      bitIdx++;
    });

    // visitAlignmentPatterns()
    // visitTimingPatterns

    // traverseAlign
    for (let y = 0; y < qrWidth; y++) {
      for (let x = 0; x < qrWidth; x++) {
        const idx = y * qrWidth + x;
        if (data[idx] === 0) continue;

        const bitIdx = finalBitIndex[idx];

        if (bitIdx === 0 && !(x === qrWidth - 1 && y === qrWidth - 1)) {
          // functional pattern
        } else {
          const byteIdx = bitIdx >> 3;
          // if (num === 0 || seen.has(num)) continue;
          // seen.add(num);
          const block = (byteIdx % blocks) + (byteIdx < dataPerG1 * blocks ? 0 : g1Blocks);

          const bitPos = 7 - (bitIdx % 8);
        }

        // const val = cwMatrix[y * qrWidth + x];
        // if (val === 0 || seen.has(val)) continue;
        // seen.add(val);
        // coveredByBlock[(val - 1) >> 8].push({ val, dist: Math.hypot(x - center, y - center) });
      }
    }
    for (const arr of coveredByBlock) arr.sort((a, b) => a.dist - b.dist);

    // Pre-apply mask: maskedStencil[y*w+x] = desired stored bit at (x,y).
    // stencil 1 (black) → 1 XOR masker; stencil 2 (white) → 0 XOR masker;
    // stencil 0 (untouched) → 0.
    const masker = MASK_FUNC[mask];
    const maskedStencil = new Uint8Array(qrWidth * qrWidth);
    for (let y = 0; y < qrWidth; y++) {
      for (let x = 0; x < qrWidth; x++) {
        const s = data[y * qrWidth + x];
        if (s === 0) continue;
        maskedStencil[y * qrWidth + x] = (s === 1 ? 1 : 0) ^ (masker(x, y) ? 1 : 0);
      }
    }

    const getDesiredByte = (val: number): number => {
      let byte = 0;
      for (const [mx, my] of packedToModules.get(val)!) {
        byte = (byte << 1) | maskedStencil[my * qrWidth + mx];
      }
      return byte;
    };

    // const paddingPerBlock: Uint8Array[] = [];
    const broken: { index: number; value: number }[] = [];
    const brokenBudget = threshold - 3;

    for (let b = 0; b < blocks; b++) {
      const k = b < g1Blocks ? dataPerG1 : dataPerG1 + 1;
      const T = mPerBlock[b].length;
      const allCov = coveredByBlock[b];

      const kNonTextSet = new Set<number>();
      for (const { val } of allCov) {
        if (kNonTextSet.size >= k - T) break;
        if (!textValSet.has(val)) kNonTextSet.add(val);
      }

      const brokenVals: number[] = [];
      for (const { val } of allCov) {
        if (brokenVals.length >= brokenBudget) break;
        if (!textValSet.has(val) && kNonTextSet.has(val)) continue;
        brokenVals.push(val);
      }

      // Solve: data = Msub⁻¹ × targetVals where Msub[j][i] = G[i][targetCols[j]].
      const targetCols = new Uint8Array(k);
      const targetVals = new Uint8Array(k);
      for (let i = 0; i < T; i++) {
        const v = mPerBlock[b][i];
        targetCols[i] = (v - 1) & 0xff;
        targetVals[i] = preByteArray[packedToByteIdx.get(v)!];
      }
      let filled = T;
      for (const val of kNonTextSet) {
        targetCols[filled] = (val - 1) & 0xff;
        targetVals[filled++] = getDesiredByte(val);
      }
      if (filled < k) {
        const usedCols = new Set<number>(targetCols.subarray(0, filled));
        for (let col = T; col < k && filled < k; col++) {
          if (!usedCols.has(col)) {
            targetCols[filled] = col;
            targetVals[filled++] = 0;
          }
        }
      }

      const G = buildGeneratorMatrix(k, eccPerBlock);
      const Msub: Uint8Array[] = Array.from({ length: k }, (_, j) => {
        const row = new Uint8Array(k);
        for (let i = 0; i < k; i++) row[i] = G[i][targetCols[j]];
        return row;
      });
      const Minv = gf256MatrixInvert(Msub);
      const solvedData = new Uint8Array(k);
      for (let i = 0; i < k; i++) {
        for (let j = 0; j < k; j++) solvedData[i] ^= gf256Mul(Minv[i][j], targetVals[j]);
      }

      // solvedData[T..k-1] = solved padding values for the caller to inject.
      // paddingPerBlock.push(solvedData.slice(T));

      for (const val of brokenVals) {
        broken.push({ index: packedToInterleavedIdx.get(val)!, value: getDesiredByte(val) });
      }
    }

    // return { paddingPerBlock, broken };
  }
}
