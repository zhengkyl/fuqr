// Takes [encoder, minVersion, maxVersion, minEcl, maxEcl, mask, content, svg]
// Gives version-ecl-mask-hash, then a space and the svg if svg is [margin, size, attributes] or an error code.
import { createInterface } from "node:readline";
import { AlphanumericEncoder, MixedEncoder, NumericEncoder } from "../src/extras/encoders.ts";
import {
  ByteEncoder,
  FuriousQrError,
  type GenerateOptions,
  generateWithEncoder,
  Module,
  renderSvg,
} from "../src/furious-qr.ts";

const ENCODERS = {
  numeric: NumericEncoder,
  alphanumeric: AlphanumericEncoder,
  byte: ByteEncoder,
  mixed: MixedEncoder,
};

type Request = [
  keyof typeof ENCODERS,
  number,
  number,
  number,
  number,
  number,
  string,
  [number, string | null, string] | null,
];

// FNV-1a 64 of dark modules as 0 or 1, as two 32 bit halves. Prime is 2^40 + 0x1b3.
function hash(matrix: Uint8Array) {
  let hi = 0xcbf29ce4;
  let lo = 0x84222325;
  for (const module of matrix) {
    lo = (lo ^ (module & Module.ON)) >>> 0;
    const low = lo * 0x1b3;
    hi = (Math.imul(hi, 0x1b3) + Math.floor(low / 2 ** 32) + (lo << 8)) >>> 0;
    lo = low >>> 0;
  }
  return hi.toString(16).padStart(8, "0") + lo.toString(16).padStart(8, "0");
}

function answer([encoder, minVersion, maxVersion, minEcl, maxEcl, mask, content, svg]: Request) {
  const options = { minVersion, maxVersion, minEcl, maxEcl, mask } as GenerateOptions;
  try {
    const qr = generateWithEncoder(new ENCODERS[encoder](content), options);
    const answer = `${qr.version}-${qr.ecl}-${qr.mask}-${hash(qr.matrix)}`;
    if (svg === null) return answer;
    return `${answer} ${renderSvg(qr, { margin: svg[0], size: svg[1], attributes: svg[2] })}`;
  } catch (err) {
    if (err instanceof FuriousQrError) return err.code;
    throw err;
  }
}

for await (const line of createInterface({ input: process.stdin })) {
  process.stdout.write(JSON.stringify(answer(JSON.parse(line) as Request)) + "\n");
}
