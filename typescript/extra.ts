import { imageToStencil, type Logo } from "../gitignore/full.js";

/**
 * In older browsers (Firefox), SVG's without width/height have 0 naturalWidth/naturalHeight.
 *
 * This extracts size from viewBox, falling back to width and height, without rounding to integer values
 *
 * The dataUrl is also smaller, b/c not base64 encoded (33% overhead)
 */
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
