export function generate(
  text: string,
  options: {
    minVersion?: Version;
    maxVersion?: Version;
    minEcl?: Ecl;
    maxEcl?: Ecl;
    mask?: Mask;
  } = {},
  plugins: Plugin[] = [],
) {
  const encoder = new ByteEncoder(text);
  const { minVersion = 1, maxVersion = 40, minEcl = 0, maxEcl = 3, mask = 2 } = options;

  const { version, ecl } = findCapacity(
    encoder,
    { minVersion, maxVersion, minEcl, maxEcl },
    plugins,
  );
  return buildMatrix(encoder, { version, ecl, mask }, plugins);
}

// A black or white QR square is a bit (sometimes module/pixel).
// A group of eight QR bits is a byte (sometimes symbol/codeword).
// Message and error correction bytes make up a block (sometimes codeword).
// Blocks are interleaved and fill up the QR data section.
//
// Here is the hierarchy of vocabulary
// |--------------------- data ----------------------|
// |---------------- block ---------------|  ...etc
// |---------- message ----------|-- ec --|
// |--- content ---|-- padding --|

export type Version = number; // 1 to 40 inclusive
export type Ecl = 0 | 1 | 2 | 3;
export type Mask = 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7;
export const Module = {
  ON: 1 << 0,
  DATA: 1 << 1,
  FINDER: 1 << 2,
  ALIGNMENT: 1 << 3,
  TIMING: 1 << 4,
  FORMAT: 1 << 5,
  VERSION: 1 << 6,
  MODIFIER: 1 << 7,
};
export class FvqrError extends Error {
  code: string;
  constructor(code: string, message: string) {
    super(message);
    this.name = "FvqrError";
    this.code = code;
  }
}

export interface Encoder {
  bitLen(version: number): number;
  encode(version: number, push: (bits: number, len: number) => void): void;
}

export class ByteEncoder implements Encoder {
  public bytes: Uint8Array;
  constructor(text: string) {
    this.bytes = new TextEncoder().encode(text);
  }
  bitLen(version: number) {
    const cci = version < 10 ? 8 : 16;
    return 4 + cci + this.bytes.length * 8;
  }
  encode(version: number, push: (bits: number, len: number) => void) {
    const cci = version < 10 ? 8 : 16;
    const bytes = this.bytes;

    push(0b0100, 4);
    push(bytes.length, cci);
    for (const b of bytes) {
      push(b, 8);
    }
  }
}

export function findCapacity(
  encoder: Encoder,
  options: { minVersion: Version; maxVersion: Version; minEcl: Ecl; maxEcl: Ecl },
  plugins: Plugin[],
): { version: Version; ecl: Ecl } {
  const { minVersion, maxVersion, minEcl, maxEcl } = options;

  for (let version = minVersion; version <= maxVersion; version++) {
    const reqBytes = Math.ceil(encoder.bitLen(version) / 8);
    const dataBytes = NUM_DATA_BITS[version] >> 3;

    if (reqBytes <= dataBytes - NUM_EC_BYTES[version][minEcl]) {
      let ecl = minEcl;
      while (ecl < maxEcl && reqBytes <= dataBytes - NUM_EC_BYTES[version][ecl + 1]) {
        ecl++;
      }

      const capacity = { version, ecl };
      plugins.forEach((p) => p.mutateCapacity(capacity, encoder));
      return capacity;
    }
  }

  throw new FvqrError("TEXT_TOO_LONG", `Cannot fit in version ${maxVersion}`);
}

export interface Plugin {
  mutateCapacity(capacity: { version: Version; ecl: Ecl }, encoder: Encoder): void;
  mutateMessage(
    messageBytes: Uint8Array,
    contentLen: number,
    meta: {
      matrix: Uint8Array;
      version: Version;
      ecl: Ecl;
      mask: Mask;
    },
  ): void;
  mutateSequence(interleaved: Uint8Array): void;
  mutateMatrix(matrix: Uint8Array, version: Version): void;
}

