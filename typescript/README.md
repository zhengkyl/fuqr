# fuqr

A small and hackable QR code generator

## Install

```sh
pnpm add fuqr
```

Or simply copy `src/fuqr.ts`.

## Usage

```js
import { generate, renderCanvas, renderSvg } from "fuqr";

const qr = generate("https://github.com/zhengkyl/fuqr");

// as svg string
const svg = renderSvg(qr);

// with options
const svg = renderSvg(qr, {
  margin: 2,
  attributes: 'xmlns="http://www.w3.org/2000/svg" width="300px" height="300px"'
});

// OR

// as HTML canvas
const canvas = renderCanvas(qr);

// with options
const canvas = renderCanvas(qr, {
  margin: 2,
  scale: 10,
  canvas: document.createElement("canvas"),
});

// canvas supports "image/png" and "image/jpeg"
const url = canvas.toDataURL("image/png");
// or
canvas.toBlob((blob) => {
  const url = URL.createObjectURL(blob);
  // etc
}, "image/png");
```

### Advanced

For control over encoding mode, anything that implements `Encoder` can used with `generateWithEncoder()`. For convenience, `NumericEncoder`, `AlphanumericEncoder`, and `MixedEncoder` are included in `fuqr/extras/encoders`.

The third argument to the generate functions is a list of plugins. A `Plugin` defines hooks that run during the generating process. For now, the `PixelArtPlugin` and `FittedLogoPlugin` are included in the extras folder, but behavior is not yet stabilized.

Although everything is exported, only the `generate` and `render` prefixed functions are intended for general use. However, you may find some internal functions to be very helpful.

- `buildSvgPath()` and `buildCanvasData()` provide composable rendering logic.

- `buildBlueprint()` from `fuqr/extras/blueprint` can be used to find the block and bit index of every pixel. 

- The `visit` helpers run a callback function with the coords of every pixel in a structural section. Looping through the data section, however, requires `iterateMostlyDataModules()` and manually skipping structural pixels. 