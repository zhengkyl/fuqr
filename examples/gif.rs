use std::fs::File;

use fuqr::{
    data::Data,
    matrix::{Margin, Matrix, QrMatrix},
    qrcode::{Mode, Version, ECL},
    render::{image::render_image, RenderData},
};
use image::{codecs::gif::GifEncoder, Delay, Frame, ImageError, Rgba};

fn circle(matrix: &Matrix) -> Result<(), ImageError> {
    let center_x = matrix.width() / 2;
    let center_y = matrix.height() / 2;

    let max_size = 150;
    let min_size = 30;
    let max_dist = f64::sqrt(center_x as f64 * center_x as f64 + center_y as f64 + center_y as f64);
    let per_dist = (max_size - min_size) as f64 / max_dist as f64;

    let mut v = vec![100; matrix.width() * matrix.height()];
    for y in 0..matrix.height() {
        for x in 0..matrix.width() {
            let dx = isize::abs(x as isize - (center_x as isize)) as f64;
            let dy = isize::abs(y as isize - (center_y as isize)) as f64;
            let dist = f64::sqrt(dx * dx + dy * dy);

            let size = per_dist * dist;

            v[y * matrix.width() + x] = size as u8 + min_size;
        }
    }
    // By default unit = 1, meaning 1 pixel per qr code pixel
    let render = RenderData::new(&matrix).unit(10).scale_matrix(Some(&v));

    let buf: image::ImageBuffer<Rgba<u8>, Vec<u8>> =
        image::ImageBuffer::from_raw(render.width(), render.height(), render_image(&render))
            .unwrap();

    buf.save("tmp/debug.png")?;

    Ok(())
}

fn gif(matrix: &Matrix) -> Result<(), ImageError> {
    let out = File::create("tmp/waves.gif")?;
    let mut encoder = GifEncoder::new(out);
    encoder.set_repeat(image::codecs::gif::Repeat::Infinite)?;

    let period = 100;
    let middle = period / 3; // half -> smooth, anything less has an edge

    let x_period = 10;
    let x_middle = x_period / 2;
    for j in (0..50).rev() {
        let mut v = vec![100; matrix.width() * matrix.height()];
        for y in 0..matrix.height() {
            for x in 0..matrix.width() {
                let index = (x + y) as isize;
                let offset_x = isize::abs(x_middle - (x as isize % x_period));

                let pos = isize::abs(
                    middle as isize - (((index + offset_x) * 3 + (j * 2)) % period) as isize,
                );

                let s = 180 - 2 * pos;
                v[y * matrix.width() + x] = s as u8;
            }
        }
        // By default unit = 1, meaning 1 pixel per qr code pixel
        let render = RenderData::new(&matrix).unit(10).scale_matrix(Some(&v));

        let buf: image::ImageBuffer<Rgba<u8>, Vec<u8>> =
            image::ImageBuffer::from_raw(render.width(), render.height(), render_image(&render))
                .unwrap();

        // gifs are limited to 50fps, any higher and it resets to 10fps
        let frame = Frame::from_parts(buf, 0, 0, Delay::from_numer_denom_ms(1000, 30));
        encoder.encode_frame(frame)?;
    }

    Ok(())
}

fn main() {
    let data = Data::new(
        "https://github.com/zhengkyl/fuqr",
        Mode::Byte,
        Version(1),
        ECL::High,
    );

    let data = match data {
        Some(x) => x,
        None => return,
    };
    let matrix = Matrix::new(data, None, Margin::new(2));

    circle(&matrix);

    // gif(&matrix)
}
