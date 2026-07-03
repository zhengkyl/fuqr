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
