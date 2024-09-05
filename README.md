# fuqr

freshly unearthed qr codes

## Examples

All example code is WIP and in a very unpolished state.

### `/examples/scale.rs`

Scaling modules based on position.

| Circle                                 | Stripes                                  | Waves                                |
| -------------------------------------- | ---------------------------------------- | ------------------------------------ |
| ![circle](./examples/scale_circle.png) | ![stripes](./examples/scale_stripes.gif) | ![waves](./examples/scale_waves.gif) |

### `/examples/weave.rs`

No need to stick to a boring pixel grid.

| Thick                                | Thin                               | Diagonal                               |
| ------------------------------------ | ---------------------------------- | -------------------------------------- |
| ![thick](./examples/weave_thick.png) | ![thin](./examples/weave_thin.png) | ![diagonal](./examples/weave_diag.png) |

### `/examples/layers.rs`

Layering is neat, but it can seriously degrade scanning ability if done without care.

See [Halftone QR Codes](https://cgv.cs.nthu.edu.tw/projects/Recreational_Graphics/Halftone_QRCodes), [Micrography QR Codes](https://cgv.cs.nthu.edu.tw/projects/Recreational_Graphics/MQRC), [Amazing QR](https://github.com/x-hw/amazing-qr) for more thoughtful implementations with high scannability.

| Background                                      | Minimalist                         | Improved scannability                  |
| ----------------------------------------------- | ---------------------------------- | -------------------------------------- |
| ![background](./examples/layers_background.png) | ![thin](./examples/layers_min.gif) | ![diagonal](./examples/layers_max.gif) |

### Misc bugs and experiments

| Have                                        | Some                                      | More                            |
| ------------------------------------------- | ----------------------------------------- | ------------------------------- |
| ![bathroom](./examples/misc/bathroom.png)   | ![diamonds](./examples/misc/diamonds.gif) | ![mmm](./examples/misc/mmm.png) |
| ![mountains](./examples/misc/mountains.png) | ![diamonds](./examples/misc/zebra.gif)    |                                 |

## Other

- Great QR code generator tutorial
  - https://www.thonky.com/qr-code-tutorial/
- Reference generator implementations
  - https://github.com/erwanvivien/fast_qr
  - https://github.com/unjs/uqr
- Reference scanner implementations
  - https://github.com/zxing/zxing
  - https://github.com/glassechidna/zxing-cpp (fork of zxing's cpp port)
  - https://github.com/opencv/opencv_contrib/tree/4.x/modules/wechat_qrcode (fork of zxing-cpp with "massive upgrades")

### TODO

- [ ] send typed array to wasm

### Benchmarks

There are definitely easy perf wins, but I was surprised how decent this performs. Rather, why is the `qrcode` crate so slow?

| Test     | Implementation | Time (µs) / (ms)   | Compared to `fast_qr` |
| -------- | -------------- | ------------------ | --------------------- |
| **V03H** | fuqr           | 73.210 - 76.356 µs | ~1.1 slower           |
|          | qrcode         | 505.68 - 517.48 µs | ~7.4 slower           |
|          | fast_qr        | 69.313 - 71.027 µs | 1.0 (Fastest)         |
| **V10H** | fuqr           | 363.13 - 369.72 µs | ~1.4 slower           |
|          | qrcode         | 2.2020 - 2.2414 ms | ~8.5 slower           |
|          | fast_qr        | 260.03 - 270.07 µs | 1.0 (Fastest)         |
| **V40H** | fuqr           | 2.9916 - 3.0453 ms | ~1.3 slower           |
|          | qrcode         | 21.117 - 21.508 ms | ~9.0 slower           |
|          | fast_qr        | 2.3923 - 2.4474 ms | 1.0 (Fastest)         |
