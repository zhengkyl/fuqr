use crate::matrix::Module;

use super::{RenderData, Toggle};

pub fn render_svg(render: &RenderData) -> String {
    let mut output = String::with_capacity(40 * (render.width() * render.width()) / 2);
    output.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}">"#,
        render.width(),
        render.width()
    ));

    if render.toggled(Toggle::Background) {
        output.push_str(&format!(
            r#"<rect width="{}" height="{}" fill="{}"/>"#,
            render.width(),
            render.width(),
            render.background
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

    for y in 0..render.qr_code.matrix.width {
        for x in 0..render.qr_code.matrix.width {
            let module_on = render.qr_code.matrix.get(x, y).has(Module::ON);

            if module_on != on {
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
