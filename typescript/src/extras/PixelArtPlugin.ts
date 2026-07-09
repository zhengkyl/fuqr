import {
  type Details,
  EXP_TABLE,
  FuqrError,
  type GenerateOptions,
  generatorPolynomial,
  iterateMostlyDataModules,
  LOG_TABLE,
  MASKERS,
  Module,
  NUM_BLOCKS,
  NUM_DATA_BITS,
  NUM_EC_BYTES,
  type Plugin,
  polynomialRemainder,
  visitAlignmentPatterns,
  visitTimingPatterns,
} from "../fuqr.ts";

type Break = { index: number; mask: number; value: number };
// Each weightedStencil value is (weight << 1) | bit, indexed by module position.
// Weight 0 means don't care. Data module bits are pre-mask, function module bits are final.
export class PixelArtPlugin implements Plugin {
  weightedStencil: Uint8Array;
  // Error correction bytes kept in reserve for real-world decode errors.
  // The rest of the budget is spent breaking forced bytes. Infinity disables breaks.
  reservedEc: number;
  breaks: Break[] = [];

  // Animation state: previous frame's message and ec bytes, used to hold bytes
  // stable across frames. Reuse one instance across frames (reassigning
  // weightedStencil) to engage it.
  prevMsg: Uint8Array | null = null;
  prevEc: Uint8Array | null = null;
  private numMessageBytes = 0;
  private numEcBytes = 0;
  // How many times each message byte has been a padding solver, so the churn
  // can rotate fairly through every byte instead of sticking to fixed columns.
  private solverUse = new Uint32Array(0);

  constructor(weightedStencil: Uint8Array, reservedEc = 3) {
    this.weightedStencil = weightedStencil;
    this.reservedEc = reservedEc;
  }

  mutateDetails(details: Details, options: Required<GenerateOptions>): void {
    let w = details.version * 4 + 17;
    if (w * w > this.weightedStencil.length) {
      throw new FuqrError("STENCIL_TOO_SMALL", "Required version exceeds stencil version");
    }

    while (w * w !== this.weightedStencil.length) {
      if (details.version == options.maxVersion) {
        if (w * w > this.weightedStencil.length) {
          throw new FuqrError("STENCIL_WRONG_SIZE", "Stencil must be (4n + 17)^2 for 1<=n<=40");
        } else {
          throw new FuqrError("STENCIL_TOO_BIG", "Stencil version exceeds max version");
        }
      }

      details.version += 1;
      w += 4;
    }

    details.ecl = 0;
  }

  // draw over timing and all alignment patterns except the only used (bottom right)
  mutateMatrix(matrix: Uint8Array, { version }: Details) {
    const weightedStencil = this.weightedStencil;
    const width = version * 4 + 17;
    const override = (x: number, y: number) => {
      const posIdx = y * width + x;
      const stencilVal = weightedStencil[posIdx];
      if (stencilVal >> 1 === 0) return;
      matrix[posIdx] = (matrix[posIdx] & ~Module.ON) | (stencilVal & Module.ON);
    };
    visitTimingPatterns(width, override);

    const sparedStart = width - 9;
    visitAlignmentPatterns(version, width, (x: number, y: number) => {
      if (x >= sparedStart && y >= sparedStart) return;
      override(x, y);
    });
  }

