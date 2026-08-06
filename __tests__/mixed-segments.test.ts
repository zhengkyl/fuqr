import { expect, test } from "vitest";
import { type Ecl, type Mask } from "../typescript/src/fuqr.ts";
import { BYTE_CONTENTS, VERSIONS } from "./data.ts";
import {
  compare,
  fuqrBitLen,
  generateFuqr,
  generateNodeQrcode,
  type Mode,
  nodeQrcodeBitLen,
} from "./generators.ts";
import { scanZxing } from "./scanners.ts";

const ECLS = [0, 1, 2, 3] as Ecl[];

test.each(VERSIONS)(
  "mixed mode version %i",
  async (version) => {
    for (const ecl of ECLS) {
      for (const content of BYTE_CONTENTS) {
        for (const mask of [0, 1, 2, 3, 4, 5, 6, 7] as Mask[]) {
          const input = { content, mode: "mixed" as Mode, version, ecl, mask };
          const fuqr = generateFuqr(input);
          const nodeQrcode = generateNodeQrcode(input);
          const difference = compare(input, fuqr, nodeQrcode);
          if (difference === "") continue;

          if (fuqr == null || nodeQrcode == null) {
            expect(difference).toBe("");
            continue;
          }

          // Make sure optimal
          expect(fuqrBitLen(input)).toBeLessThanOrEqual(nodeQrcodeBitLen(input));

          expect(await scanZxing({ matrix: fuqr, version })).toEqual({
            content,
            version,
            ecl,
            mask,
          });
        }
      }
    }
  },
  100_000,
);
