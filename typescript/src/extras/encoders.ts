import { ByteEncoder, FuqrError, type Encoder } from "../fuqr.ts";

export class NumericEncoder implements Encoder {
  static cci(version: number) {
    return 10 + (version > 9 ? 2 : 0) + (version > 26 ? 2 : 0);
  }
  static segmentBitLen(len: number, version: number) {
    // 10 bits per 3 digits, 4 or 7 bits for 1 or 2 leftover digits
    return 4 + NumericEncoder.cci(version) + Math.ceil((len * 10) / 3);
  }
  static encodeSegment(
    bytes: Uint8Array,
    start: number,
    end: number,
    version: number,
    push: (bits: number, len: number) => void,
  ) {
    push(0b0001, 4);
    push(end - start, NumericEncoder.cci(version));
    let i = start;
    for (; i + 3 <= end; i += 3) {
      push((bytes[i] - 0x30) * 100 + (bytes[i + 1] - 0x30) * 10 + (bytes[i + 2] - 0x30), 10);
    }
    switch (end - i) {
      case 1:
        push(bytes[i] - 0x30, 4);
        break;
      case 2:
        push((bytes[i] - 0x30) * 10 + (bytes[i + 1] - 0x30), 7);
        break;
    }
  }

  public bytes: Uint8Array;
  constructor(content: string) {
    const bytes = new Uint8Array(content.length);
    for (let i = 0; i < content.length; i++) {
      const byte = content.charCodeAt(i);
      if (byte < 0x30 || 0x39 < byte) {
        throw new FuqrError("INVALID_ENCODING", `Content is not numeric`);
      }
      bytes[i] = content.charCodeAt(i);
    }
    this.bytes = bytes;
  }

  bitLen(version: number) {
    return NumericEncoder.segmentBitLen(this.bytes.length, version);
  }

  encode(version: number, push: (bits: number, len: number) => void) {
    NumericEncoder.encodeSegment(this.bytes, 0, this.bytes.length, version, push);
  }
}

// Alphanumeric value of each byte, or 255 if not alphanumeric
const B45 = new Uint8Array(256).fill(255);
for (let i = 0; i < 45; i++) {
  B45["0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:".charCodeAt(i)] = i;
}

export class AlphanumericEncoder implements Encoder {
  static byteToB45(c: number): number {
    return c < 256 ? B45[c] : 255;
  }
  static cci(version: number) {
    return 9 + (version > 9 ? 2 : 0) + (version > 26 ? 2 : 0);
  }
  static segmentBitLen(len: number, version: number) {
    // 11 bits per 2 chars, 6 bits for 1 leftover char
    return 4 + AlphanumericEncoder.cci(version) + Math.ceil((len * 11) / 2);
  }
  static encodeSegment(
    bytes: Uint8Array,
    start: number,
    end: number,
    version: number,
    push: (bits: number, len: number) => void,
  ) {
    push(0b0010, 4);
    push(end - start, AlphanumericEncoder.cci(version));
    let i = start;
    for (; i + 2 <= end; i += 2) {
      push(B45[bytes[i]] * 45 + B45[bytes[i + 1]], 11);
    }
    if (i < end) push(B45[bytes[i]], 6);
  }

  public bytes: Uint8Array;
  constructor(content: string) {
    const bytes = new Uint8Array(content.length);
    for (let i = 0; i < content.length; i++) {
      const byte = content.charCodeAt(i);
      if (byte > 255 || B45[byte] === 255) {
        throw new FuqrError("INVALID_ENCODING", `Content is not alphanumeric`);
      }
      bytes[i] = byte;
    }
    this.bytes = bytes;
  }

  bitLen(version: number) {
    return AlphanumericEncoder.segmentBitLen(this.bytes.length, version);
  }

  encode(version: number, push: (bits: number, len: number) => void) {
    AlphanumericEncoder.encodeSegment(this.bytes, 0, this.bytes.length, version, push);
  }
}

type Segment = { mode: number; start: number; end: number };

// Indexed by segment mode
const ENCODERS = [NumericEncoder, AlphanumericEncoder, ByteEncoder];

