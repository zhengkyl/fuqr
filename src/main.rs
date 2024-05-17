use std::fs;

use fuqr::{
    codewords::Codewords,
    data::{Data, Segment},
    matrix::{Matrix, Module},
    qrcode::{Mask, Mode, Version, ECL, NUM_DATA_MODULES},
    render::svg::{render_svg, FinderPattern, SvgOptions, Toggle},
};

fn main() -> std::io::Result<()> {
    let data = Data::new(
        vec![Segment {
            mode: Mode::Byte,
            text: "aGREETINGS TRAVELER123456789",
        }],
        Version(2),
    );
    // todo
    // rn codewords takes over data, but could copy to allow change ecl, version
    let codewords = Codewords::new(data, ECL::Low);
    let matrix = Matrix::new(codewords, Mask::M0);
    // todo
    // func to change mask
    fs::write(
        "test.svg",
        render_svg(
            &matrix,
            SvgOptions::new()
                .finder_pattern(FinderPattern::Cross)
                .finder_roundness(1.0)
                .toggle(Toggle::Invert)
                .toggle(Toggle::Background)
                .foreground("#000".into())
                .background("#ff0000".into()),
        ),
    )?;
    // println!("{}", render_utf8(&matrix));
    Ok(())
}
