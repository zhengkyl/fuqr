use fuqr::{
    data::Data,
    matrix::{Margin, Matrix},
    qrcode::{Mask, Mode, Version, ECL},
    render::{text::render_utf8, RenderData},
};

/// Not an actual example, just a place to debug stuff
fn main() {
    let data = Data::new("Greetings traveler", Mode::Byte, Version(1), ECL::Low);

    let data = match data {
        Some(x) => x,
        None => return,
    };
    // todo
    // rn codewords takes over data, but could copy to allow change ecl, version
    let matrix = Matrix::new(data, Some(Mask::M0), Margin::new(2));
    println!("{}", render_utf8(&RenderData::new(&matrix)));
}
