import {
  ALIGN_OFFSETS,
  FORMAT_INFO,
  MASK_FUNC,
  NUM_BLOCKS,
  NUM_CODEWORDS,
  NUM_DATA_MODULES,
  NUM_EC_CODEWORDS,
  VERSION_INFO,
} from "./constants.js";
import { generatorPolynomial, remainder } from "./ecc.js";
import type { Encoder } from "./encoder.js";
import type { Fixer } from "./fixer.js";
import { Module, type Ecl, type Mask, type Version } from "./types.js";

export function buildMatrix(
  encoder: Encoder,
  fixer: Fixer,
  version: Version,
  ecl: Ecl,
  mask: Mask = 2,
): { matrix: Uint8Array; version: Version; ecl: Ecl; mask: Mask } {
  const modules = NUM_DATA_MODULES[version];
  const codewords = Math.floor(modules / 8);
  const remainderBits = modules % 8;
  const ecCodewords = NUM_EC_CODEWORDS[version][ecl];
  const dataCodewords = codewords - ecCodewords;

  const byteArray = new Uint8Array(dataCodewords);
  let buf = 0,
    bufLen = 0,
    bytePos = 0;
  const push = (value: number, n: number) => {
    buf = (buf << n) | value;
    bufLen += n;
    while (bufLen >= 8) {
      bufLen -= 8;
      byteArray[bytePos++] = (buf >> bufLen) & 0xff;
    }
  };

  encoder.encode(version, push);

  const remainingDataBits = dataCodewords * 8 - bytePos * 8 - bufLen;
  push(0, remainingDataBits < 4 ? remainingDataBits : 4);
  push(0, (8 - bufLen) & 7);

  let alternating = 0b1110_1100;
  for (let i = bytePos; i < dataCodewords; i++) {
    push(alternating, 8);
    alternating ^= 0b1111_1101;
  }

  const stencil = new Uint8Array();

  const width = version * 4 + 17;
  const matrix = new Uint8Array(width * width);
  const set = (x: number, y: number, value: number) => (matrix[y * width + x] |= value);
  visitFinderPatterns(width, set);
  visitTimingPatterns(width, set);
  visitFormatInfo(ecl, mask, width, set);
  visitAlignmentPatterns(version, width, set);
  visitVersionInfo(version, width, set);

  const symbolNum = bitIndexMatrix(version, matrix);
  // const numToBlock = () => {

  // }

  // TODO
  fixer.fixPadding();

  const blocks = NUM_BLOCKS[version][ecl];
  const g2Blocks = codewords % blocks;
  const g1Blocks = blocks - g2Blocks;
  const dataPerG1Block = Math.floor(dataCodewords / blocks);
  const dataPerG2Block = dataPerG1Block + 1;
  const eccPerBlock = ecCodewords / blocks;

  const numToBlock = (c: number) => {
    return (c % blocks) + (c < dataPerG1Block * blocks ? 0 : g1Blocks);
  };

  // for (let y = 0; y < stencil)

  const interleaved = new Uint8Array(codewords + (remainderBits > 0 ? 1 : 0));

  const numG1 = g1Blocks * dataPerG1Block;
  for (let i = 0; i < numG1; i++) {
    interleaved[(i % dataPerG1Block) * blocks + Math.floor(i / dataPerG1Block)] = byteArray[i];
  }
  const numG2 = g2Blocks * dataPerG2Block;
  for (let i = 0; i < numG2; i++) {
    const col = i % dataPerG2Block;
    const row = Math.floor(i / dataPerG2Block);
    interleaved[col * blocks + row + (col === dataPerG2Block - 1 ? 0 : g1Blocks)] =
      byteArray[i + numG1];
  }

  const divisor = generatorPolynomial(eccPerBlock);
  for (let i = 0; i < g1Blocks; i++) {
    const ec = remainder(byteArray.subarray(i * dataPerG1Block, (i + 1) * dataPerG1Block), divisor);
    for (let j = 0; j < eccPerBlock; j++) interleaved[dataCodewords + j * blocks + i] = ec[j];
  }
  const g2Start = numG1;
  for (let i = 0; i < g2Blocks; i++) {
    const ec = remainder(
      byteArray.subarray(g2Start + i * dataPerG2Block, g2Start + (i + 1) * dataPerG2Block),
      divisor,
    );
    for (let j = 0; j < eccPerBlock; j++)
      interleaved[dataCodewords + j * blocks + i + g1Blocks] = ec[j];
  }

  // TODO
  fixer.fixInterleaved();

  // const width = version * 4 + 17;
  // const matrix = new Uint8Array(width * width);
  // const set = (x: number, y: number, value: number) => (matrix[y * width + x] |= value);
  // _buildFunctionPatternMatrix(version, ecl, mask, set);

  let bitIdx = 0;
  const masker = MASK_FUNC[mask];
  iterateMostlyDataModules(width, (x, y) => {
    if (matrix[y * width + x] === 0) {
      const bit = (interleaved[bitIdx >> 3] >> (7 - (bitIdx % 8))) & Module.ON;
      set(x, y, Module.DATA | (bit ^ +masker(x, y)));
      bitIdx++;
    }
  });

  return { matrix, version, ecl, mask };
}

