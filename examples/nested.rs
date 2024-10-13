use fuqr::{
    generate, generate_qart, matrix::Module, qart::WeightPixel, qr_code::Version, QrOptions,
};
use image::{ImageBuffer, Rgb};

fn main() {
    doll();
    donut();
}

fn doll() {
    let margin = 2;

    let version = 1;
    let mut prev_width = version * 4 + 17;
    let mut prev_full_width = prev_width + 2 * margin;
    let mut prev_matrix = generate(
        "1",
        &QrOptions::new()
            .strict_ecl(true)
            .min_version(Version(version)),
    )
    .unwrap()
    .matrix;

    for (message, version) in [("2", 6), ("3", 11)] {
        let width = version * 4 + 17;

        let mut weights = vec![WeightPixel::new(false, 0); width * width];

        let pad = (width - prev_full_width) / 2;
        for y in 0..prev_full_width {
            for x in 0..prev_full_width {
                let value = if x < margin
                    || y < margin
                    || x > prev_full_width - 1 - margin
                    || y > prev_full_width - 1 - margin
                {
                    false
                } else {
                    prev_matrix.get(x - margin, y - margin).has(Module::ON)
                };

                weights[(y + pad) * width + x + pad] = WeightPixel::new(value, 127);
            }
        }

        let mut matrix = generate_qart(
            message,
            &QrOptions::new()
                .strict_ecl(true)
                .min_version(Version(version)),
            &weights,
        )
        .unwrap()
        .matrix;
        for y in 0..width {
            for x in 0..width {
                let module = matrix.get(x, y);
                let weight = weights[y * width + x];
                if (module.has(Module::TIMING) || module.has(Module::ALIGNMENT))
                    && weight.weight() > 0
                {
                    matrix.set(x, y, Module(weight.value() as u8));
                }
            }
        }

        prev_matrix = matrix;
        prev_width = version * 4 + 17;
        prev_full_width = prev_width + 2 * margin;
    }

    let scale = 5;
    let img_buf = ImageBuffer::from_fn(
        (prev_full_width * scale) as u32,
        (prev_full_width * scale) as u32,
        |x, y| {
            let x = x as usize / scale;
            let y = y as usize / scale;

            if x < margin
                || y < margin
                || x > prev_full_width - 1 - margin
                || y > prev_full_width - 1 - margin
            {
                return Rgb([255, 255, 255]);
            }
            let qr_x = (x - margin) as usize;
            let qr_y = (y - margin) as usize;
            let module = prev_matrix.get(qr_x, qr_y);

            if module.has(Module::ON) {
                Rgb([0 as u8, 0, 0])
            } else {
                Rgb([255, 255, 255])
            }
        },
    );

    img_buf.save("examples/nesting_doll.png").unwrap();
}

fn donut() {
    let qr_inner = generate("NEAR", &QrOptions::new()).unwrap();
    let qr_outer = generate("FAR", &QrOptions::new()).unwrap();

    let version = 1;
    let margin = 2;
    let width = version * 4 + 17;
    let full_width = width + 2 * margin;

    let inner_scale = 3;
    let outer_scale = 11;
    let gap = (outer_scale - inner_scale) / 2;

    let mut img_buf = ImageBuffer::from_pixel(
        full_width * outer_scale,
        full_width * outer_scale,
        Rgb([255, 255, 255]),
    );

    let scaled_margin = margin * outer_scale;
    for y in 0..width {
        for x in 0..width {
            let outer = if qr_outer.matrix.get(x as usize, y as usize).has(Module::ON) {
                Rgb([0 as u8, 0, 0])
            } else {
                Rgb([255, 255, 255])
            };
            let inner = if qr_inner.matrix.get(x as usize, y as usize).has(Module::ON) {
                Rgb([0 as u8, 0, 0])
            } else {
                Rgb([255, 255, 255])
            };

            let sy = y * outer_scale;
            let sx = x * outer_scale;
            for dy in 0..outer_scale {
                for dx in 0..outer_scale {
                    img_buf.put_pixel(
                        sx + dx + scaled_margin,
                        sy + dy + scaled_margin,
                        if dx >= gap
                            && dy >= gap
                            && dx < gap + inner_scale
                            && dy < gap + inner_scale
                        {
                            inner
                        } else {
                            outer
                        },
                    );
                }
            }
        }
    }

    img_buf.save("examples/nesting_donut.png").unwrap();
}
