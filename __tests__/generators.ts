import QRCode from "qrcode";
import {
  AlphanumericEncoder,
  MixedEncoder,
  NumericEncoder,
} from "../typescript/src/extras/encoders.ts";
import {
  ByteEncoder,
  type Ecl,
  FuqrError,
  generateWithEncoder,
  type Mask,
  Module,
} from "../typescript/src/fuqr.ts";

export type Mode = "numeric" | "alphanumeric" | "byte" | "mixed";
type Input = { content: string; mode: Mode; version: number; ecl: Ecl; mask: Mask };

export function generateNodeQrcode({ content, mode, version, ecl, mask }: Input) {
  try {
    let text;
    if (mode === "mixed") {
      text = content;
    } else if (mode === "byte") {
      text = [{ mode, data: new TextEncoder().encode(content) }];
    } else {
      text = [{ mode, data: content }];
    }
    const qr = QRCode.create(text, {
      version,
      errorCorrectionLevel: (["L", "M", "Q", "H"] as const)[ecl],
      maskPattern: mask,
    });
    return qr.modules.data;
  } catch (err) {
    return null;
  }
}

function makeEncoder(mode: Mode, content: string) {
  if (mode === "byte") {
    return new ByteEncoder(content);
  } else if (mode === "alphanumeric") {
    return new AlphanumericEncoder(content);
  } else if (mode === "numeric") {
    return new NumericEncoder(content);
  } else if (mode === "mixed") {
    return new MixedEncoder(content);
  } else {
    throw new Error("bad mode: " + mode);
  }
}

export function generateFuqr({ content, mode, version, ecl, mask }: Input) {
  const encoder = makeEncoder(mode, content);

  try {
    const qr = generateWithEncoder(encoder, {
      minVersion: version,
      maxVersion: version,
      minEcl: ecl,
      maxEcl: ecl,
      mask: mask,
    });
    return qr.matrix;
  } catch (err) {
    if (err instanceof FuqrError && err.code === "TEXT_TOO_LONG") {
      return null;
    }
    throw err;
  }
}

export function fuqrBitLen({ content, mode, version }: Input) {
  return makeEncoder(mode, content).bitLen(version);
}

export function nodeQrcodeBitLen({ content, version, ecl, mask }: Input) {
  const { segments } = QRCode.create(content, {
    version,
    errorCorrectionLevel: (["L", "M", "Q", "H"] as const)[ecl],
    maskPattern: mask,
  });

  return segments.reduce((bits, segment) => {
    // only way to narrow without casting each branch
    if (segment.data instanceof Uint8Array) {
      return bits + new ByteEncoder(new TextDecoder().decode(segment.data)).bitLen(version);
    } else if (segment.mode.id === "Numeric") {
      return bits + new NumericEncoder(segment.data).bitLen(version);
    } else if (segment.mode.id === "Alphanumeric") {
      return bits + new AlphanumericEncoder(segment.data).bitLen(version);
    } else {
      throw new Error("bad segment mode: " + segment.mode.id);
    }
  }, 0);
}

export function compare(
  { content, version, ecl, mask }: Input,
  fuqr: Uint8Array | null,
  nodeQrcode: Uint8Array | null,
) {
  const where = () => `${JSON.stringify(content)} v${version} ecl${ecl} mask${mask}`;

  if (fuqr == null || nodeQrcode == null) {
    if (fuqr == nodeQrcode) return "";
    return `${where()} fuqr fit = ${fuqr != null}, node-qrcode fit = ${nodeQrcode != null}`;
  }

  for (let i = 0; i < fuqr.length; i++) {
    if ((fuqr[i] & Module.ON) === nodeQrcode[i]) continue;

    const width = version * 4 + 17;
    return `${where()} differs at ${i % width},${Math.floor(i / width)}`;
  }

  return "";
}
