import { ByteMode, FuqrError, type Encoder, type Mode } from "../fuqr.ts";

export const NumericMode: Mode = {
  indicator: 0b0001,
  cciLen: (version) => 10 + (version > 9 ? 2 : 0) + (version > 26 ? 2 : 0),
  // 10 bits per 3 digits, 4 or 7 bits for 1 or 2 leftover digits
  segLen: (len, version) => 4 + NumericMode.cciLen(version) + Math.ceil((len * 10) / 3),
};

export const AlphanumericMode: Mode = {
  indicator: 0b0010,
  cciLen: (version) => 9 + (version > 9 ? 2 : 0) + (version > 26 ? 2 : 0),
  // 11 bits per 2 chars, 6 bits for 1 leftover char
  segLen: (len, version) => 4 + AlphanumericMode.cciLen(version) + Math.ceil((len * 11) / 2),
};

export class NumericEncoder implements Encoder {
  public bytes: Uint8Array;
  constructor(content: string) {
    const bytes = new Uint8Array(content.length);
    for (let i = 0; i < content.length; i++) {
      const byte = content.charCodeAt(i);
      if (byte < 0x30 || 0x39 < byte) {
        throw new FuqrError("INVALID_ENCODING", `Content is not numeric`);
      }
      bytes[i] = byte;
    }
    this.bytes = bytes;
  }

  bitLen(version: number) {
    return NumericMode.segLen(this.bytes.length, version);
  }

  encode(version: number, push: (bits: number, len: number) => void) {
    const bytes = this.bytes;

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
    return AlphanumericMode.segLen(this.bytes.length, version);
  }

  encode(version: number, push: (bits: number, len: number) => void) {
    const bytes = this.bytes;

    push(AlphanumericMode.indicator, 4);
    push(bytes.length, AlphanumericMode.cciLen(version));
    for (let i = 0; i < Math.floor(bytes.length / 2); i++) {
      push(B45[bytes[i * 2]] * 45 + B45[bytes[i * 2 + 1]], 11);
    }
    if (bytes.length & 1) {
      push(B45[bytes[bytes.length - 1]], 6);
    }
  }
}

type Segment = { mode: number; start: number; end: number };

// Indexed by MixedEncoder segment mode
const MODES = [NumericMode, AlphanumericMode, ByteMode];

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

  private segment(version: number) {
    const modes = this.modes;
    const n = modes.length;

    if (n === 0) {
      return {
        bits: ByteMode.segLen(0, version),
        segments: [{ mode: 2, start: 0, end: 0 }],
      };
    }

    // One mode throughout is optimal as a single segment
    let mode = modes[0];
    let i = 1;
    while (i < n && modes[i] === mode) i++;
    if (i === n) {
      return {
        bits: MODES[mode].segLen(n, version),
        segments: [{ mode, start: 0, end: n }],
      };
    }

    // Costs are in sixths of a bit
    // numeric 20, alphanumeric 33, byte 48. Segments round up when closed.

    // header + first char
    const start0 = (4 + NumericMode.cciLen(version)) * 6 + 20;
    const start1 = (4 + AlphanumericMode.cciLen(version)) * 6 + 33;
    const start2 = (4 + ByteMode.cciLen(version)) * 6 + 48;

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

    const bytes = this.bytes;
    for (const { mode, start, end } of this.segments) {
      push(MODES[mode].indicator, 4);
      push(end - start, MODES[mode].cciLen(version));

      if (mode === 0) {
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
      } else if (mode === 1) {
        let i = start;
        for (; i + 2 <= end; i += 2) {
          push(B45[bytes[i]] * 45 + B45[bytes[i + 1]], 11);
        }
        if (i < end) push(B45[bytes[i]], 6);
      } else {
        for (let i = start; i < end; i++) push(bytes[i], 8);
      }
    }
  }
}
