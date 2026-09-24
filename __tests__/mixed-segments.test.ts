import { expect, test } from "vitest";
import { type Mask } from "../typescript/src/fuqr.ts";
import { BYTE_CONTENTS, ECLS, VERSIONS } from "./data.ts";
import {
  compare,
  fuqrBitLen,
  generateFuqr,
  generateNodeQrcode,
  type Mode,
  nodeQrcodeBitLen,
} from "./generators.ts";
import { scanZxing } from "./scanners.ts";

test.each(VERSIONS)(
  "mixed mode version %i",
  async (version) => {
    for (const ecl of ECLS) {
      let i = 0;
      for (const content of BYTE_CONTENTS) {
        const masks = (i === 0 ? [0, 1, 2, 3, 4, 5, 6, 7] : [2]) as Mask[];
        for (const mask of masks) {
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