  mutateMessage(
    messageBytes: Uint8Array,
    paddingStart: number,
    matrix: Uint8Array,
    details: Details,
  ) {
    const { version, ecl, mask } = details;
    const numBytes = NUM_DATA_BITS[version] >> 3;
    const numEcBytes = NUM_EC_BYTES[version][ecl];
    const numMessageBytes = numBytes - numEcBytes;
    const numBlocks = NUM_BLOCKS[version][ecl];
    const numG1Blocks = numBlocks - (numBytes % numBlocks);
    const messagePerG1 = Math.floor(numMessageBytes / numBlocks);
    const ecPerBlock = numEcBytes / numBlocks;

    // Stash the interleaved layout for fixInterleaved's ec snapshot, and drop
    // stale snapshots if the code dimensions changed between frames.
    this.numMessageBytes = numMessageBytes;
    this.numEcBytes = numEcBytes;
    if (this.prevMsg && this.prevMsg.length !== numMessageBytes) this.prevMsg = null;
    if (this.prevEc && this.prevEc.length !== numEcBytes) this.prevEc = null;
    if (this.solverUse.length !== numMessageBytes)
      this.solverUse = new Uint32Array(numMessageBytes);

    const blockStart = (b: number) => b * messagePerG1 + Math.max(0, b - numG1Blocks);

    const cPerBlock = Array.from({ length: numBlocks }, () => 0);
    const pPerBlock = Array.from({ length: numBlocks }, () => 0);
    let remainingC = paddingStart;
    for (let b = 0; b < numBlocks; b++) {
      const capacity = b < numG1Blocks ? messagePerG1 : messagePerG1 + 1;
      const filled = Math.min(capacity, remainingC);
      remainingC -= filled;
      cPerBlock[b] = filled;
      pPerBlock[b] = capacity - filled;
    }

    // byte is the solve target, mask/value are the forced bits, weight is the
    // total weight of forced bits which don't already match.
    type Option = Break & { symbol: number; byte: number; weight: number };
    const fixPerBlock: Option[][] = Array.from({ length: numBlocks }, () => []);
    const breakPerBlock: Option[][] = Array.from({ length: numBlocks }, () => []);

    let bitIdx = 0;
    const targetBuffer = [0, 0, 0, 0, 0, 0, 0, 0];

    const weightedStencil = this.weightedStencil;

    const masker = MASKERS[mask];
    const width = version * 4 + 17;
    iterateMostlyDataModules(width, (x, y) => {
      const posIdx = y * width + x;
      if (matrix[posIdx] !== 0) return;

      let stencilVal = weightedStencil[posIdx];
      if (stencilVal > 0) {
        stencilVal ^= +masker(x, y);
      }
      // msb order
      const msbBitPos = 7 - (bitIdx % 8);
      targetBuffer[msbBitPos] = stencilVal;

      const postByteIdx = bitIdx >> 3;
      if (msbBitPos === 0) {
        let block;
        let symbol;
        let isContent = false;
        let actualByte;

        if (postByteIdx < messagePerG1 * numBlocks) {
          block = postByteIdx % numBlocks;
          symbol = Math.floor(postByteIdx / numBlocks);
          actualByte = messageBytes[blockStart(block) + symbol];
          isContent = symbol < cPerBlock[block];
        } else if (postByteIdx < numMessageBytes) {
          block = (postByteIdx % numBlocks) + numG1Blocks;
          symbol = messagePerG1;
          actualByte = messageBytes[blockStart(block) + symbol];
          isContent = symbol < cPerBlock[block];
        } else {
          const ecIdx = postByteIdx - numMessageBytes;
          block = ecIdx % numBlocks;
          symbol =
            Math.floor(ecIdx / numBlocks) + (block < numG1Blocks ? messagePerG1 : messagePerG1 + 1);
          // ec bytes aren't known yet, only their forced bits matter
          actualByte = 0b1010_1010;
        }

        let forcedMask = 0;
        let forcedValue = 0;
        let weight = 0;
        for (let i = 0; i < 8; i++) {
          const target = targetBuffer[i];
          const targetWeight = target >> 1;
          if (targetWeight === 0) continue;
          forcedMask |= 1 << i;
          forcedValue |= (target & 1) << i;
          if (((actualByte >> i) & 1) !== (target & 1)) weight += targetWeight;
        }
        if (forcedMask !== 0) {
          const option = {
            symbol,
            index: postByteIdx,
            byte: (actualByte & ~forcedMask) | forcedValue,
            mask: forcedMask,
            value: forcedValue,
            weight,
          };
          if (isContent) {
            breakPerBlock[block].push(option);
          } else {
            fixPerBlock[block].push(option);
          }
        }
      }

      bitIdx++;
    });

    const breaks: Break[] = [];
    const breakLimit = Math.max(0, Math.floor(ecPerBlock / 2) - this.reservedEc);
    for (let b = 0; b < numBlocks; b++) {
      const fixable = pPerBlock[b];
      const fixOptions = fixPerBlock[b];
      const breakOptions = breakPerBlock[b];

      if (fixOptions.length > fixable) {
        fixOptions.sort((a, b) => b.weight - a.weight);
        breakOptions.push(...fixOptions.slice(fixable));
        fixOptions.length = fixable;
      }

      if (breakOptions.length > breakLimit) {
        breakOptions.sort((a, b) => b.weight - a.weight);
        breakOptions.length = breakLimit;
      }
      for (const { index, mask, value } of breakOptions) {
        breaks.push({ index, mask, value });
      }
    }
    this.breaks = breaks;

    // A fix on a data column directly sets that byte, since those codeword
    // bytes are systematic. A fix on an ec column couples the unknown padding,
    // so only those require solving. G is systematic ([I | P]), so each ec
    // constraint pairs with one unknown padding byte, giving a t×t solve.
    //
    // Every covered block must change r ec-equivalent bytes per frame (any data
    // change ripples through all of reed-solomon's parity). To avoid the ec
    // region strobing, a size-r window slides over the uncovered padding and ec
    // bytes each frame: bytes in the window float (padding becomes an unknown,
    // ec is left to recompute), bytes outside hold their previous displayed
    // value (padding pinned to prevMsg, ec pinned to prevEc). Pinning to the
    // previous value rather than a fixed pattern keeps the churn at r per frame
    // while spreading which bytes change.
    const r = ecPerBlock;
    const divisor = generatorPolynomial(ecPerBlock);
    let G = buildGeneratorMatrix(messagePerG1, ecPerBlock);
    for (let b = 0; b < numBlocks; b++) {
      if (b === numG1Blocks) {
        G = buildGeneratorMatrix(messagePerG1 + 1, ecPerBlock);
      }

      const p = pPerBlock[b];
      if (p === 0) continue;

      // No coverage means the block's data is unchanged, so its ec is static.
      const fixOptions = fixPerBlock[b];
      if (fixOptions.length === 0) continue;

      const m = cPerBlock[b];
      const start = blockStart(b);
      const k = b < numG1Blocks ? messagePerG1 : messagePerG1 + 1;

      // Apply data column fixes directly; collect picture ec column fixes.
      const ecFixes: { symbol: number; byte: number }[] = [];
      const taken = new Uint8Array(p);
      const coveredEc = new Uint8Array(r);
      for (const option of fixOptions) {
        if (option.symbol < k) {
          messageBytes[start + option.symbol] = option.byte;
          taken[option.symbol - m] = 1;
        } else {
          ecFixes.push(option);
          coveredEc[option.symbol - k] = 1;
        }
      }

      // Uncovered padding columns and the uncovered ec bytes that are
      // candidates to hold or let float.
      const padCols: number[] = [];
      for (let i = 0; i < p; i++) if (!taken[i]) padCols.push(m + i);
      const ecIdxs: number[] = [];
      for (let e = 0; e < r; e++) if (!coveredEc[e]) ecIdxs.push(e);

      // Draw padding solvers least-used first so the churn rotates through every
      // byte rather than concentrating in fixed columns. Ties keep column order.
      const solverUse = this.solverUse;
      const solverOrder = padCols
        .slice()
        .sort((a, c) => solverUse[start + a] - solverUse[start + c] || a - c);

      const prevMsg = this.prevMsg;
      const prevEc = this.prevEc;
      if (prevMsg && prevEc) {
        // Hold uncovered padding and pin uncovered ec to their previous values,
        // so only the rotating solver padding changes. Each pin costs one
        // padding solver, capped by how many uncovered padding bytes are free.
        for (const col of padCols) messageBytes[start + col] = prevMsg[start + col];
        const maxPins = padCols.length - ecFixes.length;
        for (let i = 0; i < Math.min(maxPins, ecIdxs.length); i++) {
          const e = ecIdxs[i];
          ecFixes.push({ symbol: k + e, byte: prevEc[e * numBlocks + b] });
        }
      }

      // Solve the picture ec fixes plus pins. (No history yet means no pins.)
      // Charge the solvers used so the churn rotates onward next frame.
      const solvers = solverOrder.slice(0, ecFixes.length);
      solveBlock(messageBytes, start, k, solvers, ecFixes, G, divisor);
      for (const col of solvers) solverUse[start + col]++;
    }

    // Remember the solved message so the next frame can pin held padding to it.
    this.prevMsg = messageBytes.slice();
  }

