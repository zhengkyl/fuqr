// impl fmt::Display for Symbol {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         if let Err(e) = writeln!(f) {
//             return Err(e);
//         }
//         for y in (0..self.width).step_by(2) {
//             for x in 0..self.width {
//                 let top = self.modules[x * self.width + y];
//                 let bot = if y < self.width - 1 {
//                     self.modules[x * self.width + y + 1]
//                 } else {
//                     MODULE::OFF
//                 };
//                 let c = match (top, bot) {
//                     (MODULE::ON, MODULE::ON) => '█',
//                     (MODULE::ON, MODULE::OFF) => '▀',
//                     (MODULE::OFF, MODULE::ON) => '▄',
//                     (MODULE::OFF, MODULE::OFF) => ' ',
//                     _ => unreachable!("invalid symbol"),
//                 };
//                 if let Err(e) = write!(f, "{}", c) {
//                     return Err(e);
//                 }
//             }

//             if let Err(e) = writeln!(f) {
//                 return Err(e);
//             }
//         }

//         Ok(())
//     }
// }

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
