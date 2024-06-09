use super::RenderData;
use crate::matrix::QrMatrix;

pub fn render_utf8(render: &RenderData) -> String {
    // row length +1 for \n and take ceil of rows / 2 if odd
    let mut result =
        String::with_capacity((render.matrix.width() + 1) * (render.matrix.height() + 1) / 2);
    for y in (0..render.matrix.height()).step_by(2) {
        for x in 0..render.matrix.width() {
            let top = render.matrix.get(x, y) as u8 & 1 == 1;
            let bot = if y < render.matrix.height() - 1 {
                render.matrix.get(x, y + 1) as u8 & 1 == 1
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
