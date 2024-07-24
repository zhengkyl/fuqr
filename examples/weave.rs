use fuqr::{
    data::Data,
    matrix::{Matrix, QrMatrix},
    qrcode::{Mask, Mode, Version, ECL},
};
use image::ImageError;

fn weave(matrix: &Matrix, gap: u32, flip: bool) -> image::ImageBuffer<image::Rgb<u8>, Vec<u8>> {
    let density = 11;
    let margin = 2;
    let size = matrix.width() as u32 + margin + margin;
    let mut img_buf = image::ImageBuffer::from_pixel(
        size * density,
        size * density,
        image::Rgb([180 as u8, 180, 180]),
    );

    let black = [0 as u8, 0, 0];
    let white = [255 as u8, 255, 255];

    for y in 0..size {
        for x in 0..size {
            let in_qr = y >= margin
                && y < matrix.width() as u32 + margin
                && x >= margin
                && x < matrix.width() as u32 + margin;

            let px = y * density;
            let py = x * density;

            let gap = if in_qr
                && is_finder_center(&matrix, (x - margin) as usize, (y - margin) as usize)
            {
                0
            } else {
                gap
            };

            let (black, white) = if flip { (white, black) } else { (black, white) };

            if (in_qr
                && matrix
                    .get((x - margin) as usize, (y - margin) as usize)
                    .is_on())
                ^ flip
            {
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

fn diag(matrix: &Matrix, d_gap: isize, flip: bool) -> image::ImageBuffer<image::Rgb<u8>, Vec<u8>> {
    let density = 11;
    let margin = 2;
    let size = matrix.width() as isize + margin + margin;
    let end = matrix.width() as isize + margin;

    let get_matrix = |x: isize, y: isize| -> bool {
        if y < margin || y >= end || x < margin || x >= end {
            return false;
        }
        return matrix
            .get((x - margin) as usize, (y - margin) as usize)
            .is_on();
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
                && is_finder_center(&matrix, (x - margin) as usize, (y - margin) as usize)
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
    let data = Data::new(
        "https://github.com/zhengkyl/fuqr",
        Mode::Byte,
        Version(1),
        ECL::Low,
    );

    let data = match data {
        Some(x) => x,
        None => return Ok(()),
    };
    let matrix = Matrix::new(data, Some(Mask::M0));

    let img_buf = weave(&matrix, 1, false);
    img_buf.save("tmp/weave_thick.png").unwrap();

    let img_buf = weave(&matrix, 3, true);
    img_buf.save("tmp/weave_thin.png").unwrap();

    let img_buf = diag(&matrix, 1, false);
    img_buf.save("tmp/weave_diag.png").unwrap();
    Ok(())
}

fn is_finder_center(matrix: &Matrix, x: usize, y: usize) -> bool {
    if y >= 2 && y <= 4 {
        return (x >= 2 && x <= 4) || (x >= matrix.width() - 5 && x <= matrix.width() - 3);
    }
    if y >= matrix.width() - 5 && y <= matrix.width() - 3 {
        return x >= 2 && x <= 4;
    }

    false
}
