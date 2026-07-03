import { FvqrError, Module, type Ecl, type Plugin, type Version } from "../fuqr.ts";
import { buildBlueprint } from "./blueprint.ts";

export class FittedLogoPlugin implements Plugin {
  stencil: Stencil;
  size: number;
  naturalWidth: number;
  naturalHeight: number;
  gap: number;
  reserve: number;
  coveredPositions: number[] = [];

  constructor(logo: Logo, size: number, gap: number = 1, reserve: number = 3) {
    this.stencil = logo.stencil;
    this.size = size;
    this.naturalWidth = logo.naturalWidth;
    this.naturalHeight = logo.naturalHeight;
    this.gap = gap;
    this.reserve = reserve;
  }

  mutateCapacity(capacity: { version: Version; ecl: Ecl }, ctx: { maxVersion: Version }) {
    const fits = (version: Version, ecl: Ecl) => {
      const qrWidth = version * 4 + 17;
      const logoWidth = Math.round(this.size * qrWidth);
      const logoHeight = Math.round((logoWidth * this.naturalHeight) / this.naturalWidth);
      const x0 = Math.round((qrWidth - logoWidth) / 2);
      const y0 = Math.round((qrWidth - logoHeight) / 2);
      const coverage = resizeStencil(this.stencil, logoWidth, logoHeight).data;
      const gap = this.gap;
      const positions: number[] = [];

      for (let y = Math.max(0, y0 - gap); y < Math.min(qrWidth, y0 + logoHeight + gap); y++) {
        for (let x = Math.max(0, x0 - gap); x < Math.min(qrWidth, x0 + logoWidth + gap); x++) {
          const lx = x - x0,
            ly = y - y0;
          const cxMin = Math.max(0, lx - gap),
            cxMax = Math.min(logoWidth - 1, lx + gap);
          const cyMin = Math.max(0, ly - gap),
            cyMax = Math.min(logoHeight - 1, ly + gap);
          if (cxMin > cxMax || cyMin > cyMax) continue;
          let isCovered = false;
          outerCheck: for (let cy = cyMin; cy <= cyMax; cy++) {
            for (let cx = cxMin; cx <= cxMax; cx++) {
              if (coverage[cy * logoWidth + cx]) {
                isCovered = true;
                break outerCheck;
              }
            }
          }
          if (isCovered) positions.push(y * qrWidth + x);
        }
      }
      this.coveredPositions = positions;

      const { matrix, ecPerBlock, blocks } = buildBlueprint(version, ecl);
      const blockErrors = new Uint16Array(blocks);
      const seen = new Set<number>();
      for (const pos of positions) {
        const key = matrix[pos] >> 8;
        if (key === 0 || seen.has(key)) continue;
        seen.add(key);
        blockErrors[(key - 1) >> 8]++;
      }

      const errorCapacity = ecPerBlock >> 1;
      for (let b = 0; b < blocks; b++) {
        if (blockErrors[b] > errorCapacity - this.reserve) return false;
      }
      return true;
    };

    while (!fits(capacity.version, capacity.ecl)) {
      if (capacity.version >= ctx.maxVersion) {
        throw new FvqrError("LOGO_TOO_LARGE", "Logo covers too much of the QR to stay decodable");
      }
      capacity.version++;
    }
  }

  mutateMatrix(matrix: Uint8Array) {
    for (const pos of this.coveredPositions) {
      matrix[pos] &= ~Module.ON;
    }
  }
}

export type Stencil = {
  width: number;
  height: number;
  data: Uint8Array; // alpha channel, 1 byte per pixel
};

export type Logo = {
  dataUrl: string;
  naturalWidth: number;
  naturalHeight: number;
  stencil: Stencil;
};

