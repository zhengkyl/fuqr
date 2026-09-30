import { ByteMode, FuriousQrError, type Encoder, type Mode } from "../furious-qr.ts";

export const NumericMode: Mode = {
  indicator: 0b0001,
  cciLen: (version) => 10 + (version > 9 ? 2 : 0) + (version > 26 ? 2 : 0),
  // 10 bits per 3 digits, 4 or 7 bits for 1 or 2 leftover digits
  segLen: (version, len) => 4 + NumericMode.cciLen(version) + Math.ceil((len * 10) / 3),
  encodeUtf8: (version, bytes, push) => {
    push(NumericMode.indicator, 4);
    push(bytes.length, NumericMode.cciLen(version));
    const groups = Math.floor(bytes.length / 3);
    for (let i = 0; i < groups; i++) {
      const group =
        (bytes[i * 3] - 0x30) * 100 + (bytes[i * 3 + 1] - 0x30) * 10 + (bytes[i * 3 + 2] - 0x30);
      push(group, 10);
    }
    switch (bytes.length % 3) {
      case 1:
        push(bytes[bytes.length - 1] - 0x30, 4);
        break;
      case 2:
        push((bytes[bytes.length - 2] - 0x30) * 10 + (bytes[bytes.length - 1] - 0x30), 7);
        break;
    }
  },
};

export const AlphanumericMode: Mode = {
  indicator: 0b0010,
  cciLen: (version) => 9 + (version > 9 ? 2 : 0) + (version > 26 ? 2 : 0),
  // 11 bits per 2 chars, 6 bits for 1 leftover char
  segLen: (version, len) => 4 + AlphanumericMode.cciLen(version) + Math.ceil((len * 11) / 2),
  encodeUtf8: (version, bytes, push) => {
    push(AlphanumericMode.indicator, 4);
    push(bytes.length, AlphanumericMode.cciLen(version));
    for (let i = 0; i < Math.floor(bytes.length / 2); i++) {
      push(B45_LUT[bytes[i * 2]] * 45 + B45_LUT[bytes[i * 2 + 1]], 11);
    }
    if (bytes.length & 1) {
      push(B45_LUT[bytes[bytes.length - 1]], 6);
    }
  },
};

export class NumericEncoder implements Encoder {
  public bytes: Uint8Array;
  constructor(content: string) {
    const bytes = new Uint8Array(content.length);
    for (let i = 0; i < content.length; i++) {
      const byte = content.charCodeAt(i);
      if (byte < 0x30 || 0x39 < byte) {
        throw new FuriousQrError("ENCODING_FAILURE", `Content is not numeric`);
      }
      bytes[i] = byte;
    }
    this.bytes = bytes;
  }
  bitLen(version: number) {
    return NumericMode.segLen(version, this.bytes.length);
  }
  encode(version: number, push: (bits: number, len: number) => void) {
    NumericMode.encodeUtf8(version, this.bytes, push);
  }
}

// Base45 value of each byte, or 255 if not alphanumeric
export const B45_LUT = new Uint8Array(256).fill(255);
for (let i = 0; i < 45; i++) {
  B45_LUT["0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:".charCodeAt(i)] = i;
}

export class AlphanumericEncoder implements Encoder {
  public bytes: Uint8Array;
  constructor(content: string) {
    const bytes = new Uint8Array(content.length);
    for (let i = 0; i < content.length; i++) {
      const byte = content.charCodeAt(i);
      if (byte > 255 || B45_LUT[byte] === 255) {
        throw new FuriousQrError("ENCODING_FAILURE", `Content is not alphanumeric`);
      }
      bytes[i] = byte;
    }
    this.bytes = bytes;
  }
  bitLen(version: number) {
    return AlphanumericMode.segLen(version, this.bytes.length);
  }
  encode(version: number, push: (bits: number, len: number) => void) {
    AlphanumericMode.encodeUtf8(version, this.bytes, push);
  }
}

type Segment = { mode: number; start: number; end: number };

const MODES = [NumericMode, AlphanumericMode, ByteMode];

// Cheapest mode each byte fits in
const MODE = new Uint8Array(256).fill(2);
for (let i = 0; i < 256; i++) {
  if (B45_LUT[i] < 10) MODE[i] = 0;
  else if (B45_LUT[i] !== 255) MODE[i] = 1;
}

