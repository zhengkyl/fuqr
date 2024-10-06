use fuqr::{
    generate,
    matrix::{Matrix, Module},
    QrOptions,
};
use image::ImageError;

fn weave(
    matrix: &Matrix<Module>,
    gap: u32,
    flip: bool,
) -> image::ImageBuffer<image::Rgb<u8>, Vec<u8>> {
    let density: u32 = 11;
    let margin = 2;
    let size = matrix.width + margin + margin;
    let mut img_buf = image::ImageBuffer::from_pixel(
        size as u32 * density,
        size as u32 * density,
        image::Rgb([180 as u8, 180, 180]),
    );

    let black = [0 as u8, 0, 0];
    let white = [255 as u8, 255, 255];

    for y in 0..size {
        for x in 0..size {
            let in_qr = y >= margin
                && y < matrix.width + margin
                && x >= margin
                && x < matrix.width + margin;

            let px = y as u32 * density;
            let py = x as u32 * density;

            let gap = if in_qr
                && matrix
                    .get(x - margin, y - margin)
                    .has(Module::FINDER_CENTER)
            {
                0
            } else {
                gap
            };

            let (black, white) = if flip { (white, black) } else { (black, white) };

            if (in_qr && matrix.get(x - margin, y - margin).has(Module::ON)) ^ flip {
                for dx in 0..density {
                    for dy in gap..density - gap {
                        let p = img_buf.get_pixel_mut(px + dx, py + dy);
                        *p = image::Rgb(black);
                    }
                }
                for dx in gap..density - gap {
                    for dy in 0..gap {
                        let p = img_buf.get_pixel_mut(px + dx, py + dy);
                        *p = image::Rgb(white);
                    }
                    for dy in density - gap..density {
                        let p = img_buf.get_pixel_mut(px + dx, py + dy);
                        *p = image::Rgb(white);
                    }
                }
            } else {
                for dy in 0..density {
                    for dx in gap..density - gap {
                        let p = img_buf.get_pixel_mut(px + dx, py + dy);
                        *p = image::Rgb(white);
                    }
                }
                for dy in gap..density - gap {
                    for dx in 0..gap {
                        let p = img_buf.get_pixel_mut(px + dx, py + dy);
                        *p = image::Rgb(black);
                    }
                    for dx in density - gap..density {
                        let p = img_buf.get_pixel_mut(px + dx, py + dy);
                        *p = image::Rgb(black);
                    }
                }
            }
        }
    }
    img_buf
}

fn diag(
    matrix: &Matrix<Module>,
    d_gap: isize,
    flip: bool,
) -> image::ImageBuffer<image::Rgb<u8>, Vec<u8>> {
    let density = 11;
    let margin = 2;
    let size = matrix.width as isize + margin + margin;
    let end = matrix.width as isize + margin;

    let get_matrix = |x, y| {
        if y < margin || y >= end || x < margin || x >= end {
            return false;
        }
        return matrix
            .get((x - margin) as usize, (y - margin) as usize)
            .has(Module::ON);
    };

    let mut img_buf = image::ImageBuffer::from_pixel(
        (size * density) as u32,
        (size * density) as u32,
        image::Rgb([180 as u8, 180, 180]),
    );

    let black = [0 as u8, 0, 0];
    let white = [255 as u8, 255, 255];

    for y in 0..size {
        for x in 0..size {
            let in_qr = y >= margin && y < end && x >= margin && x < end;

            let px = x * density;
            let py = y * density;

            let (black, white) = if flip { (white, black) } else { (black, white) };

            let gap = if in_qr
                && matrix
                    .get((x - margin) as usize, (y - margin) as usize)
                    .has(Module::FINDER_CENTER)
            {
                0
            } else {
                d_gap
            };

            if get_matrix(x, y) ^ flip {
                let top_overflow = if get_matrix(x + 1, y) ^ flip { 4 } else { gap };

                let bot_overflow = if get_matrix(x, y + 1) ^ flip { 4 } else { gap };

                for dy in -(top_overflow)..density + bot_overflow {
                    if py + dy < 0 || py + dy >= size * density {
                        continue;
                    }

                    let mut start = density / 2 - dy;
                    let mut length = density;

                    if dy <= density / 2 - top_overflow {
                        length = 2 * (dy + top_overflow) + 1;
                    }
                    if dy >= density / 2 + bot_overflow {
                        start = dy - (density / 2 + bot_overflow) - bot_overflow;
                        length = 2 * (density - 1 + bot_overflow - dy) + 1;
                    }

                    for dx in start + gap..start + length - gap {
                        if (px + dx) < 0 || px + dx >= size * density {
                            continue;
                        }
                        let p = img_buf.get_pixel_mut((px + dx) as u32, (py + dy) as u32);
                        *p = image::Rgb(black);
                    }
                }
            } else {
                let top_overflow = 7;
                let bot_overflow = if !get_matrix(x, y + 1) ^ flip { 4 } else { gap };

                for dy in -(top_overflow)..density + bot_overflow {
                    if py + dy < 0 || py + dy >= size * density {
                        continue;
                    }

                    let mut start = -density / 2 + dy;
                    let mut length = density;

                    if dy <= density / 2 - top_overflow {
                        start = -dy + density / 2 - top_overflow - top_overflow;
                        length = 2 * (dy + top_overflow) + 1;
                    }
                    if dy >= density / 2 + bot_overflow {
                        length = 2 * (density - 1 + bot_overflow - dy) + 1;
                    }

                    for dx in start + gap..start + length - gap {
                        if (px + dx) < 0 || px + dx >= size * density {
                            continue;
                        }
                        let p = img_buf.get_pixel_mut((px + dx) as u32, (py + dy) as u32);
                        *p = image::Rgb(white);
                    }
                }
            }
        }
    }
    img_buf
}

fn main() -> Result<(), ImageError> {
    let qr_code = generate("https://github.com/zhengkyl/fuqr", QrOptions::new()).unwrap();

    let img_buf = weave(&qr_code.matrix, 1, false);
    img_buf.save("tmp/weave_thick.png").unwrap();

    let img_buf = weave(&qr_code.matrix, 3, true);
    img_buf.save("tmp/weave_thin.png").unwrap();

    let img_buf = diag(&qr_code.matrix, 1, false);
    img_buf.save("tmp/weave_diag.png").unwrap();
    Ok(())
}
