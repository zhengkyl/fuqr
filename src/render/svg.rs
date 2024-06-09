use super::{RenderData, Toggle};
use crate::matrix::QrMatrix;

pub fn render_svg(render: &RenderData) -> String {
    let unit_width = render.matrix.width() as f64 * render.unit as f64;
    let unit_height = render.matrix.height() as f64 * render.unit as f64;

    // TODO better initial capacity
    // guestimate, roughly half of pixels are black
    let mut output = String::with_capacity(40 * render.matrix.width() * render.matrix.height() / 2);
    output.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}">"#,
        unit_width, unit_height
    ));

    if render.get(Toggle::Background) {
        output.push_str(&format!(
            r#"<rect width="{}" height="{}" fill="{}"/>"#,
            unit_width, unit_height, render.background
        ));
    }

    if render.get(Toggle::BackgroundPixels) {
        render_pixels(render, &mut output, false);
    }

    if render.get(Toggle::ForegroundPixels) {
        render_pixels(render, &mut output, true);
    }

    output.push_str("</svg>");

    output
}

fn render_pixels(render: &RenderData, output: &mut String, on: bool) {
    output.push_str(&format!("<path fill=\"{}\" d=\"", render.foreground));

    for y in 0..render.matrix.height() {
        for x in 0..render.matrix.width() {
            let x_scale = render.scale_x_matrix[y * render.matrix.width() + x];
            let y_scale = render.scale_y_matrix[y * render.matrix.width() + x];

            let module_type = render.matrix.get(x, y);

            if (module_type as u8 & 1 != on as u8) || x_scale == 0 || y_scale == 0 {
                continue;
            }

            let x_module_size = (x_scale as f64) / 100.0 * render.unit as f64;
            let y_module_size = (y_scale as f64) / 100.0 * render.unit as f64;

            // keep module centered if size != unit
            output.push_str(&format!(
                "M{},{}h{}v{}h-{}z",
                x as f64 * render.unit as f64 + (render.unit as f64 - x_module_size) / 2.0,
                y as f64 * render.unit as f64 + (render.unit as f64 - y_module_size) / 2.0,
                x_module_size,
                y_module_size,
                x_module_size
            ));
        }
    }
    output.push_str("\"/>");
}
