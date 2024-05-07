use crate::matrix::{Matrix, Module};

pub fn render_svg(matrix: &Matrix) -> String {
    let margin = 4;
    let full_width = matrix.width + (2 * margin);

    // todo better initial capacity
    // guestimate, roughly half of pixels are black
    let mut result = String::with_capacity(40 * matrix.width * matrix.width / 2);
    result.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {0} {0}">"#,
        full_width
    ));
    result.push_str(&format!(
        r#"<rect height="{0}" width="{0}" fill="{1}"/>"#,
        full_width, "#fff"
    ));

    for module_type in [
        Module::FinderON,
        Module::AlignmentON,
        Module::TimingON,
        Module::FormatON,
        Module::VersionON,
        Module::DataON,
    ] {
        if module_type == Module::AlignmentON && matrix.width == 1 * 4 + 17 {
            continue;
        }
        if module_type == Module::VersionON && matrix.width < 7 * 4 + 17 {
            continue;
        }
        result.push_str("<path fill=\"#000\" d=\"");

        for x in 0..matrix.width {
            for y in 0..matrix.width {
                // match type and be ON
                if (matrix.get(x, y)) != module_type {
                    continue;
                }

                result.push_str(&format!(r#"M{},{}h1v1h-1z"#, x + margin, y + margin));
            }
        }

        result.push_str("\"/>");
    }
    result.push_str("</svg>");

    result
}
