use crate::matrix::Module;

use super::RenderData;

pub fn render_utf8(render: &RenderData) -> String {
    // row length +1 for \n and take ceil of rows / 2 if odd
    let mut result = String::with_capacity((render.width() + 1) * (render.width() + 1) / 2);

    let start = render.margin;
    let end = render.qr_code.matrix.width + start;

    for y in (0..render.width()).step_by(2) {
        for x in 0..render.width() {
            if x < start || x >= end {
                result.push(' ');
                continue;
            }

            let top = if y >= start && y < end {
                render
                    .qr_code
                    .matrix
                    .get(x - start, y - start)
                    .has(Module::ON)
            } else {
                false
            };

            let bot = if y + 1 >= start && y + 1 < end {
                render
                    .qr_code
                    .matrix
                    .get(x - start, y - start + 1)
                    .has(Module::ON)
            } else {
                false
            };

            let c = match (top, bot) {
                (true, true) => '█',
                (true, false) => '▀',
                (false, true) => '▄',
                (false, false) => ' ',
            };
            result.push(c);
        }
        result.push('\n');
    }
    result
}
