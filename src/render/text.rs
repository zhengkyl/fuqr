use crate::matrix::Matrix;

pub fn render_utf8(matrix: &Matrix) -> String {
    // row length +1 for \n and take ceil of rows / 2 if odd
    let mut result = String::with_capacity((matrix.width + 1) * (matrix.width + 1) / 2);
    for y in (0..matrix.width).step_by(2) {
        for x in 0..matrix.width {
            let top = matrix.get(x, y) as u8 & 1 == 1;
            let bot = if y < matrix.width - 1 {
                matrix.get(x, y + 1) as u8 & 1 == 1
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
