const EXP_TABLE = new Uint8Array(255);
const LOG_TABLE = new Uint8Array(256);
{
  let x = 1;
  for (let i = 0; i < 255; i++) {
    EXP_TABLE[i] = x;
    LOG_TABLE[x] = i;
    if (x & 0b1000_0000) {
      x <<= 1;
      x ^= 0b1_0001_1101;
    } else {
      x <<= 1;
    }
  }
}

export function remainder(data: Uint8Array, generator: Uint8Array): Uint8Array {
  const base = new Uint8Array(data.length + generator.length);
  base.set(data);
  for (let i = 0; i < data.length; i++) {
    if (base[i] === 0) continue;
    const alphaDiff = LOG_TABLE[base[i]];
    for (let j = 0; j < generator.length; j++) {
      base[i + j + 1] ^= EXP_TABLE[(generator[j] + alphaDiff) % 255];
    }
  }
  return base.subarray(data.length, data.length + generator.length);
}

export function generatorPolynomial(eccPerBlock: number): Uint8Array {
  let prev = new Uint8Array(eccPerBlock);
  let curr = new Uint8Array(eccPerBlock);
  for (let i = 2; i <= eccPerBlock; i++) {
    [prev, curr] = [curr, prev];
    curr[i - 1] = (prev[i - 2] + i - 1) % 255;
    for (let j = i - 2; j > 0; j--) {
      const exp = (prev[j - 1] + i - 1) % 255;
      curr[j] = LOG_TABLE[EXP_TABLE[prev[j]] ^ EXP_TABLE[exp]];
    }
    curr[0] = LOG_TABLE[EXP_TABLE[prev[0]] ^ EXP_TABLE[i - 1]];
  }
  return curr;
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
    row.set(remainder(basis, divisor), k);
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
