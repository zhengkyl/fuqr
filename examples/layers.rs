use std::{fs::File, io::BufReader};

use fuqr::{
    generate,
    matrix::{Matrix, Module},
    QrOptions,
};
use image::{
    codecs::gif::{GifDecoder, GifEncoder},
    imageops::{self, FilterType},
    AnimationDecoder, Delay, DynamicImage, Frame, GenericImage, GenericImageView, ImageBuffer,
    ImageError, Rgba,
};

fn overlay(
    matrix:  &Matrix<Module>,
    gif_path: &str,
    out_path: &str,
    pixel_size: u32,
    bg_cover_size: u32,
    fg_cover_size: u32,
) -> Result<(), ImageError> {
    let margin = 2;
    let width = (matrix.width as u32 + margin * 2) * pixel_size;

    let out = File::create(out_path)?;
    let mut encoder = GifEncoder::new(out);
    encoder.set_repeat(image::codecs::gif::Repeat::Infinite)?;

    let overlay = BufReader::new(File::open(gif_path)?);
    let decoder = GifDecoder::new(overlay)?;
    let o_frames = decoder.into_frames();

    for o_frame in o_frames {
        let mut img_buf = ImageBuffer::from_pixel(width, width, Rgba([255, 255, 255, 255]));
        if bg_cover_size > 0 {
            img_buf = draw_qr(
                DynamicImage::ImageRgba8(img_buf),
                matrix,
                margin,
                pixel_size,
                bg_cover_size,
            )?
            .to_rgba8();
        }
        let o_frame = o_frame.unwrap();

        let ratio = o_frame.buffer().width() as f64 / o_frame.buffer().height() as f64;
        let o_width = (width as f64 * ratio) as u32;
        let o_frame = imageops::resize(o_frame.buffer(), o_width, width, FilterType::Nearest);

        imageops::overlay(
            &mut img_buf,
            &o_frame,
            (width as i64 - o_width as i64) / 2,
            0,
        );

        if fg_cover_size > 0 {
            img_buf = draw_qr(
                DynamicImage::ImageRgba8(img_buf),
                matrix,
                margin,
                pixel_size,
                fg_cover_size,
            )?
            .to_rgba8();
        }

        let frame = Frame::from_parts(img_buf, 0, 0, Delay::from_numer_denom_ms(1000, 6));
        encoder.encode_frame(frame)?;
    }

    Ok(())
}

fn background(matrix:  &Matrix<Module>) -> Result<(), ImageError> {
    let img = image::open("examples/misc/jeancarloemer.jpg")?;
    let pixel_size = 6;
    let margin = 2;
    let width = (matrix.width as u32 + margin * 2) * pixel_size;

    let img = img
        .resize(
            width,
            width,
            // Nearest is fastest and noisiest resize filter
            FilterType::Nearest,
        )
        .grayscale();

    let img = draw_qr(img, matrix, margin, pixel_size, 2)?;
    img.save("tmp/layers_background.png")?;

    Ok(())
}

fn draw_qr(
    mut img: DynamicImage,
    matrix:  &Matrix<Module>,
    margin: u32,
    pixel_size: u32,
    cover_size: u32,
) -> Result<DynamicImage, ImageError> {
    let size = (matrix.width as u32 + margin * 2) * pixel_size;
    assert_eq!(size, img.width());
    assert_eq!(size, img.height());

    let gap = (pixel_size - cover_size) / 2;

    let luma = img
        .resize(
            matrix.width as u32,
            matrix.width as u32,
            FilterType::Nearest,
        )
        .grayscale();

    for y in 0..matrix.width {
        for x in 0..matrix.width {
            let module = matrix.get(x, y);
            let on = module.has(Module::ON);
            let pixel = if on {
                [0, 0, 0, 255]
            } else {
                [255, 255, 255, 255]
            };

            if module.has(Module::FINDER) {
                for dy in 0..pixel_size {
                    for dx in 0..pixel_size {
                        img.put_pixel(
                            (x as u32 + margin) * pixel_size + dx,
                            (y as u32 + margin) * pixel_size + dy,
                            image::Rgba(pixel),
                        )
                    }
                }
            }
            // QR code scanners use local blackpoint thresholds,
            // or at least a global blackpoint based on image heuristics
            // We'll keep things simple
            let l = luma.get_pixel(x as u32, y as u32).0[0];
            if (on && l > 200) || (!on && l < 50) {
                continue;
            }

            for dy in 0..cover_size {
                for dx in 0..cover_size {
                    img.put_pixel(
                        (x as u32 + margin) * pixel_size + dx + gap,
                        (y as u32 + margin) * pixel_size + dy + gap,
                        image::Rgba(pixel),
                    )
                }
            }
        }
    }

    Ok(img)
}

fn main() -> Result<(), ImageError> {
    let qr_code = generate("https://github.com/zhengkyl/fuqr", &QrOptions::new()).unwrap();

    overlay(
        &qr_code.matrix,
        "examples/misc/spin.gif",
        "tmp/layers_min.gif",
        6,
        3,
        0,
    )?;

    overlay(
        &qr_code.matrix,
        "examples/misc/4floss.gif",
        "tmp/layers_max.gif",
        6,
        6,
        1,
    )?;

    background(&qr_code.matrix)?;

    Ok(())
}
