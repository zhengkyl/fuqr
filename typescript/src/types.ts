export type Version = number; // 1 to 40 inclusive
export type Mask = 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7;
export type Ecl = 0 | 1 | 2 | 3;

export const Module = {
  ON: 1 << 0,
  DATA: 1 << 1,
  FINDER: 1 << 2,
  ALIGNMENT: 1 << 3,
  TIMING: 1 << 4,
  FORMAT: 1 << 5,
  VERSION: 1 << 6,
  MODIFIER: 1 << 7,
};

export type QrErrorCode = "TEXT_TOO_LONG" | "LOGO_TOO_LARGE";
export class QrError extends Error {
  code: QrErrorCode;
  constructor(code: QrErrorCode) {
    super(code);
    this.name = "QrError";
    this.code = code;
  }
}

export const MAX_VERSION = 40;
