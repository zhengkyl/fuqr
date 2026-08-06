// The manual reader. It walks the matrix with the library's own layout helpers
// and decodes what it finds, so it checks the parts a scanner glosses over:
// exact codewords, unused remainder bits and the padding pattern.
import {
  type Ecl,
  iterateMostlyDataModules,
  type Mask,
  MASKERS,
  Module,
  visitFormatInfo,
  visitVersionInfo,
} from "../typescript/src/fuqr.ts";
import { buildBlueprint } from "../typescript/src/extras/blueprint.ts";

export type Qr = { matrix: Uint8Array; version: number };

export type Examined = {
  version: number;
  ecl: Ecl;
  mask: Mask;
  content: string;
  padding: Uint8Array;
};

const ALPHANUMERIC = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

const EXP = new Uint8Array(512);
const LOG = new Uint8Array(256);
{
  let x = 1;
  for (let i = 0; i < 255; i++) {
    EXP[i] = x;
    LOG[x] = i;
    x = (x << 1) ^ (x & 0x80 ? 0x11d : 0);
  }
  for (let i = 255; i < 512; i++) EXP[i] = EXP[i - 255];
}

const mul = (a: number, b: number) => (a === 0 || b === 0 ? 0 : EXP[LOG[a] + LOG[b]]);

// Replaying the visit for every combination is cheaper than reversing the bch,
// and it checks both copies plus the dark module along the way.
function readFormatInfo(qr: Qr) {
  const width = qr.version * 4 + 17;
  const found: { ecl: Ecl; mask: Mask }[] = [];

  for (let ecl = 0; ecl < 4; ecl++) {
    for (let mask = 0; mask < 8; mask++) {
      let matches = true;
      visitFormatInfo(ecl as Ecl, mask as Mask, width, (x, y, value) => {
        matches &&= ((qr.matrix[y * width + x] ^ value) & Module.ON) === 0;
      });
      if (matches) found.push({ ecl: ecl as Ecl, mask: mask as Mask });
    }
  }

  if (found.length !== 1) throw new Error(`${found.length} format info candidates matched`);
  return found[0];
}

function checkVersionInfo(qr: Qr) {
  if (qr.version < 7) return;
  const width = qr.version * 4 + 17;
  let matches = true;
  visitVersionInfo(qr.version, width, (x, y, value) => {
    matches &&= ((qr.matrix[y * width + x] ^ value) & Module.ON) === 0;
  });
  if (!matches) throw new Error("version info does not match the matrix");
}

// Unmasks the data modules into per block codewords. The blueprint tags each
// module with the codeword it belongs to, which de-interleaves them for free.
function readCodewords(qr: Qr, ecl: Ecl, mask: Mask) {
  const blueprint = buildBlueprint(qr.version, ecl, mask);
  const { width, blocks, g1Blocks, messagePerG1, ecPerBlock } = blueprint;

  const codewords = Array.from(
    { length: blocks },
    (_, b) => new Uint8Array(messagePerG1 + (b >= g1Blocks ? 1 : 0) + ecPerBlock),
  );

  const masker = MASKERS[mask];
  iterateMostlyDataModules(width, (x, y) => {
    const pos = y * width + x;
    const cell = blueprint.matrix[pos];
    if ((cell & Module.DATA) === 0) return;

    const bit = (qr.matrix[pos] & Module.ON) ^ +masker(x, y);
    const key = cell >>> 8;
    if (key === 0) {
      if (bit !== 0) throw new Error(`remainder bit at ${x},${y} is set`);
      return;
    }
    // The eight modules of a codeword are consecutive, most significant first.
    const block = (key - 1) >> 8;
    const offset = (key - 1) & 0xff;
    codewords[block][offset] = (codewords[block][offset] << 1) | bit;
  });

  // A valid codeword is divisible by the generator, so it vanishes at every one
  // of its roots alpha^0 through alpha^(r-1). A scanner would repair the damage
  // instead of reporting it, which would hide a wrong codeword.
  codewords.forEach((codeword, b) => {
    for (let root = 0; root < ecPerBlock; root++) {
      let sum = 0;
      for (const byte of codeword) sum = mul(sum, EXP[root]) ^ byte;
      if (sum !== 0) throw new Error(`block ${b} is not a valid codeword (root ${root})`);
    }
  });

  const message = new Uint8Array(codewords.reduce((n, c) => n + c.length - ecPerBlock, 0));
  let at = 0;
  for (const codeword of codewords) {
    message.set(codeword.subarray(0, codeword.length - ecPerBlock), at);
    at += codeword.length - ecPerBlock;
  }
  return message;
}

