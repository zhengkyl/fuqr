// Differential fuzz of the Typescript library against node-qrcode and zxing.
// Writes nothing, since it checks the reference, not ports. Pass a seed to
// reproduce a run.
import { type Ecl, type Mask } from "../typescript/src/fuqr.ts";
import { type Case, check, describe, makeRandom, MODES, run } from "./common.ts";

const ROUNDS = 5;

// Version 40-L capacity in chars
const MAX_LEN = { numeric: 7089, alphanumeric: 4296, byte: 2953, mixed: 7089 };

const seed = Number(process.argv[2] ?? Math.floor(Math.random() * 2 ** 32));
console.log(`seed ${seed}`);
const random = makeRandom(seed);

let failures = 0;
for (const mode of MODES) {
  const cases: Case[] = [];
  // Every version and ecl, since block layout depends on both
  for (let round = 0; round < ROUNDS; round++) {
    for (let version = 1; version <= 40; version++) {
      for (const ecl of [0, 1, 2, 3] as Ecl[]) {
        const pinned = { mode, minVersion: version, maxVersion: version, minEcl: ecl, maxEcl: ecl };

        // Halve content until it fits, so blocks are mostly filled with content
        let c = {
          ...pinned,
          mask: random.int(0, 7) as Mask,
          content: random.content(mode, MAX_LEN[mode]),
        };
        while ("error" in run(c)) {
          const codePoints = [...c.content];
          c = { ...c, content: codePoints.slice(0, codePoints.length >> 1).join("") };
        }
        cases.push(c);

        // Cut URLs are junk, so skip ones that don't fit
        if (mode !== "mixed") continue;
        const url = { ...pinned, mask: random.int(0, 7) as Mask, content: random.url() };
        if (!("error" in run(url))) cases.push(url);
      }
    }
  }

  for (const c of cases) {
    const error = await check(c, run(c));
    if (error === "") continue;
    failures++;
    console.error(`${describe(c)} ${error}`);
  }
  console.log(`${mode}: checked ${cases.length} cases`);
}

if (failures > 0) {
  console.error(`${failures} bad cases, seed ${seed}`);
  process.exit(1);
}
