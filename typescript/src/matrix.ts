import {
  ALIGN_OFFSETS,
  FORMAT_INFO,
  MASK_FUNC,
  NUM_BLOCKS,
  NUM_BYTES,
  NUM_DATA_MODULES,
  NUM_EC_BYTES,
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
  const numModules = NUM_DATA_MODULES[version];
  const numBytes = Math.floor(numModules / 8);
  const remainderBits = numModules % 8;
  const numEcBytes = NUM_EC_BYTES[version][ecl];
  const numMessageBytes = numBytes - numEcBytes;

  const messageBytes = new Uint8Array(numMessageBytes);
  let buf = 0,
    bufLen = 0,
    bytePos = 0;
  const push = (value: number, n: number) => {
    buf = (buf << n) | value;
    bufLen += n;
    while (bufLen >= 8) {
      bufLen -= 8;
      messageBytes[bytePos++] = (buf >> bufLen) & 0xff;
    }
  };

  encoder.encode(version, push);

  const remainingDataBits = numMessageBytes * 8 - bytePos * 8 - bufLen;
  push(0, remainingDataBits < 4 ? remainingDataBits : 4);
  push(0, (8 - bufLen) & 7);

  const c = bytePos;
  let alternating = 0b1110_1100;
  for (let i = bytePos; i < numMessageBytes; i++) {
    push(alternating, 8);
    alternating ^= 0b1111_1101;
  }

  const width = version * 4 + 17;
  const matrix = new Uint8Array(width * width);
  const set = (x: number, y: number, value: number) => (matrix[y * width + x] |= value);
  visitFinderPatterns(width, set);
  visitTimingPatterns(width, set);
  visitFormatInfo(ecl, mask, width, set);
  visitAlignmentPatterns(version, width, set);
  visitVersionInfo(version, width, set);

  fixer.fixMatrix(matrix, version);
  fixer.fixMessage(messageBytes, c, matrix, version, ecl, mask);

  const blocks = NUM_BLOCKS[version][ecl];
  const g2Blocks = numBytes % blocks;
  const g1Blocks = blocks - g2Blocks;
  const messagePerG1 = Math.floor(numMessageBytes / blocks);
  const messagePerG2 = messagePerG1 + 1;
  const ecPerBlock = numEcBytes / blocks;

  const interleaved = new Uint8Array(numBytes + (remainderBits > 0 ? 1 : 0));

  const numG1 = g1Blocks * messagePerG1;
  for (let i = 0; i < numMessageBytes; i++) {
    const col = Math.floor(i / blocks);
    const row = i % blocks;
    if (col < messagePerG1) {
      interleaved[i] = messageBytes[row * messagePerG1 + col + Math.max(0, row - g1Blocks)];
    } else {
      interleaved[i] = messageBytes[numG1 + row * messagePerG2 + col];
    }
  }

  const divisor = generatorPolynomial(ecPerBlock);
  for (let i = 0; i < g1Blocks; i++) {
    const ec = remainder(messageBytes.subarray(i * messagePerG1, (i + 1) * messagePerG1), divisor);
    for (let j = 0; j < ecPerBlock; j++) interleaved[numMessageBytes + j * blocks + i] = ec[j];
  }
  const g2Start = numG1;
  for (let i = 0; i < g2Blocks; i++) {
    const ec = remainder(
      messageBytes.subarray(g2Start + i * messagePerG2, g2Start + (i + 1) * messagePerG2),
      divisor,
    );
    for (let j = 0; j < ecPerBlock; j++)
      interleaved[numMessageBytes + j * blocks + i + g1Blocks] = ec[j];
  }

  fixer.fixInterleaved(interleaved);

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

// 0 is function OR first bit
export function bitIndexMatrix(version: number, templateMatrix: Uint8Array) {
  const symbolNumMatrix = new Uint16Array(templateMatrix.length);
  const totalCodewords = NUM_BYTES[version];
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

export function iterateMostlyDataModules(width: number, callback: (x: number, y: number) => void) {
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
) {
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
) {
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
) {
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
) {
  if (version < 7) return;
  const versionInfo = VERSION_INFO[version - 7];
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
) {
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
