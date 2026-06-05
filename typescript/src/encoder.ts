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
    const full = Math.floor(this.bytes.length / 3) * 10;
    const remainder = Math.ceil((this.bytes.length % 3) * 3.5);
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
    const full = Math.floor(this.bytes.length / 2) * 11;
    const remainder = (this.bytes.length % 2) * 6;
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