export function buildMatrix(
  encoder: Encoder,
  meta: {
    version: Version;
    ecl: Ecl;
    mask: Mask;
  },
  plugins: Plugin[],
): { matrix: Uint8Array; version: Version; ecl: Ecl; mask: Mask } {
  const { version, ecl, mask } = meta;

  const numModules = NUM_DATA_BITS[version];
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

  const contentLen = bytePos;
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

  plugins.forEach((p) => p.mutateMessage(messageBytes, contentLen, { matrix, version, ecl, mask }));

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

  plugins.forEach((p) => p.mutateSequence(interleaved));

  let bitIdx = 0;
  const masker = MASKERS[mask];
  iterateMostlyDataModules(width, (x, y) => {
    if (matrix[y * width + x] === 0) {
      const bit = (interleaved[bitIdx >> 3] >> (7 - (bitIdx % 8))) & Module.ON;
      set(x, y, Module.DATA | (bit ^ +masker(x, y)));
      bitIdx++;
    }
  });

  plugins.forEach((p) => p.mutateMatrix(matrix, version));

  return { matrix, version, ecl, mask };
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

export const EXP_TABLE = new Uint8Array(255);
export const LOG_TABLE = new Uint8Array(256);
{
  let x = 1;
  for (let i = 0; i < 255; i++) {
    EXP_TABLE[i] = x;
    LOG_TABLE[x] = i;
    if (x & 0b1000_0000) {
      x <<= 1;
      x ^= 0b1_0001_1101;
    } else {
      x <<= 1;
    }
  }
}

export function remainder(data: Uint8Array, generator: Uint8Array): Uint8Array {
  const base = new Uint8Array(data.length + generator.length);
  base.set(data);
  for (let i = 0; i < data.length; i++) {
    if (base[i] === 0) continue;
    const alphaDiff = LOG_TABLE[base[i]];
    for (let j = 0; j < generator.length; j++) {
      base[i + j + 1] ^= EXP_TABLE[(generator[j] + alphaDiff) % 255];
    }
  }
  return base.subarray(data.length, data.length + generator.length);
}

export function generatorPolynomial(eccPerBlock: number): Uint8Array {
  let prev = new Uint8Array(eccPerBlock);
  let curr = new Uint8Array(eccPerBlock);
  for (let i = 2; i <= eccPerBlock; i++) {
    [prev, curr] = [curr, prev];
    curr[i - 1] = (prev[i - 2] + i - 1) % 255;
    for (let j = i - 2; j > 0; j--) {
      const exp = (prev[j - 1] + i - 1) % 255;
      curr[j] = LOG_TABLE[EXP_TABLE[prev[j]] ^ EXP_TABLE[exp]];
    }
    curr[0] = LOG_TABLE[EXP_TABLE[prev[0]] ^ EXP_TABLE[i - 1]];
  }
  return curr;
}

// prettier-ignore
export const NUM_DATA_BITS = [
  0,
  208, 359, 567, 807,
  1079, 1383, 1568, 1936,
  2336, 2768, 3232, 3728,
  4256, 4651, 5243, 5867,
  6523, 7211, 7931, 8683,
  9252, 10068, 10916, 11796,
  12708, 13652, 14628, 15371,
  16411, 17483, 18587, 19723,
  20891, 22091, 23008, 24272,
  25568, 26896, 28256, 29648,
];

// prettier-ignore
export const NUM_EC_BYTES = [
  [0, 0, 0, 0],
  [7, 10, 13, 17], [10, 16, 22, 28], [15, 26, 36, 44], [20, 36, 52, 64],
  [26, 48, 72, 88], [36, 64, 96, 112], [40, 72, 108, 130], [48, 88, 132, 156],
  [60, 110, 160, 192], [72, 130, 192, 224], [80, 150, 224, 264], [96, 176, 260, 308],
  [104, 198, 288, 352], [120, 216, 320, 384], [132, 240, 360, 432], [144, 280, 408, 480],
  [168, 308, 448, 532], [180, 338, 504, 588], [196, 364, 546, 650], [224, 416, 600, 700],
  [224, 442, 644, 750], [252, 476, 690, 816], [270, 504, 750, 900], [300, 560, 810, 960],
  [312, 588, 870, 1050], [336, 644, 952, 1110], [360, 700, 1020, 1200], [390, 728, 1050, 1260],
  [420, 784, 1140, 1350], [450, 812, 1200, 1440], [480, 868, 1290, 1530], [510, 924, 1350, 1620],
  [540, 980, 1440, 1710], [570, 1036, 1530, 1800], [570, 1064, 1590, 1890], [600, 1120, 1680, 1980],
  [630, 1204, 1770, 2100], [660, 1260, 1860, 2220], [720, 1316, 1950, 2310], [750, 1372, 2040, 2430],
];

// prettier-ignore
export const NUM_BLOCKS = [
  [0, 0, 0, 0],
  [1, 1, 1, 1], [1, 1, 1, 1], [1, 1, 2, 2], [1, 2, 2, 4],
  [1, 2, 4, 4], [2, 4, 4, 4], [2, 4, 6, 5], [2, 4, 6, 6],
  [2, 5, 8, 8], [4, 5, 8, 8], [4, 5, 8, 11], [4, 8, 10, 11],
  [4, 9, 12, 16], [4, 9, 16, 16], [6, 10, 12, 18], [6, 10, 17, 16],
  [6, 11, 16, 19], [6, 13, 18, 21], [7, 14, 21, 25], [8, 16, 20, 25],
  [8, 17, 23, 25], [9, 17, 23, 34], [9, 18, 25, 30], [10, 20, 27, 32],
  [12, 21, 29, 35], [12, 23, 34, 37], [12, 25, 34, 40], [13, 26, 35, 42],
  [14, 28, 38, 45], [15, 29, 40, 48], [16, 31, 43, 51], [17, 33, 45, 54],
  [18, 35, 48, 57], [19, 37, 51, 60], [19, 38, 53, 63], [20, 40, 56, 66],
  [21, 43, 59, 70], [22, 45, 62, 74], [24, 47, 65, 77], [25, 49, 68, 81],
];

// prettier-ignore
/** Starts at 7 */
export const ALIGN_OFFSETS = [
          16, 18,
  20, 22, 24, 26,
  28, 20, 22, 24,
  24, 26, 28, 28,
  22, 24, 24, 26,
  26, 28, 28, 24,
  24, 26, 26, 26,
  28, 28, 24, 26,
  26, 26, 28, 28,
];

// prettier-ignore
/** Starts at 7 */
export const VERSION_INFO = [
                    0x07c94, 0x085bc,
  0x09a99, 0x0a4d3, 0x0bbf6, 0x0c762,
  0x0d847, 0x0e60d, 0x0f928, 0x10b78,
  0x1145d, 0x12a17, 0x13532, 0x149a6,
  0x15683, 0x168c9, 0x177ec, 0x18ec4,
  0x191e1, 0x1afab, 0x1b08e, 0x1cc1a,
  0x1d33f, 0x1ed75, 0x1f250, 0x209d5,
  0x216f0, 0x228ba, 0x2379f, 0x24b0b,
  0x2542e, 0x26a64, 0x27541, 0x28c69,
];

// prettier-ignore
export const FORMAT_INFO = [
  [0x77c4, 0x72f3, 0x7daa, 0x789d, 0x662f, 0x6318, 0x6c41, 0x6976],
  [0x5412, 0x5125, 0x5e7c, 0x5b4b, 0x45f9, 0x40ce, 0x4f97, 0x4aa0],
  [0x355f, 0x3068, 0x3f31, 0x3a06, 0x24b4, 0x2183, 0x2eda, 0x2bed],
  [0x1689, 0x13be, 0x1ce7, 0x19d0, 0x0762, 0x0255, 0x0d0c, 0x083b],
];

export const MASKERS: ((x: number, y: number) => boolean)[] = [
  (x, y) => (x + y) % 2 == 0,
  (_, y) => y % 2 == 0,
  (x, _) => x % 3 == 0,
  (x, y) => (x + y) % 3 == 0,
  (x, y) => (Math.floor(x / 3) + (y >> 1)) % 2 == 0,
  (x, y) => ((x * y) % 2) + ((x * y) % 3) == 0,
  (x, y) => (((x * y) % 2) + ((x * y) % 3)) % 2 == 0,
  (x, y) => (((x + y) % 2) + ((x * y) % 3)) % 2 == 0,
];