// Returns a Uint16Array where each data module stores (block << 8 | offset) + 1.
// 0 = non-data / remainder bit.
// Also returns the per-block error correction threshold (max correctable errors).
// export function generateCodewordMatrix(
//   version: Version,
//   ecl: Ecl,
// ): { matrix: Uint16Array; threshold: number } {
//   const totalCodewords = NUM_CODEWORDS[version];
//   const ecCodewords = NUM_EC_CODEWORDS[version][ecl];
//   const dataCodewords = totalCodewords - ecCodewords;
//   const blocks = NUM_BLOCKS[version][ecl];
//   const g1Blocks = blocks - (totalCodewords % blocks);
//   const dataPerG1 = Math.floor(dataCodewords / blocks);
//   const eccPerBlock = ecCodewords / blocks;

//   const blockOffset = new Uint8Array(blocks);
//   const cwToPackedU16 = new Uint16Array(totalCodewords);
//   for (let c = 0; c < dataCodewords; c++) {
//     const block = c < dataPerG1 * blocks ? c % blocks : g1Blocks + (c - dataPerG1 * blocks);
//     cwToPackedU16[c] = ((block << 8) | blockOffset[block]++) + 1;
//   }
//   for (let c = dataCodewords; c < totalCodewords; c++) {
//     const block = (c - dataCodewords) % blocks;
//     cwToPackedU16[c] = ((block << 8) | blockOffset[block]++) + 1;
//   }

//   const width = version * 4 + 17;
//   const matrix = new Uint16Array(width * width);
//   const markReserved = (x: number, y: number, _v: number) => {
//     matrix[y * width + x] = 0xffff;
//   };
//   traverseFinderPatterns(width, markReserved);
//   traverseAlignmentPatterns(version, width, markReserved);
//   traverseTimingPatterns(width, markReserved);
//   traverseVersionInfo(version, width, markReserved);
//   traverseFormatInfo(width, 0, 0, markReserved);

//   let bitIdx = 0;
//   traverseDataBits(width, (x, y) => {
//     if (matrix[y * width + x] > 0) return;
//     if (bitIdx < totalCodewords * 8) matrix[y * width + x] = cwToPackedU16[bitIdx >> 3];
//     bitIdx++;
//   });

//   for (let i = 0; i < matrix.length; i++) if (matrix[i] === 0xffff) matrix[i] = 0;

//   return { matrix, threshold: eccPerBlock >> 1 };
// }

// 0 is function OR first bit
export function bitIndexMatrix(version: number, templateMatrix: Uint8Array) {
  const symbolNumMatrix = new Uint16Array(templateMatrix.length);
  const totalCodewords = NUM_CODEWORDS[version];
  const width = version * 4 + 17;
  let bitIndex = 0;
  iterateMostlyDataModules(width, (x, y) => {
    if (templateMatrix[y * width + x] > 0) return;
    if (bitIndex < totalCodewords * 8) {
      symbolNumMatrix[y * width + x] = bitIndex;
    }
    bitIndex++;
  });
  return symbolNumMatrix;
}

export function iterateMostlyDataModules(
  width: number,
  callback: (x: number, y: number) => void,
): void {
  let y = width - 1;
  let dy = -1;
  let rowEnd = 9;
  let rowSteps = width - 10;
  for (let x = width - 1; x > 0; x -= 2) {
    while (true) {
      callback(x, y);
      callback(x - 1, y);
      if (y === rowEnd) break;
      y += dy;
    }
    dy *= -1;
    switch (x) {
      case width - 7:
        rowSteps = width - 1;
        rowEnd = 0;
        break;
      case 10:
        rowSteps = width - 18;
        rowEnd = 9;
        y = width - 9;
        break;
      case 8:
        x--;
      // fallthrough
      default:
        rowEnd += dy * rowSteps;
    }
  }
}