// Open segment states: numeric with length 1, 2, 0 mod 3, alphanumeric with
// length 1, 0 mod 2, then byte. Each continues the one before it in its mode.
const STATE_MODE = [0, 0, 0, 1, 1, 2];
const PREV = [2, 0, 1, 4, 3, 5];
const CHAR_BITS = [4, 3, 3, 6, 5, 8];
// Only a mode's first state opens a segment
const FIRST = [0, 3, 5];
const INF = 2 ** 30;

export class MixedEncoder implements Encoder {
  public bytes: Uint8Array;
  public modes: Uint8Array;
  public segments: Segment[];
  public version: number;
  // Segmentation only depends on char count indicator lengths, so there are
  // only 3 distinct results: versions 1-9, 10-26, 27-40
  private cached: { bits: number; segments: Segment[] }[];

  constructor(content: string) {
    this.segments = [];
    this.version = 0;
    this.cached = [];

    const bytes = new TextEncoder().encode(content);
    const modes = new Uint8Array(bytes.length);
    for (let i = 0; i < bytes.length; i++) modes[i] = MODE[bytes[i]];
    this.bytes = bytes;
    this.modes = modes;
  }

  bitLen(version: number): number {
    const group = version < 10 ? 0 : version < 27 ? 1 : 2;
    const result = (this.cached[group] ??= this.segment(version));
    this.segments = result.segments;
    this.version = version;
    return result.bits;
  }

  // Viterbi over open segment states, exact since a state fixes the bits of the
  // next char. Fewest bits win, then fewest segments, then the earliest start.
  private segment(version: number) {
    const modes = this.modes;
    const n = modes.length;

    if (n === 0) {
      return {
        bits: ByteMode.segLen(version, 0),
        segments: [{ mode: 2, start: 0, end: 0 }],
      };
    }

    const headers = MODES.map((m) => 4 + m.cciLen(version));
    // Best path into each state as bits, segment count, and segment start
    let [bits, count, start] = [0, 0, 0].map(() => new Int32Array(6).fill(INF));
    let [nextBits, nextCount, nextStart] = [0, 0, 0].map(() => new Int32Array(6));
    // Low 3 bits are the best state at i, then 1 bit per mode if it opened at i
    const trace = new Uint8Array(n);
    let closedBits = 0;
    let closedCount = 0;

    for (let i = 0; i < n; i++) {
      let best = 0;
      for (let k = 0; k < 6; k++) {
        const mode = STATE_MODE[k];
        const p = PREV[k];
        nextBits[k] = modes[i] > mode ? INF : bits[p] + CHAR_BITS[k];
        nextCount[k] = count[p];
        nextStart[k] = start[p];

        // Continuing starts earlier, so opening must be strictly better
        const open = closedBits + headers[mode] + CHAR_BITS[k];
        if (
          k === FIRST[mode] &&
          modes[i] <= mode &&
          (open < nextBits[k] || (open === nextBits[k] && closedCount + 1 < nextCount[k]))
        ) {
          nextBits[k] = open;
          nextCount[k] = closedCount + 1;
          nextStart[k] = i;
          trace[i] |= 8 << mode;
        }

        const d =
          nextBits[k] - nextBits[best] ||
          nextCount[k] - nextCount[best] ||
          nextStart[k] - nextStart[best];
        if (d < 0) best = k;
      }
      trace[i] |= best;
      closedBits = nextBits[best];
      closedCount = nextCount[best];
      [bits, nextBits] = [nextBits, bits];
      [count, nextCount] = [nextCount, count];
      [start, nextStart] = [nextStart, start];
    }

    const segments: Segment[] = [];
    let k = trace[n - 1] & 7;
    let end = n;
    for (let i = n - 1; i >= 0; i--) {
      const mode = STATE_MODE[k];
      if (k === FIRST[mode] && trace[i] & (8 << mode)) {
        segments.push({ mode, start: i, end });
        end = i;
        k = trace[i - 1] & 7;
      } else {
        k = PREV[k];
      }
    }
    segments.reverse();

    return { bits: closedBits, segments };
  }

  encode(version: number, push: (bits: number, len: number) => void) {
    this.bitLen(version);

    const bytes = this.bytes;
    for (const { mode, start, end } of this.segments) {
      MODES[mode].encodeUtf8(version, bytes.subarray(start, end), push);
    }
  }
}
