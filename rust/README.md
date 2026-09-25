# fuqr

A small and hackable QR code generator

Dependency-free, heap-free `no_std` port of the Typescript library.

## Usage

```rust
use fuqr::{generate, render_svg_into, GenerateOptions, QrCode, SvgOptions};

let qr = generate("Hello, World!", GenerateOptions::default(), &mut [])?;

// Any core::fmt::Write sink works: String, or a fixed buffer to stay heap-free.
let mut svg = String::new();
render_svg_into(&qr, SvgOptions::default(), &mut svg)?;
```

### Reducing memory usage

This library is heap-free, so `QrCode<const N: usize>` defaults to fitting a maximum size version 40 QR code. Shrink `N`
with `modules_for` to cap memory.

```rust
// supports maximum size qr code; matrix is ~31KB
let qr = generate(content, options, &mut [])?;

// for up to 134 chars; matrix is ~1.7kb
let qr = generate::<{ modules_for(6) }>(content, options, &mut [])?;

// for up to 425 chars; matrix is ~4.8kb
let qr = generate::<{ modules_for(13) }>(content, options, &mut [])?;
```

`render_svg_into()`'s stack use also scales with the same parameter.

### Advanced

For control over encoding mode, anything that implements `Encoder` can be used with `generate_with_encoder()`. `NumericEncoder`, `AlphanumericEncoder`, and `MixedEncoder` (shortest mix of modes) are included in `fuqr::extras::encoders`. `ByteMode`, `NumericMode`, and `AlphanumericMode` provide each mode's header and bit lengths for custom encoders.

`fuqr::extras` requires the `extras` feature.

```toml
[dependencies]
fuqr = { version = "2", features = ["extras"] }
```

```rust
use fuqr::extras::encoders::MixedEncoder;

// MixedEncoder needs one scratch byte per content byte to stay heap-free
let mut scratch = [0u8; 64];
let qr = generate_with_encoder(&mut MixedEncoder::new(content, &mut scratch), options, &mut [])?;
```

The extra plugins from the Typescript library are not yet implemented here.
