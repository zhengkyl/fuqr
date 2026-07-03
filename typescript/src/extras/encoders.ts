import type { Encoder } from "../fuqr.ts";

export class NumericEncoder implements Encoder {
  public bytes: Uint8Array;
  constructor(text: string) {
    const bytes = new Uint8Array(text.length);
    for (let i = 0; i < text.length; i++) {
      bytes[i] = text.charCodeAt(i);
    }
    this.bytes = bytes;
  }

  bitLen(version: number) {
    const cci = 10 + (version > 9 ? 2 : 0) + (version > 26 ? 2 : 0);
    const byteLen = this.bytes.length;
    const full = Math.floor(byteLen / 3) * 10;
    const remainder = Math.ceil((byteLen % 3) * 3.5);
    return 4 + cci + full + remainder;
  }

  encode(version: number, push: (bits: number, len: number) => void) {
    const cci = 10 + (version > 9 ? 2 : 0) + (version > 26 ? 2 : 0);
    const bytes = this.bytes;

    push(0b0001, 4);
    push(bytes.length, cci);
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

export class AlphanumericEncoder implements Encoder {
  static byteToB45(c: number): number {
    if (c >= 0x41 && c <= 0x5a) return c - 0x41 + 10; // A-Z
    if (c === 0x3a) return 44; // ':'
    if (c >= 0x30 && c <= 0x39) return c - 0x30; // 0-9
    if (c === 0x20) return 36; // ' '
    if (c === 0x24) return 37; // '$'
    if (c === 0x25) return 38; // '%'
    if (c === 0x2a) return 39; // '*'
    if (c === 0x2b) return 40; // '+'
    if (c === 0x2d) return 41; // '-'
    if (c === 0x2e) return 42; // '.'
    if (c === 0x2f) return 43; // '/'
    return 255;
  }

  public bytes: Uint8Array;

  constructor(text: string) {
    const bytes = new Uint8Array(text.length);
    for (let i = 0; i < text.length; i++) {
      bytes[i] = text.charCodeAt(i);
    }
    this.bytes = bytes;
  }

  bitLen(version: number) {
    const cci = 9 + (version > 9 ? 2 : 0) + (version > 26 ? 2 : 0);
    const byteLen = this.bytes.length;
    const full = Math.floor(byteLen / 2) * 11;
    const remainder = (byteLen % 2) * 6;
    return 4 + cci + full + remainder;
  }

  encode(version: number, push: (bits: number, len: number) => void) {
    const cci = 9 + (version > 9 ? 2 : 0) + (version > 26 ? 2 : 0);
    const bytes = this.bytes;
    const byteToB45 = AlphanumericEncoder.byteToB45;

    push(0b0010, 4);
    push(bytes.length, cci);
    for (let i = 0; i < Math.floor(bytes.length / 2); i++) {
      const group = byteToB45(bytes[i * 2]) * 45 + byteToB45(bytes[i * 2 + 1]);
      push(group, 11);
    }
    if (bytes.length & 1) {
      push(byteToB45(bytes[bytes.length - 1]), 6);
    }
  }
}

export class MixedEncoder implements Encoder {
  public bytes: Uint8Array;
  public modes: Uint8Array;
  public segments: { mode: number; start: number; end: number }[] = [];
  public version = 0;

  constructor(text: string) {
    const bytes = new TextEncoder().encode(text);
    const modes = new Uint8Array(bytes.length);

    for (let i = 0; i < bytes.length; i++) {
      const byte = bytes[i];

      if (0x30 <= byte && byte <= 0x39) {
        modes[i] = 0;
      } else if (AlphanumericEncoder.byteToB45(byte) !== 255) {
        modes[i] = 1;
      } else {
        modes[i] = 2;
        // multibyte
        if (byte & 0b1100_000) {
          i++;
          if (byte & 0b1110_000) {
            i++;
            if (byte & 0b1111_000) {
              i++;
            }
          }
        }
      }
    }
    this.bytes = bytes;
    this.modes = modes;
  }

  bitLen(version: number): number {
    const modes = this.modes;
    const n = modes.length;

    const cciDiff = (version > 9 ? 2 : 0) + (version > 26 ? 2 : 0);
    const headers = [4 + 10 + cciDiff, 4 + 9 + cciDiff, 4 + (version < 10 ? 8 : 16)];

    const ithCost = [
      (i: number) => (i % 3 === 0 ? 4 : 3),
      (i: number) => (i % 2 === 0 ? 6 : 5),
      () => 8,
    ];

    const dp = [
      modes[0] <= 0 ? headers[0] + ithCost[0](0) : Infinity,
      modes[0] <= 1 ? headers[1] + ithCost[1](0) : Infinity,
      headers[2] + ithCost[2](0),
    ];
    const run = [modes[0] <= 0 ? 1 : 0, modes[0] <= 1 ? 1 : 0, 1];
    const choice = new Uint8Array(n * 3);

    for (let i = 1; i < n; i++) {
      const newDp = [Infinity, Infinity, Infinity];
      const newRun = [0, 0, 0];
      const swapFrom = [Math.min(dp[1], dp[2]), Math.min(dp[0], dp[2]), Math.min(dp[0], dp[1])];

      for (let m = 0; m < 3; m++) {
        if (modes[i] > m) continue;

        const stay = dp[m] + ithCost[m](run[m]);
        const swap = swapFrom[m] + headers[m] + ithCost[m](0);
        if (stay <= swap) {
          newDp[m] = stay;
          newRun[m] = run[m] + 1;
          choice[i * 3 + m] = m;
        } else {
          newDp[m] = swap;
          newRun[m] = 1;

          if (m === 0) choice[i * 3 + m] = dp[1] <= dp[2] ? 1 : 2;
          else if (m === 1) choice[i * 3 + m] = dp[0] <= dp[2] ? 0 : 2;
          else choice[i * 3 + m] = dp[0] <= dp[1] ? 0 : 1;
        }
      }

      dp[0] = newDp[0];
      dp[1] = newDp[1];
      dp[2] = newDp[2];
      run[0] = newRun[0];
      run[1] = newRun[1];
      run[2] = newRun[2];
    }

    let m = dp[0] <= dp[1] ? (dp[0] <= dp[2] ? 0 : 2) : dp[1] <= dp[2] ? 1 : 2;
    const cost = dp[m];
    const segments: { mode: number; start: number; end: number }[] = [];
    let end = n;
    for (let i = n - 1; i >= 1; i--) {
      const prev = choice[i * 3 + m];
      if (prev !== m) {
        segments.push({ mode: m, start: i, end });
        end = i;
        m = prev;
      }
    }
    segments.push({ mode: m, start: 0, end });
    segments.reverse();

    this.segments = segments;
    this.version = version;

    return cost;
  }

  encode(version: number, push: (bits: number, len: number) => void) {
    if (this.version !== version) this.bitLen(version);

    const bytes = this.bytes;
    const byteToB45 = AlphanumericEncoder.byteToB45;
    const cciDiff = (version > 9 ? 2 : 0) + (version > 26 ? 2 : 0);

    for (const { mode, start, end } of this.segments) {
      const len = end - start;

      if (mode === 0) {
        push(0b0001, 4);
        push(len, 10 + cciDiff);
        const groups = Math.floor(len / 3);
        for (let i = 0; i < groups; i++) {
          const b = start + i * 3;
          push((bytes[b] - 0x30) * 100 + (bytes[b + 1] - 0x30) * 10 + (bytes[b + 2] - 0x30), 10);
        }
        switch (len % 3) {
          case 1:
            push(bytes[end - 1] - 0x30, 4);
            break;
          case 2:
            push((bytes[end - 2] - 0x30) * 10 + (bytes[end - 1] - 0x30), 7);
            break;
        }
      } else if (mode === 1) {
        push(0b0010, 4);
        push(len, 9 + cciDiff);
        const pairs = Math.floor(len / 2);
        for (let i = 0; i < pairs; i++) {
          const b = start + i * 2;
          push(byteToB45(bytes[b]) * 45 + byteToB45(bytes[b + 1]), 11);
        }
        if (len & 1) push(byteToB45(bytes[end - 1]), 6);
      } else {
        push(0b0100, 4);
        push(len, version < 10 ? 8 : 16);
        for (let i = start; i < end; i++) push(bytes[i], 8);
      }
    }
  }
}
