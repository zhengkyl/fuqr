use super::RenderData;
use crate::matrix::QrMatrix;

pub fn render_utf8(render: &RenderData) -> String {
    // row length +1 for \n and take ceil of rows / 2 if odd
    let mut result =
        String::with_capacity(((render.width() + 1) * (render.width() + 1) / 2) as usize);

    let start = render.margin as usize;
    let end = render.matrix.width() + start;

    for y in (0..render.width() as usize).step_by(2) {
        for x in 0..render.width() as usize {
            if x < start || x >= end {
                result.push(' ');
                continue;
            }

            let top = if y >= start && y < end {
                render.matrix.get(x - start, y - start).is_on()
            } else {
                false
            };

            let bot = if y + 1 >= start && y + 1 < end {
                render.matrix.get(x - start, y - start + 1) as u8 & 1 == 1
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