  // overwrite intentional broken bytes
  mutateSequence(interleaved: Uint8Array) {
    for (const { index, mask, value } of this.breaks) {
      interleaved[index] = (interleaved[index] & ~mask) | value;
    }
    // Remember the displayed ec bytes so the next frame can pin them.
    this.prevEc = interleaved.slice(this.numMessageBytes, this.numMessageBytes + this.numEcBytes);
  }
}

// Solves one block so its codeword hits the ecFixes' targets, writing the
// unknown padding columns into data at base and leaving the rest untouched.
// Requires unknownCols.length === ecFixes.length, which is square and
// invertible because every submatrix of a reed-solomon parity matrix is.
function solveBlock(
  data: Uint8Array,
  base: number,
  k: number,
  unknownCols: number[],
  ecFixes: { symbol: number; byte: number }[],
  G: Uint8Array[],
  divisor: Uint8Array,
) {
  const t = unknownCols.length;
  if (t === 0) return;

  const known = data.slice(base, base + k);
  for (const col of unknownCols) known[col] = 0;
  const knownEc = polynomialRemainder(known, divisor);
  const rhs = new Uint8Array(t);
  for (let j = 0; j < t; j++) rhs[j] = ecFixes[j].byte ^ knownEc[ecFixes[j].symbol - k];

  const A: Uint8Array[] = Array.from({ length: t }, (_, j) => {
    const row = new Uint8Array(t);
    for (let i = 0; i < t; i++) row[i] = G[unknownCols[i]][ecFixes[j].symbol];
    return row;
  });
  const solved = gf256Solve(A, rhs);
  for (let i = 0; i < t; i++) data[base + unknownCols[i]] = solved[i];
}

