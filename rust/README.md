# fuqr

Dependency-free, heap-free `no_std` port of [`../typescript/src/fuqr.ts`](../typescript/src/fuqr.ts).

```rust
use fuqr::{generate, render_svg_into, GenerateOptions, QrCode, SvgOptions};

let qr = generate("Hello, World!", GenerateOptions::default(), &mut [])?;

// Any core::fmt::Write sink works: String, or a fixed buffer to stay heap-free.
let mut svg = String::new();
render_svg_into(&qr, SvgOptions::default(), &mut svg)?;
```

## Sizing

This library is heap-free, so `QrCode<const N: usize>` defaults to fitting a maximum size version 40 QR code. Shrink `N`
with `modules_for` to cap memory.

```rust
// supports maximum size qr code; matrix is ~31KB
let qr = generate(content, options, &mut [])?;

// for regular urls; matrix is ~1.7kb, at most 134 chars
let qr = generate::<{ modules_for(6) }>(content, options, &mut [])?;

// for huge urls; matrix is ~4.8kb, at most 425 chars
let qr = generate::<{ modules_for(13) }>(content, options, &mut [])?;
```

`render_svg_into`'s contour-tracing scratch is also `[u8; N]`, so its stack
use scales with the same parameter.

