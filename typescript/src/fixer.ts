import { MASK_FUNC, NUM_BLOCKS, NUM_CODEWORDS, NUM_EC_CODEWORDS } from "./constants.js";
import { buildGeneratorMatrix, gf256MatrixInvert, gf256Mul } from "./ecc.js";
import { iterateMostlyDataModules } from "./matrix.js";
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
  constructor() {}

  fits(version: Version, ecl: Ecl, reqCw: number): boolean {
    return true;
  }

  fixPadding(): void {
    throw new Error("Method not implemented.");
  }
  fixInterleaved(): void {
    throw new Error("Method not implemented.");
  }

  stamp(
    weightedStencil: Uint8Array,
    matrix: Uint8Array,
    mask: Mask,
    preByteArray: Uint8Array,
    m: number,
    totalM: number,
    // cwMatrix: Uint16Array,
    // threshold: number,
    version: Version,
    ecl: Ecl,
  ) {
    const qrWidth = version * 4 + 17;
    const totalCodewords = NUM_CODEWORDS[version];
    const ecCodewords = NUM_EC_CODEWORDS[version][ecl];
    const dataCodewords = totalCodewords - ecCodewords;
    const blocks = NUM_BLOCKS[version][ecl];
    const g1Blocks = blocks - (totalCodewords % blocks);
    const dataPerG1 = Math.floor(dataCodewords / blocks);
    const eccPerBlock = ecCodewords / blocks;

    const fixPerBlock: { symbol: number; byte: number; weight: number }[][] = Array.from(
      { length: blocks },
      () => [],
    );
    const breakPerBlock: { symbol: number; byte: number; weight: number }[][] = Array.from(
      { length: blocks },
      () => [],
    );

    const mPerBlock = Array.from({ length: blocks }, () => 0);
    const pPerBlock = Array.from({ length: blocks }, () => 0);
    let remainingM = m;
    for (let b = 0; b < blocks; b++) {
      const capacity = b < g1Blocks ? dataPerG1 : dataPerG1 + 1;
      const filled = Math.min(capacity, remainingM);
      remainingM -= filled;
      mPerBlock[b] = filled;
      pPerBlock[b] = capacity - filled;
    }

    const blockVal = Array.from(
      { length: blocks },
      (_, index) =>
        new Uint8Array(index < g1Blocks ? dataPerG1 + eccPerBlock : dataPerG1 + eccPerBlock + 1),
    );

    // visitAlignmentPatterns()
    // visitTimingPatterns

    let bitIdx = 0;
    let targetBuffer = [0, 0, 0, 0, 0, 0, 0, 0];

    const masker = MASK_FUNC[mask];
    iterateMostlyDataModules(qrWidth, (x, y) => {
      const posIdx = y * qrWidth + x;
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
        let preByteIdx;
        let actualByte;

        if (postByteIdx < dataPerG1 * blocks) {
          block = postByteIdx % blocks;
          symbol = Math.floor(postByteIdx / blocks);

          preByteIdx = block * dataPerG1 + Math.max(0, block - g1Blocks) + symbol;
          actualByte = preByteArray[preByteIdx];
        } else if (postByteIdx < dataCodewords) {
          block = (postByteIdx % blocks) + g1Blocks;
          symbol = dataPerG1;

          preByteIdx = block * dataPerG1 + (block - g1Blocks) + symbol;
          actualByte = preByteArray[preByteIdx];
        } else {
          const eccIdx = postByteIdx - dataCodewords;
          block = eccIdx % blocks;
          symbol = Math.floor(eccIdx / blocks) + (block < g1Blocks ? dataPerG1 : dataPerG1 + 1);
          actualByte = 0b1010_1010;
        }

        // TODO don't need ecc values
        blockVal[block][symbol] = actualByte;

        let byte = actualByte;
        let weight = 0;
        for (let i = 0; i < 8; i++) {
          const target = targetBuffer[i];
          const targetWeight = target >> 1;
          if (targetWeight === 0) continue;
          const targetBit = target & 1;
          const actualBit = (actualByte >> i) & 1;
          if (targetBit === actualBit) continue;
          weight += targetWeight;
          byte ^= 1 << i;
        }

        const fixable = preByteIdx == null || symbol >= mPerBlock[block];

        if (fixable) {
          fixPerBlock[block].push({ symbol, byte, weight });
        } else {
          breakPerBlock[block].push({ symbol, byte, weight });
        }
      }

      bitIdx++;
    });

    const breakLimit = Math.floor(eccPerBlock / 2) - 3;
    for (let i = 0; i < blocks; i++) {
      const fixable = pPerBlock[i];
      const fixOptions = fixPerBlock[i];
      const breakOptions = breakPerBlock[i];

      if (fixOptions.length > fixable) {
        fixOptions.sort((a, b) => b.weight - a.weight);
        fixPerBlock[i] = fixOptions.slice(0, fixable);
        breakOptions.push(...fixOptions.slice(fixable));
      }

      if (breakOptions.length > breakLimit) {
        breakOptions.sort((a, b) => b.weight - a.weight);
      }
    }

    const broken: { index: number; value: number }[] = [];
    const brokenBudget = Math.floor(eccPerBlock / 2) - 3;

    let G = buildGeneratorMatrix(dataPerG1, eccPerBlock);
    for (let b = 0; b < blocks; b++) {
      if (b === g1Blocks) {
        G = buildGeneratorMatrix(dataPerG1 + 1, eccPerBlock);
      }

      const k = b < g1Blocks ? dataPerG1 : dataPerG1 + 1;

      const p = pPerBlock[b];
      if (p === 0) continue;
      const fixOptions = fixPerBlock[b];
      if (fixOptions.length === 0) continue;
      const m = mPerBlock[b];

      const targetCols = new Uint8Array(k);
      const targetVals = new Uint8Array(k);

      for (let i = 0; i < k; i++) {
        targetCols[i] = i;
        targetVals[i] = blockVal[b][i];
      }

      const fixes = Math.min(p, fixOptions.length);
      const dataSet = new Set<number>();
      for (let i = 0; i < fixes; i++) {
        const { symbol, byte } = fixOptions[i];
        if (symbol < k) {
          targetVals[symbol] = byte;
          dataSet.add(symbol);
        }
      }
      let eccSlot = m;
      for (let i = 0; i < fixes; i++) {
        const { symbol, byte } = fixOptions[i];
        if (symbol >= k) {
          while (eccSlot < k && dataSet.has(eccSlot)) eccSlot++;
          if (eccSlot >= k) break;
          targetCols[eccSlot] = symbol;
          targetVals[eccSlot] = byte;
          eccSlot++;
        }
      }

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
    }
  }
}
