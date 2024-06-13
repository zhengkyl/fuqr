use fuqr::{
    data::Data,
    matrix::{Margin, Matrix, QrMatrix},
    qrcode::{Mask, Mode, Version, ECL},
};
use image::ImageError;

fn weave(matrix: &Matrix, gap: u32, flip: bool) -> image::ImageBuffer<image::Rgb<u8>, Vec<u8>> {
    let density = 11;
    let mut img_buf = image::ImageBuffer::from_pixel(
        matrix.width() as u32 * density,
        matrix.height() as u32 * density,
        image::Rgb([180 as u8, 180, 180]),
    );

    let black = [0 as u8, 0, 0];
    let white = [255 as u8, 255, 255];

    for y in 0..matrix.height() {
        for x in 0..matrix.width() {
            let px = x as u32 * density;
            let py = y as u32 * density;

            let gap = if is_finder_center(&matrix, x, y) {
                0
            } else {
                gap
            };

            let (black, white) = if flip { (white, black) } else { (black, white) };

            if matrix.get(x, y).is_on() ^ flip {
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
    let density: isize = 11;
    let mut img_buf = image::ImageBuffer::from_pixel(
        matrix.width() as u32 * density as u32,
        matrix.height() as u32 * density as u32,
        image::Rgb([180 as u8, 180, 180]),
    );

    let black = [0 as u8, 0, 0];
    let white = [255 as u8, 255, 255];

    for y in 0..matrix.height() {
        for x in 0..matrix.width() {
            let px = x as isize * density;
            let py = y as isize * density;

            let (black, white) = if flip { (white, black) } else { (black, white) };

            let gap = if is_finder_center(&matrix, x, y) {
                0
            } else {
                d_gap
            };

            if matrix.get(x, y).is_on() ^ flip {
                let top_overflow = if x < matrix.width() - 1 && matrix.get(x + 1, y).is_on() ^ flip
                {
                    4
                } else {
                    gap
                };

                let bot_overflow = if x < matrix.height() - 1 && matrix.get(x, y + 1).is_on() ^ flip
                {
                    4
                } else {
                    gap
                };

                for dy in -(top_overflow)..density + bot_overflow {
                    if py + dy < 0 || py + dy >= matrix.height() as isize * density {
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
                        if (px + dx) < 0 || px + dx >= matrix.width() as isize * density {
                            continue;
                        }
                        let p = img_buf.get_pixel_mut((px + dx) as u32, (py + dy) as u32);
                        *p = image::Rgb(black);
                    }
                }
            } else {
                let top_overflow = 7;
                let bot_overflow =
                    if y < matrix.height() - 1 && !matrix.get(x, y + 1).is_on() ^ flip {
                        4
                    } else {
                        gap
                    };

                for dy in -(top_overflow)..density + bot_overflow {
                    if py + dy < 0 || py + dy >= matrix.height() as isize * density {
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
                        if (px + dx) < 0 || px + dx >= matrix.width() as isize * density {
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
    let matrix = Matrix::new(data, Some(Mask::M0), Margin::new(2));

    let img_buf = weave(&matrix, 1, false);
    img_buf.save("tmp/weave_thick.png").unwrap();

    let img_buf = weave(&matrix, 3, true);
    img_buf.save("tmp/weave_thin.png").unwrap();

    let img_buf = diag(&matrix, 1, false);
    img_buf.save("tmp/weave_diag.png").unwrap();
    Ok(())
}

fn is_finder_center(matrix: &Matrix, x: usize, y: usize) -> bool {
    if y >= matrix.margin.top + 2 && y <= matrix.margin.top + 4 {
        return (x >= matrix.margin.left + 2 && x <= matrix.margin.left + 4)
            || (x >= matrix.width() - 1 - matrix.margin.right - 4
                && x <= matrix.width() - 1 - matrix.margin.right - 2);
    }
    if y >= matrix.height() - 1 - matrix.margin.bottom - 4
        && y <= matrix.height() - 1 - matrix.margin.bottom - 2
    {
        return x >= matrix.margin.left + 2 && x <= matrix.margin.left + 4;
    }

    false
}