export function gf256Mul(a: number, b: number): number {
  if (a === 0 || b === 0) return 0;
  return EXP_TABLE[(LOG_TABLE[a] + LOG_TABLE[b]) % 255];
}

// Builds the k×n systematic generator matrix G = [I_k | P] over GF(256).
export function buildGeneratorMatrix(k: number, r: number): Uint8Array[] {
  const divisor = generatorPolynomial(r);
  const G: Uint8Array[] = [];
  for (let i = 0; i < k; i++) {
    const row = new Uint8Array(k + r);
    row[i] = 1;
    const basis = new Uint8Array(k);
    basis[i] = 1;
    row.set(polynomialRemainder(basis, divisor), k);
    G.push(row);
  }
  return G;
}

// Solves the k×k system A·x = b over GF(256) via Gauss-Jordan elimination.
export function gf256Solve(A: Uint8Array[], b: Uint8Array): Uint8Array {
  const k = A.length;
  const aug = A.map((row, i) => {
    const r = new Uint8Array(k + 1);
    r.set(row);
    r[k] = b[i];
    return r;
  });

  for (let col = 0; col < k; col++) {
    let pivotRow = -1;
    for (let row = col; row < k; row++) {
      if (aug[row][col] !== 0) {
        pivotRow = row;
        break;
      }
    }
    if (pivotRow === -1) throw new Error("Singular matrix");
    [aug[col], aug[pivotRow]] = [aug[pivotRow], aug[col]];

    const pivotInv = EXP_TABLE[(255 - LOG_TABLE[aug[col][col]]) % 255];
    for (let j = col; j <= k; j++) aug[col][j] = gf256Mul(aug[col][j], pivotInv);

    for (let row = 0; row < k; row++) {
      if (row === col || aug[row][col] === 0) continue;
      const factor = aug[row][col];
      for (let j = col; j <= k; j++) aug[row][j] ^= gf256Mul(factor, aug[col][j]);
    }
  }

  return Uint8Array.from(aug, (row) => row[k]);
}