// Cheapest mode each byte fits in: 0 numeric, 1 alphanumeric, 2 byte
// All multibyte UTF-8 bytes look like 1xxx_xxxx, so they are always 2
const MODE = new Uint8Array(256).fill(2);
for (let i = 0; i < 256; i++) {
  if (B45[i] < 10) MODE[i] = 0;
  else if (B45[i] !== 255) MODE[i] = 1;
}

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

  // Costs are in sixths of a bit so each char has a fixed cost:
  // numeric 20, alphanumeric 33, byte 48. Segments round up when closed.
  private segment(version: number) {
    const modes = this.modes;
    const n = modes.length;

    if (n === 0) {
      return {
        bits: ByteEncoder.segmentBitLen(0, version),
        segments: [{ mode: 2, start: 0, end: 0 }],
      };
    }

    // One mode throughout is optimal as a single segment
    let mode = modes[0];
    let i = 1;
    while (i < n && modes[i] === mode) i++;
    if (i === n) {
      return {
        bits: ENCODERS[mode].segmentBitLen(n, version),
        segments: [{ mode, start: 0, end: n }],
      };
    }

    // header + first char
    const start0 = (4 + NumericEncoder.cci(version)) * 6 + 20;
    const start1 = (4 + AlphanumericEncoder.cci(version)) * 6 + 33;
    const start2 = (4 + ByteEncoder.cci(version)) * 6 + 48;

    // prev[i] packs the mode at i - 1 for each mode at i, 2 bits per mode
    const prev = new Uint8Array(n);

    let c0 = mode === 0 ? start0 : Infinity;
    let c1 = mode <= 1 ? start1 : Infinity;
    let c2 = start2;

    for (i = 1; i < n; i++) {
      // Cost of closing a segment in each mode
      const r0 = Math.ceil(c0 / 6) * 6;
      const r1 = Math.ceil(c1 / 6) * 6;
      const r2 = Math.ceil(c2 / 6) * 6;
      mode = modes[i];
      let p = 0;

      // Staying wins ties to avoid pointless segments
      // Switching to the same mode is never better than staying
      if (mode === 0) {
        const from = r1 <= r2 ? 1 : 2;
        const swap = (from === 1 ? r1 : r2) + start0;
        const stay = c0 + 20;
        if (stay <= swap) {
          c0 = stay;
        } else {
          c0 = swap;
          p = from;
        }
      } else {
        c0 = Infinity;
      }

      if (mode <= 1) {
        const from = r0 <= r2 ? 0 : 2;
        const swap = (from === 0 ? r0 : r2) + start1;
        const stay = c1 + 33;
        if (stay <= swap) {
          c1 = stay;
          p |= 1 << 2;
        } else {
          c1 = swap;
          p |= from << 2;
        }
      } else {
        c1 = Infinity;
      }

      const from = r0 <= r1 ? 0 : 1;
      const swap = (from === 0 ? r0 : r1) + start2;
      const stay = c2 + 48;
      if (stay <= swap) {
        c2 = stay;
        p |= 2 << 4;
      } else {
        c2 = swap;
        p |= from << 4;
      }

      prev[i] = p;
    }

    let m = c0 <= c1 ? (c0 <= c2 ? 0 : 2) : c1 <= c2 ? 1 : 2;
    const bits = Math.ceil((m === 0 ? c0 : m === 1 ? c1 : c2) / 6);

    const segments: Segment[] = [];
    let end = n;
    for (let i = n - 1; i >= 1; i--) {
      const p = (prev[i] >> (m * 2)) & 0b11;
      if (p !== m) {
        segments.push({ mode: m, start: i, end });
        end = i;
        m = p;
      }
    }
    segments.push({ mode: m, start: 0, end });
    segments.reverse();

    return { bits, segments };
  }

  encode(version: number, push: (bits: number, len: number) => void) {
    this.bitLen(version);
    for (const { mode, start, end } of this.segments) {
      ENCODERS[mode].encodeSegment(this.bytes, start, end, version, push);
    }
  }
}
