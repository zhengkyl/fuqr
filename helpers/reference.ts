// Independent spec reference for encoders

export const NUMERIC = 0;
export const ALPHANUMERIC = 1;
export const BYTE = 2;
export type Mode = 0 | 1 | 2;
export type Segment = { mode: Mode; len: number };

const INDICATORS = [0b0001, 0b0010, 0b0100];
const B45 = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

export const ENCODERS = ["numeric", "alphanumeric", "byte", "mixed"] as const;
export type Encoder = (typeof ENCODERS)[number];

// Cheapest mode a UTF-8 byte fits in
export function modeOf(byte: number): Mode {
  if (0x30 <= byte && byte <= 0x39) return NUMERIC;
  if (B45.includes(String.fromCharCode(byte))) return ALPHANUMERIC;
  return BYTE;
}

// ISO 18004 Table 3
export function cciLen(mode: Mode, version: number) {
  const group = version <= 9 ? 0 : version <= 26 ? 1 : 2;
  return [
    [10, 12, 14],
    [9, 11, 13],
    [8, 16, 16],
  ][mode][group];
}

export function segmentBits(mode: Mode, len: number, version: number) {
  const header = 4 + cciLen(mode, version);
  if (mode === NUMERIC) return header + 10 * Math.floor(len / 3) + [0, 4, 7][len % 3];
  if (mode === ALPHANUMERIC) return header + 11 * Math.floor(len / 2) + 6 * (len % 2);
  return header + 8 * len;
}

export const totalBits = (segments: Segment[], version: number) =>
  segments.reduce((bits, { mode, len }) => bits + segmentBits(mode, len, version), 0);

type Best = { bits: number; count: number; start: number; mode: Mode };

// Best segmentation of every prefix of bytes, by trying every last segment.
// Fewest bits win, then fewest segments, then the longest last segment.
export function prefixes(bytes: Uint8Array, version: number) {
  const best: Best[] = [{ bits: 0, count: 0, start: 0, mode: BYTE }];
  for (let end = 1; end <= bytes.length; end++) {
    let b: Best = { bits: Infinity, count: Infinity, start: Infinity, mode: BYTE };
    for (const mode of [NUMERIC, ALPHANUMERIC, BYTE] as Mode[]) {
      for (let start = end - 1; start >= 0 && modeOf(bytes[start]) <= mode; start--) {
        const bits = best[start].bits + segmentBits(mode, end - start, version);
        const count = best[start].count + 1;
        if (
          bits < b.bits ||
          (bits === b.bits && (count < b.count || (count === b.count && start < b.start)))
        ) {
          b = { bits, count, start, mode };
        }
      }
    }
    best.push(b);
  }
  return best;
}

// Segments of the best segmentation of bytes[0..end]
export function backtrack(best: Best[], end: number) {
  const segments: Segment[] = [];
  while (end > 0) {
    const { start, mode } = best[end];
    segments.push({ mode, len: end - start });
    end = start;
  }
  return segments.reverse();
}

// Segments an encoder must emit, or null if it rejects the content.
// Empty content is one empty segment, in byte mode for the mixed encoder.
export function expectedSegments(
  encoder: Encoder,
  bytes: Uint8Array,
  version: number,
): Segment[] | null {
  if (encoder !== "mixed") {
    const mode = ENCODERS.indexOf(encoder) as Mode;
    if (bytes.some((b) => modeOf(b) > mode)) return null;
    return [{ mode, len: bytes.length }];
  }
  if (bytes.length === 0) return [{ mode: BYTE, len: 0 }];
  return backtrack(prefixes(bytes, version), bytes.length);
}

export type Options = {
  minVersion: number;
  maxVersion: number;
  minEcl: number;
  maxEcl: number;
  mask: number;
};

// codewords[version - 1][ecl] is the number of data codewords
export function expectedDetails(
  bitsAt: (version: number) => number,
  { minVersion, maxVersion, minEcl, maxEcl, mask }: Options,
  codewords: number[][],
) {
  for (let version = minVersion; version <= maxVersion; version++) {
    const fits = (ecl: number) => bitsAt(version) <= 8 * codewords[version - 1][ecl];
    if (!fits(minEcl)) continue;
    let ecl = minEcl;
    while (ecl < maxEcl && fits(ecl + 1)) ecl++;
    return { version, ecl, mask };
  }
  return null;
}

// Parses an encoder's pushes back into segments and bytes, throwing on
// anything malformed.
export function decode(pushes: [value: number, len: number][], version: number) {
  const bits: number[] = [];
  for (const [value, len] of pushes) {
    if (!Number.isInteger(value) || value < 0 || value >= 2 ** len) {
      throw new Error(`pushed ${value} in ${len} bits`);
    }
    for (let i = len - 1; i >= 0; i--) bits.push((value >> i) & 1);
  }

  let pos = 0;
  const read = (len: number) => {
    if (pos + len > bits.length) throw new Error(`read past end at bit ${pos}`);
    let value = 0;
    for (let i = 0; i < len; i++) value = value * 2 + bits[pos++];
    return value;
  };
  const readBelow = (len: number, limit: number) => {
    const value = read(len);
    if (value >= limit) throw new Error(`${value} in ${len} bits at bit ${pos - len}`);
    return value;
  };

  const segments: Segment[] = [];
  const out: number[] = [];
  while (pos < bits.length) {
    const mode = INDICATORS.indexOf(read(4)) as Mode | -1;
    if (mode === -1) throw new Error(`bad mode indicator at bit ${pos - 4}`);
    const len = read(cciLen(mode, version));
    segments.push({ mode, len });
    if (mode === NUMERIC) {
      for (let i = 0; i < len; i += 3) {
        const digits = Math.min(3, len - i);
        const value = readBelow([0, 4, 7, 10][digits], 10 ** digits);
        for (const c of value.toString().padStart(digits, "0")) out.push(c.charCodeAt(0));
      }
    } else if (mode === ALPHANUMERIC) {
      for (let i = 0; i < len; i += 2) {
        if (len - i === 1) {
          out.push(B45.charCodeAt(readBelow(6, 45)));
        } else {
          const value = readBelow(11, 45 * 45);
          out.push(B45.charCodeAt(Math.floor(value / 45)), B45.charCodeAt(value % 45));
        }
      }
    } else {
      for (let i = 0; i < len; i++) out.push(read(8));
    }
  }
  return { segments, bytes: new Uint8Array(out), bits: bits.length };
}

// First len code points of text repeated
export const cycle = (text: string, len: number) => {
  const chars = [...text];
  return Array.from({ length: len }, (_, i) => chars[i % chars.length]).join("");
};
