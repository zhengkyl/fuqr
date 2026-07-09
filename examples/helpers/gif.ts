// Minimal GIF89a encoder: global palette, full replacement frames, loops forever.
// Frames are palette indices, one byte per pixel.
export function encodeGif(
  width: number,
  height: number,
  palette: [number, number, number][],
  frames: Uint8Array[],
  delayCs: number,
): Uint8Array {
  const out: number[] = [];
  const u16 = (n: number) => out.push(n & 0xff, (n >> 8) & 0xff);
  const ascii = (s: string) => out.push(...[...s].map((c) => c.charCodeAt(0)));

  ascii("GIF89a");
  u16(width);
  u16(height);
  let depth = 1;
  while (1 << depth < palette.length) depth++;
  out.push(0x80 | (depth - 1), 0, 0); // global palette of 2^depth colors
  for (let i = 0; i < 1 << depth; i++) out.push(...(palette[i] ?? [0, 0, 0]));

  out.push(0x21, 0xff, 0x0b); // netscape looping extension
  ascii("NETSCAPE2.0");
  out.push(3, 1, 0, 0, 0);

  const minCodeSize = Math.max(2, depth);
  const clearCode = 1 << minCodeSize;
  const endCode = clearCode + 1;

  for (const frame of frames) {
    out.push(0x21, 0xf9, 4, 0b100); // graphic control, keep previous frame
    u16(delayCs);
    out.push(0, 0);
    out.push(0x2c); // image descriptor, full frame
    u16(0);
    u16(0);
    u16(width);
    u16(height);
    out.push(0, minCodeSize);

    // variable code size lzw, packed lsb first
    const bytes: number[] = [];
    let buf = 0;
    let bufLen = 0;
    let codeSize = minCodeSize + 1;
    let nextCode = endCode + 1;
    let table = new Map<number, number>();
    const push = (code: number) => {
      buf |= code << bufLen;
      bufLen += codeSize;
      while (bufLen >= 8) {
        bytes.push(buf & 0xff);
        buf >>= 8;
        bufLen -= 8;
      }
    };

    push(clearCode);
    let prefix = frame[0];
    for (let i = 1; i < frame.length; i++) {
      const key = (prefix << 8) | frame[i];
      const code = table.get(key);
      if (code !== undefined) {
        prefix = code;
        continue;
      }
      push(prefix);
      if (nextCode === 4096) {
        push(clearCode);
        codeSize = minCodeSize + 1;
        nextCode = endCode + 1;
        table = new Map();
      } else {
        if (nextCode >= 1 << codeSize) codeSize++;
        table.set(key, nextCode++);
      }
      prefix = frame[i];
    }
    push(prefix);
    push(endCode);
    if (bufLen > 0) bytes.push(buf & 0xff);

    for (let i = 0; i < bytes.length; i += 255) {
      const block = bytes.slice(i, i + 255);
      out.push(block.length, ...block);
    }
    out.push(0); // end of image data
  }

  out.push(0x3b); // trailer
  return Uint8Array.from(out);
}
