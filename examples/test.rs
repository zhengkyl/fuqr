use fuqr::{
    bit_info::{Bit, BitInfo},
    data::Data,
    generate,
    qr_code::{Mask, Mode, QrCode, Version, ECL},
    render::{text::render_utf8, RenderData},
    QrOptions,
};
use image::GenericImageView;

fn main() {

    let img = image::open("examples/misc/apple_41.png").unwrap();


    let mut data =
        Data::new_verbose("hello there", Mode::Byte, Version(6), false, ECL::Low, true).unwrap();

    let bit_info = BitInfo::new(data.mode, data.version, data.ecl);

    // for y in 0..bit_info.matrix.width {
    //     for x in 0..bit_info.matrix.width {
    //         print!(
    //             "{:>2}",
    //             if (bit_info.matrix.get(x, y).module == Bit::ERROR_CORRECTION) {
    //                 1
    //             } else {
    //                 0
    //             }
    //         );
    //     }
    //     println!();
    // }
    // return;

    let size = bit_info.matrix.width as usize;
    let mut dd = vec![false; size * size];
    for y in 0.. size {
      for x in 0..size {
        let t = img.get_pixel(x as u32, y as u32).0[0];
        if t < 127 {
          dd[y * size + x] = true;
        }
      }
    }

    data.push_bits(0, 4);
    data.set_image_bits(&bit_info, Mask::M0, &dd);

    let qr_code = QrCode::new(data, Some(Mask::M0));

    // let qr_code = generate("test", QrOptions::new()).unwrap();

    // dbg!(&qr_code);
    let render = RenderData::new(&qr_code);
    print!("{}", render_utf8(&render));
}