export function visitFinderPatterns(
  width: number,
  set: (x: number, y: number, value: number) => void,
): void {
  const setFinder = (x: number, y: number) => {
    for (let dy = -3; dy <= 3; dy++) {
      for (let dx = -3; dx <= 3; dx++) {
        const max = Math.max(Math.abs(dy), Math.abs(dx));
        set(
          x + dx,
          y + dy,
          max === 3
            ? Module.FINDER | Module.ON
            : max === 2
              ? Module.FINDER
              : Module.FINDER | Module.MODIFIER | Module.ON,
        );
      }
    }
  };
  setFinder(3, 3);
  setFinder(3, width - 4);
  setFinder(width - 4, 3);
}

export function visitAlignmentPatterns(
  version: number,
  width: number,
  set: (x: number, y: number, value: number) => void,
): void {
  if (version < 2) return;
  const setAlignment = (cx: number, cy: number) => {
    for (let dy = -2; dy <= 2; dy++) {
      for (let dx = -2; dx <= 2; dx++) {
        const max = Math.max(Math.abs(dy), Math.abs(dx));
        set(
          cx + dx,
          cy + dy,
          max === 2
            ? Module.ALIGNMENT | Module.ON
            : max === 1
              ? Module.ALIGNMENT
              : Module.ALIGNMENT | Module.MODIFIER | Module.ON,
        );
      }
    }
  };
  const first = 6;
  const last = width - 7;
  const len = Math.floor(version / 7) + 2;
  const coords: number[] = [first];
  if (version >= 7) {
    for (let i = len - 2; i >= 1; i--) coords.push(last - i * ALIGN_OFFSETS[version - 7]);
  }
  coords.push(last);
  for (let i = 0; i < len; i++) {
    for (let j = 0; j < len; j++) {
      if ((i === 0 && (j === 0 || j === len - 1)) || (i === len - 1 && j === 0)) continue;
      setAlignment(coords[i], coords[j]);
    }
  }
}

export function visitTimingPatterns(
  width: number,
  set: (x: number, y: number, value: number) => void,
): void {
  for (let i = 8; i < width - 8; i++) {
    const module = Module.TIMING | ((i & 1) ^ Module.ON);
    set(6, i, module);
    set(i, 6, module);
  }
}

export function visitVersionInfo(
  version: number,
  width: number,
  set: (x: number, y: number, value: number) => void,
): void {
  if (version < 7) return;
  const versionInfo = VERSION_INFO[version];
  for (let i = 0; i < 6; i++) {
    for (let j = 0; j < 3; j++) {
      const bit = (versionInfo >> (i * 3 + j)) & Module.ON;
      set(width - 11 + j, i, Module.VERSION | bit);
      set(i, width - 11 + j, Module.VERSION | Module.MODIFIER | bit);
    }
  }
}

export function visitFormatInfo(
  ecl: Ecl,
  mask: Mask,
  width: number,
  set: (x: number, y: number, value: number) => void,
): void {
  const formatInfo = FORMAT_INFO[ecl][mask];
  for (let i = 0; i < 6; i++) set(8, i, Module.FORMAT | ((formatInfo >> i) & Module.ON));
  set(8, 7, Module.FORMAT | ((formatInfo >> 6) & Module.ON));
  set(8, 8, Module.FORMAT | ((formatInfo >> 7) & Module.ON));
  set(7, 8, Module.FORMAT | ((formatInfo >> 8) & Module.ON));
  for (let i = 9; i < 15; i++) set(14 - i, 8, Module.FORMAT | ((formatInfo >> i) & Module.ON));

  const formatCopy = Module.FORMAT | Module.MODIFIER;
  for (let i = 0; i < 8; i++) set(width - 1 - i, 8, formatCopy | ((formatInfo >> i) & Module.ON));
  for (let i = 8; i < 15; i++) set(8, width - 15 + i, formatCopy | ((formatInfo >> i) & Module.ON));
  set(8, width - 8, formatCopy | Module.ON);
}
