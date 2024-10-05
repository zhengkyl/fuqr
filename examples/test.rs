use fuqr::{
    bit_info::{BitInfo, Info, QartCode},
    constants::{NUM_DATA_MODULES, NUM_EC_CODEWORDS},
    data::Data,
    generate,
    matrix::Module,
    qr_code::{Mask, Mode, QrCode, Version, ECL},
    render::{text::render_utf8, RenderData},
    QrOptions,
};
use image::{imageops::FilterType, GenericImageView};

fn main() {
    // let version = 2;
    // let codewords = NUM_DATA_MODULES[version] / 8;

    // let ecl = 0;
    // let ec_codewords = NUM_EC_CODEWORDS[version][ecl];
    // let data_codewords = codewords - ec_codewords;

    // println!("V{} ECL{} {}d {}e", version, ecl, data_codewords, ec_codewords);

    // return;
    //
    let mut data = Data::new_verbose(
        "https://github.com/zhengkyl/fuqr",
        Mode::Byte,
        Version(10),
        false,
        ECL::Low,
        true,
    )
    .unwrap();

    // let mut qr_code = QrCode::new(data, Some(Mask::M0));
    // let render = RenderData::new(&qr_code);
    // print!("{}", render_utf8(&render));
    // return;

    let qart = QartCode::new(&mut data, Mask::M0);
    // let bit_info = BitInfo::new(data.mode, data.version, data.ecl);

    let size = (qart.data.version.0 * 4 + 17) - (2 * 6);
    let img = image::open("examples/misc/apple_41.png").unwrap();
    let img = img
        .resize(
            size as u32,
            size as u32,
            FilterType::Nearest,
        )
        .grayscale();

    let mut dd = vec![false; size * size];
    for y in 0..size {
        for x in 0..size {
            // dd[y * size + x] = x % 7 == (y % 7) || x % 7 == (size - 1 - y) % 7;
            let t = img.get_pixel(x as u32, y as u32).0[0];
            if t < 127 {
                dd[y * size + x] = true;
            }
        }
    }

    let qr_code = qart.to_qr_code(dd);

    // data.push_bits(0, 4);
    // data.set_image_bits(&bit_info, Mask::M0, &dd);

    // let mut qr_code = QrCode::new(data, Some(Mask::M0));

    // for y in 0..qr_code.matrix.width {
    //     for x in 0..qr_code.matrix.width {
    //         // let condition = qr_code.matrix.get(x, y).has(Module::TIMING)
    //         //     || (qr_code.matrix.get(x, y).has(Module::ALIGNMENT)
    //         //         && (x < qr_code.matrix.width - 9 || y < qr_code.matrix.width - 9));
    //         let condition = false;
    //         if condition {
    //             qr_code
    //                 .matrix
    //                 .set(x, y, Module(dd[y as usize * size + x as usize] as u8));
    //         }
    //     }
    // }

    // let qr_code = generate("test", QrOptions::new()).unwrap();

    // dbg!(&qr_code);
    let render = RenderData::new(&qr_code);
    print!("{}", render_utf8(&render));
}
