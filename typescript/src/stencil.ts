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

// Any-overlap resize: destination pixel is set if any source pixel covering it is non-zero.
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
