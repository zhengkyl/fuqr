import {
  type Ecl,
  iterateMostlyDataModules,
  type Mask,
  Module,
  NUM_BLOCKS,
  NUM_DATA_BITS,
  NUM_EC_BYTES,
  type Version,
  visitAlignmentPatterns,
  visitFinderPatterns,
  visitFormatInfo,
  visitTimingPatterns,
  visitVersionInfo,
} from "../fuqr.ts";

// Per-module info + derived layout. Low byte = Module flags; high bits =
// codeword key ((block<<8|offset)+1, 0 if none), so (key-1)>>8 is its block.
export function buildBlueprint(version: Version, ecl: Ecl = 0, mask: Mask = 2) {
  const width = version * 4 + 17;
  const numBytes = Math.floor(NUM_DATA_BITS[version] / 8);
  const numEcBytes = NUM_EC_BYTES[version][ecl];
  const numMessageBytes = numBytes - numEcBytes;
  const blocks = NUM_BLOCKS[version][ecl];
  const g1Blocks = blocks - (numBytes % blocks);
  const messagePerG1 = Math.floor(numMessageBytes / blocks);
  const ecPerBlock = numEcBytes / blocks;

  // Codeword index (in interleaved order) -> packed block/offset key.
  const cwToKey = new Uint32Array(numBytes);
  const blockOffset = new Uint8Array(blocks);
  for (let c = 0; c < numMessageBytes; c++) {
    const block = c < messagePerG1 * blocks ? c % blocks : g1Blocks + (c - messagePerG1 * blocks);
    cwToKey[c] = ((block << 8) | blockOffset[block]++) + 1;
  }
  for (let c = numMessageBytes; c < numBytes; c++) {
    const block = (c - numMessageBytes) % blocks;
    cwToKey[c] = ((block << 8) | blockOffset[block]++) + 1;
  }

  const matrix = new Uint32Array(width * width);
  const set = (x: number, y: number, value: number) => (matrix[y * width + x] |= value);
  visitFinderPatterns(width, set);
  visitTimingPatterns(width, set);
  visitFormatInfo(ecl, mask, width, set);
  visitAlignmentPatterns(version, width, set);
  visitVersionInfo(version, width, set);

  let bitIdx = 0;
  iterateMostlyDataModules(width, (x, y) => {
    const pos = y * width + x;
    if (matrix[pos] !== 0) return;
    const byteIdx = bitIdx >> 3;
    matrix[pos] = Module.DATA | (byteIdx < numBytes ? cwToKey[byteIdx] << 8 : 0);
    bitIdx++;
  });

  return {
    matrix,
    width,
    numBytes,
    numMessageBytes,
    numEcBytes,
    blocks,
    g1Blocks,
    messagePerG1,
    ecPerBlock,
  };
}
