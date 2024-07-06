# fuqr

fearlessly utilitarian qr codes

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

See [Halftone QR Codes](http://vecg.cs.ucl.ac.uk/Projects/SmartGeometry/halftone_QR/halftoneQR_sigga13.html), [Micrography QR Codes](https://cgv.cs.nthu.edu.tw/projects/Recreational_Graphics/MQRC), [Amazing QR](https://github.com/x-hw/amazing-qr) for more thoughtful implementations with high scannability.

| Background                                      | Minimalist                         | Improved scannability                       |
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
  - https://github.com/opencv/opencv_contrib/tree/4.x/modules/wechat_qrcode (fork of zxing-cpp with massive upgrades)

