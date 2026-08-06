import { describe, expect, test } from "vitest";
import { type Ecl, type Mask } from "../typescript/src/fuqr.ts";
import { ALPHANUMERIC_CONTENTS, BYTE_CONTENTS, NUMERIC_CONTENTS, VERSIONS } from "./data.ts";
import { compare, generateFuqr, generateNodeQrcode, type Mode } from "./generators.ts";

const ECLS = [0, 1, 2, 3] as Ecl[];

const GROUPS: [Mode, string[]][] = [
  ["numeric", NUMERIC_CONTENTS],
  ["alphanumeric", ALPHANUMERIC_CONTENTS],
  ["byte", BYTE_CONTENTS],
];

describe.each(GROUPS)("%s mode", (mode, contents) => {
  test.each(VERSIONS)(
    "version %i",
    (version) => {
      for (const ecl of ECLS) {
        let i = 0;
        for (const content of contents) {
          // Only test all masks on first content for all version * ecl combos
          // Masking isn't affected by content, and this reduces test time a lot
          const masks = (i === 0 ? [0, 1, 2, 3, 4, 5, 6, 7] : [2]) as Mask[];
          i++;
          for (const mask of masks) {
            const input = {
              content,
              mode,
              version,
              ecl,
              mask,
            };
            const difference = compare(input, generateFuqr(input), generateNodeQrcode(input));
            expect(difference).toBe("");
          }
        }
      }
    },
    60_000,
  );
});
