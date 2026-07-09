// Custom finders (circle, triangle, square), a cephalopod tucked into the
// fourth corner, and data modules traced into merged svg contours.
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildSvgPath, generate, Module } from "../typescript/src/fuqr.ts";

const qr = generate("https://github.com/zhengkyl/fuqr");
const margin = 2;
const fg = "#000";
const bg = "#fff";

writeFileSync(join(import.meta.dirname, "outputs/squid.svg"), renderSVG());

function renderSVG(): string {
  const rowLen = qr.version * 4 + 17;
  const width = rowLen + 2 * margin;

  let svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${width} ${width}" width="300" height="300">`;
  svg += `<rect width="${width}" height="${width}" fill="${bg}"/>`;
  svg += `<g fill="${fg}">`;

  for (let y = 0; y < 8; y++) {
    for (let x = 0; x < rowLen; x++) {
      qr.matrix[y * rowLen + x] = 0;
    }
  }

  svg += `<path fill="${fg}" d="`;
  svg += circle(margin, margin, 7, true);
  svg += circle(margin + 1, margin + 1, 5, false);
  svg += circle(margin + 2, margin + 2, 3, true);
  svg += `"/>`;

  svg += `<path fill="${fg}" d="`;
  svg += triangle(width / 2, margin, (7 / Math.sqrt(3)) * 2, true);
  svg += triangle(width / 2, margin + 2, (4 / Math.sqrt(3)) * 2, false);
  svg += triangle(width / 2, margin + 4, (1 / Math.sqrt(3)) * 2, true);
  svg += `"/>`;

  svg += `<path fill="${fg}" d="`;
  svg += `M${width - 7 - margin},${margin}h7v7h-7z`;
  svg += `M${width - 7 - margin + 1},${margin + 1}v5h5v-5z`;
  svg += `M${width - 7 - margin + 2},${margin + 2}h3v3h-3z`;
  svg += `"/>`;

  svg += `<g transform="translate(${margin} ${width - 7 - margin})">
    <path d="M9.4 9H-2C2 5.7 5.3 5.8 9.4 9Z" style="fill:#d20964"/>
    <ellipse cx="3.9" cy="3.3" rx="3.7" ry="4.1" style="fill:#f76f96"/>
    <circle cx="3.5" cy="3.7" r="3.9" style="fill:#2e2e2e"/>
    <ellipse cx="-.2" cy="5.6" rx="3.8" ry="3.1" style="fill:#111" transform="rotate(-45.7) skewX(-.6)"/>
    <path d="M1.2 1.3v4.4h4.5V1.3ZM2 2h3v3H2z" style="fill:#fff"/>
    <g style="fill:#000" transform="rotate(-9 -13.5 11.5)">
      <circle cx="1.8" cy="4.5" r=".1"/>
      <circle cx="1.8" cy="5.1" r=".1"/>
      <circle cx="1.8" cy="5.6" r=".1"/>
      <circle cx="1.4" cy="5.3" r=".1"/>
      <circle cx="1.8" cy="6.1" r=".1"/>
      <circle cx="1.4" cy="5.9" r=".1"/>
      <circle cx="1.8" cy="6.7" r=".1"/>
      <circle cx="1.4" cy="6.4" r=".1"/>
      <circle cx="1.8" cy="7.2" r=".1"/>
      <circle cx="1.4" cy="6.9" r=".1"/>
      <circle cx="1.8" cy="7.7" r=".1"/>
      <circle cx="1.4" cy="7.5" r=".1"/>
      <circle cx="1.8" cy="8.3" r=".1"/>
      <circle cx="1.4" cy="8" r=".1"/>
      <circle cx="1.8" cy="8.8" r=".1"/>
      <circle cx="1.4" cy="8.5" r=".1"/>
      <circle cx="1.8" cy="9.3" r=".1"/>
      <circle cx="2.6" cy="4" r=".1"/>
      <circle cx="2.6" cy="4.5" r=".1"/>
      <circle cx="2.2" cy="4.3" r=".1"/>
      <circle cx="2.6" cy="5.1" r=".1"/>
      <circle cx="2.2" cy="4.8" r=".1"/>
      <circle cx="2.6" cy="5.6" r=".1"/>
      <circle cx="2.2" cy="5.3" r=".1"/>
      <circle cx="2.6" cy="6.1" r=".1"/>
      <circle cx="2.2" cy="5.9" r=".1"/>
      <circle cx="2.6" cy="6.7" r=".1"/>
      <circle cx="2.2" cy="6.4" r=".1"/>
      <circle cx="2.6" cy="7.2" r=".1"/>
      <circle cx="2.2" cy="6.9" r=".1"/>
      <circle cx="2.6" cy="7.7" r=".1"/>
      <circle cx="2.2" cy="7.5" r=".1"/>
      <circle cx="2.6" cy="8.3" r=".1"/>
      <circle cx="2.2" cy="8" r=".1"/>
      <circle cx="2.6" cy="8.8" r=".1"/>
      <circle cx="2.2" cy="8.5" r=".1"/>
      <circle cx="2.6" cy="9.3" r=".1"/>
      <circle cx="2.2" cy="9.1" r=".1"/>
      <circle cx="2.6" cy="9.8" r=".1"/>
      <circle cx="2.2" cy="9.6" r=".1"/>
      <circle cx="3.4" cy="4" r=".1"/>
      <circle cx="3" cy="3.7" r=".1"/>
      <circle cx="3.4" cy="4.5" r=".1"/>
      <circle cx="3" cy="4.3" r=".1"/>
      <circle cx="3.4" cy="5.1" r=".1"/>
      <circle cx="3" cy="4.8" r=".1"/>
      <circle cx="3.4" cy="5.6" r=".1"/>
      <circle cx="3" cy="5.3" r=".1"/>
      <circle cx="3.4" cy="6.1" r=".1"/>
      <circle cx="3" cy="5.9" r=".1"/>
      <circle cx="3.4" cy="6.7" r=".1"/>
      <circle cx="3" cy="6.4" r=".1"/>
      <circle cx="3.4" cy="7.2" r=".1"/>
      <circle cx="3" cy="6.9" r=".1"/>
      <circle cx="3.4" cy="7.7" r=".1"/>
      <circle cx="3" cy="7.5" r=".1"/>
      <circle cx="3.4" cy="8.3" r=".1"/>
      <circle cx="3" cy="8" r=".1"/>
      <circle cx="3.4" cy="8.8" r=".1"/>
      <circle cx="3" cy="8.5" r=".1"/>
      <circle cx="3.4" cy="9.3" r=".1"/>
      <circle cx="3" cy="9.1" r=".1"/>
      <circle cx="3.4" cy="9.8" r=".1"/>
      <circle cx="3" cy="9.6" r=".1"/>
      <circle cx="3.8" cy="3.2" r=".1"/>
      <circle cx="4.2" cy="4" r=".1"/>
      <circle cx="3.8" cy="3.7" r=".1"/>
      <circle cx="4.2" cy="4.5" r=".1"/>
      <circle cx="3.8" cy="4.3" r=".1"/>
      <circle cx="4.2" cy="5.1" r=".1"/>
      <circle cx="3.8" cy="4.8" r=".1"/>
      <circle cx="4.2" cy="5.6" r=".1"/>
      <circle cx="3.8" cy="5.3" r=".1"/>
      <circle cx="4.2" cy="6.1" r=".1"/>
      <circle cx="3.8" cy="5.9" r=".1"/>
      <circle cx="4.2" cy="6.7" r=".1"/>
      <circle cx="3.8" cy="6.4" r=".1"/>
      <circle cx="4.2" cy="7.2" r=".1"/>
      <circle cx="3.8" cy="6.9" r=".1"/>
      <circle cx="4.2" cy="7.7" r=".1"/>
      <circle cx="3.8" cy="7.5" r=".1"/>
      <circle cx="4.2" cy="8.3" r=".1"/>
      <circle cx="3.8" cy="8" r=".1"/>
      <circle cx="4.2" cy="8.8" r=".1"/>
      <circle cx="3.8" cy="8.5" r=".1"/>
      <circle cx="4.2" cy="9.3" r=".1"/>
      <circle cx="3.8" cy="9.1" r=".1"/>
      <circle cx="4.2" cy="9.8" r=".1"/>
      <circle cx="3.8" cy="9.6" r=".1"/>
      <circle cx="4.6" cy="3.2" r=".1"/>
      <circle cx="5" cy="4" r=".1"/>
      <circle cx="4.6" cy="3.7" r=".1"/>
      <circle cx="5" cy="4.5" r=".1"/>
      <circle cx="4.6" cy="4.3" r=".1"/>
      <circle cx="5" cy="5.1" r=".1"/>
      <circle cx="4.6" cy="4.8" r=".1"/>
      <circle cx="5" cy="5.6" r=".1"/>
      <circle cx="4.6" cy="5.3" r=".1"/>
      <circle cx="5" cy="6.1" r=".1"/>
      <circle cx="4.6" cy="5.9" r=".1"/>
      <circle cx="5" cy="6.7" r=".1"/>
      <circle cx="4.6" cy="6.4" r=".1"/>
      <circle cx="5" cy="7.2" r=".1"/>
      <circle cx="4.6" cy="6.9" r=".1"/>
      <circle cx="5" cy="7.7" r=".1"/>
      <circle cx="4.6" cy="7.5" r=".1"/>
      <circle cx="5" cy="8.3" r=".1"/>
      <circle cx="4.6" cy="8" r=".1"/>
      <circle cx="5" cy="8.8" r=".1"/>
      <circle cx="4.6" cy="8.5" r=".1"/>
      <circle cx="5" cy="9.3" r=".1"/>
      <circle cx="4.6" cy="9.1" r=".1"/>
      <circle cx="5" cy="9.8" r=".1"/>
      <circle cx="4.6" cy="9.6" r=".1"/>
      <circle cx="5.4" cy="3.2" r=".1"/>
      <circle cx="5.8" cy="4" r=".1"/>
      <circle cx="5.4" cy="3.7" r=".1"/>
      <circle cx="5.8" cy="4.5" r=".1"/>
      <circle cx="5.4" cy="4.3" r=".1"/>
      <circle cx="5.8" cy="5.1" r=".1"/>
      <circle cx="5.4" cy="4.8" r=".1"/>
      <circle cx="5.8" cy="5.6" r=".1"/>
      <circle cx="5.4" cy="5.3" r=".1"/>
      <circle cx="5.8" cy="6.1" r=".1"/>
      <circle cx="5.4" cy="5.9" r=".1"/>
      <circle cx="5.8" cy="6.7" r=".1"/>
      <circle cx="5.4" cy="6.4" r=".1"/>
      <circle cx="5.8" cy="7.2" r=".1"/>
      <circle cx="5.4" cy="6.9" r=".1"/>
      <circle cx="5.8" cy="7.7" r=".1"/>
      <circle cx="5.4" cy="7.5" r=".1"/>
      <circle cx="5.8" cy="8.3" r=".1"/>
      <circle cx="5.4" cy="8" r=".1"/>
      <circle cx="5.8" cy="8.8" r=".1"/>
      <circle cx="5.4" cy="8.5" r=".1"/>
      <circle cx="5.8" cy="9.3" r=".1"/>
      <circle cx="5.4" cy="9.1" r=".1"/>
      <circle cx="5.8" cy="9.8" r=".1"/>
      <circle cx="5.4" cy="9.6" r=".1"/>
      <circle cx="6.6" cy="4" r=".1"/>
      <circle cx="6.2" cy="3.7" r=".1"/>
      <circle cx="6.6" cy="4.5" r=".1"/>
      <circle cx="6.2" cy="4.3" r=".1"/>
      <circle cx="6.6" cy="5.1" r=".1"/>
      <circle cx="6.2" cy="4.8" r=".1"/>
      <circle cx="6.6" cy="5.6" r=".1"/>
      <circle cx="6.2" cy="5.3" r=".1"/>
      <circle cx="6.6" cy="6.1" r=".1"/>
      <circle cx="6.2" cy="5.9" r=".1"/>
      <circle cx="6.6" cy="6.7" r=".1"/>
      <circle cx="6.2" cy="6.4" r=".1"/>
      <circle cx="6.6" cy="7.2" r=".1"/>
      <circle cx="6.2" cy="6.9" r=".1"/>
      <circle cx="6.6" cy="7.7" r=".1"/>
      <circle cx="6.2" cy="7.5" r=".1"/>
      <circle cx="6.6" cy="8.3" r=".1"/>
      <circle cx="6.2" cy="8" r=".1"/>
      <circle cx="6.6" cy="8.8" r=".1"/>
      <circle cx="6.2" cy="8.5" r=".1"/>
      <circle cx="6.6" cy="9.3" r=".1"/>
      <circle cx="6.2" cy="9.1" r=".1"/>
      <circle cx="6.2" cy="9.6" r=".1"/>
      <circle cx="7.4" cy="5.1" r=".1"/>
      <circle cx="7" cy="4.8" r=".1"/>
      <circle cx="7.4" cy="5.6" r=".1"/>
      <circle cx="7" cy="5.3" r=".1"/>
      <circle cx="7.4" cy="6.1" r=".1"/>
      <circle cx="7" cy="5.9" r=".1"/>
      <circle cx="7.4" cy="6.7" r=".1"/>
      <circle cx="7" cy="6.4" r=".1"/>
      <circle cx="7.4" cy="7.2" r=".1"/>
      <circle cx="7" cy="6.9" r=".1"/>
      <circle cx="7.4" cy="7.7" r=".1"/>
      <circle cx="7" cy="7.5" r=".1"/>
      <circle cx="7.4" cy="8.3" r=".1"/>
      <circle cx="7" cy="8" r=".1"/>
      <circle cx="7.4" cy="8.8" r=".1"/>
      <circle cx="7" cy="8.5" r=".1"/>
      <circle cx="7" cy="9.1" r=".1"/>
    </g>
    <path d="M3.6-.6c.8 0 3.7 1.5 4 4 .3 3.4-1.7 4.5-4 4.5-2.6 0-4-.7-4-4.2s3.2-4.4 4-4.3m0 .7C2.6.1.2 1.3.2 3.8.2 5 .4 6 1 6.4c.5.4 1.3.6 2.5.6s2-.2 2.5-.6c.5-.5.8-1.3.8-2.6 0-2.3-2-3.6-3.2-3.7" style="fill:#f3316d"/>
  </g>`;

  // finders are drawn by hand above, so drop them before tracing the rest
  for (let i = 0; i < qr.matrix.length; i++) {
    if (qr.matrix[i] & Module.FINDER) qr.matrix[i] = 0;
  }

  svg += `<path d="${buildSvgPath(qr, margin, 1)}"/>`;
  svg += `</g></svg>`;

  return svg;
}

function circle(x: number, y: number, width: number, cw: boolean) {
  const r = width / 2;
  let svg = `M${x + r},${y}`;
  if (cw) {
    svg += `a${r},${r} 0,0,1 ${r},${r}`;
    svg += `a${r},${r} 0,0,1 -${r},${r}`;
    svg += `a${r},${r} 0,0,1 -${r},-${r}`;
    svg += `a${r},${r} 0,0,1 ${r},-${r}`;
  } else {
    svg += `a${r},${r} 0,0,0 -${r},${r}`;
    svg += `a${r},${r} 0,0,0 ${r},${r}`;
    svg += `a${r},${r} 0,0,0 ${r},-${r}`;
    svg += `a${r},${r} 0,0,0 -${r},-${r}`;
  }
  return svg;
}

function triangle(x: number, y: number, width: number, cw: boolean) {
  const h = width / 2;
  const r3 = Math.sqrt(3);
  return cw ? `M${x},${y}l${h},${h * r3}h-${width}z` : `M${x},${y}l-${h},${h * r3}h${width}z`;
}
