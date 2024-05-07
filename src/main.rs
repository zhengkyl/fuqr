use std::fs;

use fuqr::{
    codewords::Codewords,
    data::{Data, Segment},
    matrix::Matrix,
    qrcode::{Mask, Mode, Version, ECL},
    render::{svg::render_svg, text::render_utf8},
};

fn main() -> std::io::Result<()> {
    let data = Data::new(
        vec![Segment {
            mode: Mode::Alphanumeric,
            text: "GREETINGS TRAVELER",
        }],
        Version(1),
    );
    // todo
    // rn codewords takes over data, but could copy to allow change ecl, version
    let codewords = Codewords::new(data, ECL::Low);
    let matrix = Matrix::new(codewords, Mask(0));
    // todo
    // func to change mask

    fs::write("test.svg", render_svg(&matrix))?;
    println!("{}", render_utf8(&matrix));
    Ok(())
}