function readMessage(message: Uint8Array, version: number) {
  let pos = 0;
  const read = (n: number) => {
    if (pos + n > message.length * 8) throw new Error("message ended mid segment");
    let value = 0;
    for (let i = 0; i < n; i++, pos++) {
      value = (value << 1) | ((message[pos >> 3] >> (7 - (pos & 7))) & 1);
    }
    return value;
  };

  const cciDiff = (version > 9 ? 2 : 0) + (version > 26 ? 2 : 0);
  const bytes: number[] = [];

  while (message.length * 8 - pos >= 4) {
    const mode = read(4);
    if (mode === 0) break;

    if (mode === 0b0001) {
      let count = read(10 + cciDiff);
      while (count >= 3) {
        const group = read(10);
        if (group > 999) throw new Error(`numeric group ${group} out of range`);
        bytes.push(
          0x30 + Math.floor(group / 100),
          0x30 + (Math.floor(group / 10) % 10),
          0x30 + (group % 10),
        );
        count -= 3;
      }
      if (count === 2) {
        const group = read(7);
        if (group > 99) throw new Error(`numeric pair ${group} out of range`);
        bytes.push(0x30 + Math.floor(group / 10), 0x30 + (group % 10));
      } else if (count === 1) {
        const digit = read(4);
        if (digit > 9) throw new Error(`numeric digit ${digit} out of range`);
        bytes.push(0x30 + digit);
      }
    } else if (mode === 0b0010) {
      let count = read(9 + cciDiff);
      while (count >= 2) {
        const pair = read(11);
        if (pair >= 45 * 45) throw new Error(`alphanumeric pair ${pair} out of range`);
        bytes.push(
          ALPHANUMERIC.charCodeAt(Math.floor(pair / 45)),
          ALPHANUMERIC.charCodeAt(pair % 45),
        );
        count -= 2;
      }
      if (count === 1) {
        const value = read(6);
        if (value >= 45) throw new Error(`alphanumeric value ${value} out of range`);
        bytes.push(ALPHANUMERIC.charCodeAt(value));
      }
    } else if (mode === 0b0100) {
      let count = read(version < 10 ? 8 : 16);
      while (count-- > 0) bytes.push(read(8));
    } else {
      throw new Error(`unsupported mode ${mode.toString(2)}`);
    }
  }

  // Everything past the terminator is padding, once the stream realigns to the
  // next byte boundary.
  const padding = message.subarray(Math.ceil(pos / 8));
  for (let i = 0; i < padding.length; i++) {
    const expected = i % 2 === 0 ? 0xec : 0x11;
    if (padding[i] !== expected) {
      throw new Error(`padding byte ${i} is ${padding[i]}, expected ${expected}`);
    }
  }

  return {
    content: new TextDecoder("utf-8", { fatal: true }).decode(Uint8Array.from(bytes)),
    padding,
  };
}

export function examine(qr: Qr): Examined {
  const { ecl, mask } = readFormatInfo(qr);
  checkVersionInfo(qr);
  return {
    version: qr.version,
    ecl,
    mask,
    ...readMessage(readCodewords(qr, ecl, mask), qr.version),
  };
}
