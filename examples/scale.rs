use std::fs::File;

use fuqr::{
    data::Data,
    matrix::{Matrix, QrMatrix},
    qrcode::{Mode, Version, ECL},
};
use image::{codecs::gif::GifEncoder, Delay, Frame, ImageError, Rgb, Rgba};

fn circle(matrix: &Matrix) -> Result<(), ImageError> {
    let center = matrix.width() / 2;

    let margin = 2;
    let unit = 10;
    let max_size = 18;
    let min_size = 3;
    let max_dist = f64::sqrt(2.0 * center as f64 * center as f64);
    let per_dist = (max_size - min_size) as f64 / max_dist as f64;

    let size = (matrix.width() as u32 + margin * 2) * unit;
    let mut buf: image::ImageBuffer<Rgb<u8>, Vec<u8>> =
        image::ImageBuffer::from_pixel(size, size, Rgb([255, 255, 255]));

    for y in 0..matrix.width() {
        for x in 0..matrix.width() {
            if !matrix.get(x, y).is_on() {
                continue;
            }
            let dx = isize::abs(x as isize - (center as isize)) as f64;
            let dy = isize::abs(y as isize - (center as isize)) as f64;
            let dist = f64::sqrt(dx * dx + dy * dy);

            let pixel_size = (per_dist * dist) as u32 + min_size;
            let offset = (unit as isize - pixel_size as isize) / 2;
            for dy in 0..pixel_size {
                for dx in 0..pixel_size {
                    let xi = (x as u32 + margin) * unit + dx;
                    let yi = (y as u32 + margin) * unit + dy;
                    let pixel = buf.get_pixel_mut(
                        (xi as isize + offset) as u32,
                        (yi as isize + offset) as u32,
                    );
                    *pixel = image::Rgb([0, 0, 0])
                }
            }
        }
    }

    buf.save("tmp/scale_circle.png")?;

    Ok(())
}

fn stripes(matrix: &Matrix) -> Result<(), ImageError> {
    let out = File::create("tmp/scale_stripes.gif")?;
    let mut encoder = GifEncoder::new(out);
    encoder.set_repeat(image::codecs::gif::Repeat::Infinite)?;

    let period = 100;
    let middle = period / 2;
    let unit = 10;
    let margin = 2;

    let size = (matrix.width() as u32 + margin * 2) * unit;

    for j in 0..50 {
        let mut buf: image::ImageBuffer<Rgba<u8>, Vec<u8>> =
            image::ImageBuffer::from_pixel(size, size, Rgba([255, 255, 255, 255]));

        for y in 0..matrix.width() {
            for x in 0..matrix.width() {
                if !matrix.get(x, y).is_on() {
                    continue;
                }
                let index = (x + y) as isize;
                let pos = isize::abs(middle as isize - ((index * 5 + (j * 2)) % period) as isize);
                let scale = 150 - 2 * pos;

                let pixel_size = scale as u32 * unit / 100;
                let offset = (unit as isize - pixel_size as isize) / 2;
                for dy in 0..pixel_size {
                    for dx in 0..pixel_size {
                        let xi = (x as u32 + margin) * unit + dx;
                        let yi = (y as u32 + margin) * unit + dy;
                        let pixel = buf.get_pixel_mut(
                            (xi as isize + offset) as u32,
                            (yi as isize + offset) as u32,
                        );
                        *pixel = image::Rgba([0, 0, 0, 255])
                    }
                }
            }
        }

        // gifs are limited to 50fps, any higher and it resets to 10fps
        let frame = Frame::from_parts(buf, 0, 0, Delay::from_numer_denom_ms(1000, 30));
        encoder.encode_frame(frame)?;
    }

    Ok(())
}

fn waves(matrix: &Matrix) -> Result<(), ImageError> {
    let out = File::create("tmp/scale_waves.gif")?;
    let mut encoder = GifEncoder::new(out);
    encoder.set_repeat(image::codecs::gif::Repeat::Infinite)?;

    let unit = 10;
    let margin = 2;
    let size = (matrix.width() as u32 + margin * 2) * unit;

    let period = 100;
    let middle = period / 3; // half -> smooth, anything less has an edge

    let x_period = 10;
    let x_middle = x_period / 2;
    for j in (0..50).rev() {
        let mut buf: image::ImageBuffer<Rgba<u8>, Vec<u8>> =
            image::ImageBuffer::from_pixel(size, size, Rgba([255, 255, 255, 255]));
        for y in 0..matrix.width() {
            for x in 0..matrix.width() {
                if !matrix.get(x, y).is_on() {
                    continue;
                }
                let index = (x + y) as isize;
                let offset_x = isize::abs(x_middle - (x as isize % x_period));
                let pos = isize::abs(
                    middle as isize - (((index + offset_x) * 3 + (j * 2)) % period) as isize,
                );
                let scale = 180 - 2 * pos;

                let pixel_size = scale as u32 * unit / 100;

                let offset = (unit as isize - pixel_size as isize) / 2;
                for dy in 0..pixel_size {
                    for dx in 0..pixel_size {
                        let xi = (x as u32 + margin) * unit + dx;
                        let yi = (y as u32 + margin) * unit + dy;
                        let pixel = buf.get_pixel_mut(
                            (xi as isize + offset) as u32,
                            (yi as isize + offset) as u32,
                        );
                        *pixel = image::Rgba([0, 0, 0, 255])
                    }
                }
            }
        }

        // gifs are limited to 50fps, any higher and it resets to 10fps
        let frame = Frame::from_parts(buf, 0, 0, Delay::from_numer_denom_ms(1000, 30));
        encoder.encode_frame(frame)?;
    }

    Ok(())
}

fn main() -> Result<(), ImageError> {
    let data = Data::new(
        "https://github.com/zhengkyl/fuqr",
        Mode::Byte,
        Version(1),
        ECL::High,
    );

    let data = match data {
        Some(x) => x,
        None => return Ok(()),
    };
    let matrix = Matrix::new(data, None);

    circle(&matrix)?;
    stripes(&matrix)?;
    waves(&matrix)?;

    Ok(())
}
