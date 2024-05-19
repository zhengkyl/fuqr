use std::fs;

use fuqr::{
    codewords::Codewords,
    constants::NUM_EC_CODEWORDS,
    data::{Data, Segment},
    matrix::{Matrix, Module},
    qrcode::{Mask, Mode, Version, ECL},
    render::{
        svg::{render_svg, FinderPattern, SvgOptions, Toggle},
        text::render_utf8,
    },
};

fn main() -> std::io::Result<()> {
    let data = Data::new(
        vec![
            Segment {
                mode: Mode::Byte,
                text: "你好",
            },
            Segment {
                mode: Mode::Byte,
                text: "你好",
            },
            Segment {
                mode: Mode::Byte,
                text: "你好",
            },
        ],
        Version(4),
        ECL::Low,
    );

    let data = match data {
        Some(x) => x,
        None => return Ok(()),
    };
    // todo
    // rn codewords takes over data, but could copy to allow change ecl, version
    let codewords = Codewords::new(data);
    let matrix = Matrix::new(codewords, None);
    // todo
    // func to change mask
    // fs::write(
    //     "test.svg",
    //     render_svg(
    //         &matrix,
    //         SvgOptions::new()
    //             .finder_pattern(FinderPattern::Cross)
    //             .finder_roundness(1.0)
    //             .toggle(Toggle::Invert)
    //             .toggle(Toggle::Background)
    //             .foreground("#000".into())
    //             .background("#ff0000".into()),
    //     ),
    // )?;
    println!("{}", render_utf8(&matrix));
    Ok(())
}
