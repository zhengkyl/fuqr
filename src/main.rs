use std::fs;

use fuqr::{
    encode,
    matrix::{place_all, Matrix},
    qrcode::{Mode, Version},
    render::{svg::render_svg, text::render_utf8},
    Segment,
};

fn main() -> std::io::Result<()> {
    let qrcode = encode(
        vec![Segment {
            mode: Mode::Alphanumeric,
            text: "GREETINGS TRAVELER",
        }],
        Version(1),
    );
    let mut matrix = Matrix::new(qrcode.version.0);
    place_all(&mut matrix, &qrcode);

    fs::write("test.svg", render_svg(&matrix))?;
    println!("{}", render_utf8(&matrix));
    Ok(())
}