// Dimensions are rounded to integers.
// In older Firefox, SVGs without width/height have 0 naturalWidth/naturalHeight.
export async function imageToLogo(file: File): Promise<Logo> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = async () => {
      const dataUrl = reader.result as string;
      const img = new Image();
      img.src = dataUrl;
      await img.decode();

      const canvas = document.createElement("canvas");
      canvas.width = img.naturalWidth;
      canvas.height = img.naturalHeight;
      const ctx = canvas.getContext("2d")!;
      ctx.drawImage(img, 0, 0);
      const rgba = ctx.getImageData(0, 0, img.naturalWidth, img.naturalHeight).data;
      const alpha = new Uint8Array(img.naturalWidth * img.naturalHeight);
      for (let i = 0; i < alpha.length; i++) alpha[i] = rgba[i * 4 + 3];

      resolve({
        dataUrl,
        naturalWidth: img.naturalWidth,
        naturalHeight: img.naturalHeight,
        stencil: resizeStencil(
          { data: alpha, width: img.naturalWidth, height: img.naturalHeight },
          69,
          69,
        ),
      });
    };
    reader.onerror = () => reject(new Error("Failed to read image"));
    reader.readAsDataURL(file);
  });
}

// like imageToLogo with svg edgecase handling
// In older browsers (Firefox), SVG's without width/height have 0 naturalWidth/naturalHeight.
// Also output dataURL is smaller b/c not b64 encoded (33% overhead)
export async function svgToLogo(file: File): Promise<Logo> {
  const text = await file.text();
  const svgTagResult = /<svg[^>]*>/.exec(text);
  if (svgTagResult == null) throw new Error("SVG is missing svg tag");
  const svgTag = svgTagResult[0];

  let naturalWidth;
  let naturalHeight;
  const viewBoxResult = /viewBox="(.*?)"/.exec(svgTag);
  if (viewBoxResult) {
    const viewBox = viewBoxResult[1].split(/[\s,]+/).map((num) => parseFloat(num));
    naturalWidth = viewBox[2] - viewBox[0];
    naturalHeight = viewBox[3] - viewBox[1];
  } else {
    const widthResult = /width="(.*?)"/.exec(svgTag);
    const heightResult = /height="(.*?)"/.exec(svgTag);
    if (!widthResult || !heightResult) {
      throw new Error("SVG needs viewBox or width and height attributes");
    }
    naturalWidth = parseFloat(widthResult[1]);
    naturalHeight = parseFloat(heightResult[1]);
  }

  // slower? but might save a few bytes vs encodeURIComponent
  // text.replace(/%/g, '%25').replace(/#/g, '%23').replace(/"/g, '%22')
  const dataUrl = `data:image/svg+xml,${encodeURIComponent(text)}`;
  const img = new Image();
  img.src = dataUrl;
  await img.decode();

  return {
    dataUrl,
    naturalWidth,
    naturalHeight,
    stencil: imageToStencil(img, naturalWidth, naturalHeight, 69, 69),
  };
}

export function resizeStencil(stencil: Stencil, toW: number, toH: number): Stencil {
  const { data, width: fromW, height: fromH } = stencil;
  const out = new Uint8Array(toW * toH);
  const scaleX = toW / fromW;
  const scaleY = toH / fromH;

  for (let fy = 0; fy < fromH; fy++) {
    const top = Math.floor(fy * scaleY);
    const bottom = Math.min(toH, Math.ceil((fy + 1) * scaleY));
    for (let fx = 0; fx < fromW; fx++) {
      if (data[fy * fromW + fx] === 0) continue;
      const left = Math.floor(fx * scaleX);
      const right = Math.min(toW, Math.ceil((fx + 1) * scaleX));
      for (let ty = top; ty < bottom; ty++) {
        for (let tx = left; tx < right; tx++) out[ty * toW + tx] = 1;
      }
    }
  }

  return { width: toW, height: toH, data: out };
}

export function imageToStencil(
  img: HTMLImageElement,
  srcW: number,
  srcH: number,
  toW: number,
  toH: number,
): Stencil {
  const rasterW = Math.max(1, Math.ceil(srcW));
  const rasterH = Math.max(1, Math.ceil(srcH));

  const canvas = document.createElement("canvas");
  canvas.width = rasterW;
  canvas.height = rasterH;
  const ctx = canvas.getContext("2d")!;
  ctx.drawImage(img, 0, 0, rasterW, rasterH);
  const rgba = ctx.getImageData(0, 0, rasterW, rasterH).data;
  const alpha = new Uint8Array(rasterW * rasterH);
  for (let i = 0; i < alpha.length; i++) alpha[i] = rgba[i * 4 + 3];

  return resizeStencil({ data: alpha, width: rasterW, height: rasterH }, toW, toH);
}
