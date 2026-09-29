// Independent spec reference for encoders

export const NUMERIC = 0;
export const ALPHANUMERIC = 1;
export const BYTE = 2;
export type Mode = 0 | 1 | 2;
export type Segment = { mode: Mode; len: number };

export const ENCODERS = ["numeric", "alphanumeric", "byte", "mixed"] as const;
export type Encoder = (typeof ENCODERS)[number];

const B45 = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";
const MODE_OF = Array.from({ length: 256 }, (_, byte): Mode => {
  if (0x30 <= byte && byte <= 0x39) return NUMERIC;
  if (B45.includes(String.fromCharCode(byte))) return ALPHANUMERIC;
  return BYTE;
});

// ISO 18004 Table 3, by mode then version group
const CCI_LENS = [
  [10, 12, 14],
  [9, 11, 13],
  [8, 16, 16],
];

function cciLen(mode: Mode, version: number) {
  return CCI_LENS[mode][version <= 9 ? 0 : version <= 26 ? 1 : 2];
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

// Best segmentation of every prefix of bytes, from the best last segment in each mode.
// Fewest bits win, then fewest segments, then the longest last segment.
export function prefixes(bytes: Uint8Array, version: number) {
  const best: Best[] = [{ bits: 0, count: 0, start: 0, mode: BYTE }];
  let byteStart = 0;
  for (let end = 1; end <= bytes.length; end++) {
    let b: Best = { bits: Infinity, count: Infinity, start: Infinity, mode: BYTE };
    const consider = (start: number, mode: Mode) => {
      const bits = best[start].bits + segmentBits(mode, end - start, version);
      const count = best[start].count + 1;
      if (
        bits < b.bits ||
        (bits === b.bits && (count < b.count || (count === b.count && start < b.start)))
      ) {
        b = { bits, count, start, mode };
      }
    };
    for (const mode of [NUMERIC, ALPHANUMERIC] as Mode[]) {
      for (let start = end - 1; start >= 0 && MODE_OF[bytes[start]] <= mode; start--) {
        consider(start, mode);
      }
    }
    // Byte segments cost 8 bits per byte, so every end shares the best start
    // by fewest bits - 8 * start, then fewest segments, then earliest start
    const key = (start: number) => best[start].bits - 8 * start;
    const last = end - 1;
    if (
      key(last) < key(byteStart) ||
      (key(last) === key(byteStart) && best[last].count < best[byteStart].count)
    ) {
      byteStart = last;
    }
    consider(byteStart, BYTE);
    best.push(b);
  }
  return best;
}

// Segments of the best segmentation of bytes[0..end]
function backtrack(best: Best[], end: number) {
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
    if (bytes.some((b) => MODE_OF[b] > mode)) return null;
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

// First len code points of text repeated
export const cycle = (text: string, len: number) => {
  const chars = [...text];
  return Array.from({ length: len }, (_, i) => chars[i % chars.length]).join("");
};
