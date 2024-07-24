use super::{RenderData, Toggle};
use crate::matrix::QrMatrix;

pub fn render_svg(render: &RenderData) -> String {
    let scaled_width = render.width() * render.unit as u32;
    let scaled_height = render.width() * render.unit as u32;

    // TODO better initial capacity
    // guestimate, roughly half of pixels are black
    let mut output = String::with_capacity(40 * (render.width() * render.width()) as usize / 2);
    output.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}">"#,
        scaled_width, scaled_height
    ));

    if render.toggled(Toggle::Background) {
        output.push_str(&format!(
            r#"<rect width="{}" height="{}" fill="{}"/>"#,
            scaled_width, scaled_height, render.background
        ));
    }

    if render.toggled(Toggle::BackgroundPixels) {
        render_pixels(render, &mut output, false);
    }

    if render.toggled(Toggle::ForegroundPixels) {
        render_pixels(render, &mut output, true);
    }

    output.push_str("</svg>");

    output
}

fn render_pixels(render: &RenderData, output: &mut String, on: bool) {
    output.push_str(&format!(
        "<path fill=\"{}\" d=\"",
        if on {
            &render.foreground
        } else {
            &render.background
        }
    ));

    for y in 0..render.matrix.width() {
        for x in 0..render.matrix.width() {
            let module_type = render.matrix.get(x, y);

            if module_type.is_on() != on {
                continue;
            }

            // keep module centered if size != unit
            output.push_str(&format!(
                "M{},{}h{}v{}h-{}z",
                (x as u32 + render.margin as u32) * render.unit as u32,
                (y as u32 + render.margin as u32) * render.unit as u32,
                render.unit,
                render.unit,
                render.unit
            ));
        }
    }
    output.push_str("\"/>");
}
