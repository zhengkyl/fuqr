use crate::matrix::{Matrix, Module};

// todo UNTESTED CODE: HERE BE DRAGONS
// if score is wrong for all masks, then this still works
pub fn score(matrix: &Matrix<Module>) -> u32 {
    // todo what are perf implications of scoring all masks
    // 8 masks * 5 iterations (blocks + rows are non sequential access)

    fn dark_proportion(matrix: &Matrix<Module>) -> u32 {
        let mut dark = 0;
        for y in 0..matrix.width {
            for x in 0..matrix.width {
                if matrix.get(x, y).has(Module::DATA) {
                    dark += 1;
                }
            }
        }

        let percent = (dark * 20) / (20 * (matrix.width as u32) * (matrix.width as u32));
        let middle = 50;
        let diff = if percent < middle {
            middle - percent
        } else {
            percent - middle
        };
        let k = (diff) / 5;
        10 * k
    }

    fn blocks(matrix: &Matrix<Module>) -> u32 {
        let mut score = 0;
        for y in 0..matrix.width - 1 {
            for x in 0..matrix.width - 1 {
                let curr = matrix.get(x, y).has(Module::ON);
                let tr = matrix.get(x + 1, y).has(Module::ON);
                let bl = matrix.get(x, y + 1).has(Module::ON);
                let br = matrix.get(x + 1, y + 1).has(Module::ON);
                if curr == tr && curr == bl && curr == br {
                    score += 3;
                }
            }
        }
        score
    }

    // detects streaks >= 5 and finder patterns
    fn line_patterns(matrix: &Matrix<Module>, col: bool) -> u32 {
        let mut score = 0;
        let (y_mult, x_mult) = match col {
            true => (matrix.width, 1),
            false => (1, matrix.width),
        };

        let pattern_1 = 0b0000_1011101;
        let pattern_2 = 0b1011101_0000;

        for y in 0..matrix.width {
            let mut streak = 1;
            let mut streak_v = matrix.value[y as usize * y_mult as usize + 0].has(Module::ON);

            let mut window: u16 = streak_v as u16;

            for x in 1..matrix.width {
                let curr = matrix.value
                    [y as usize * y_mult as usize + x as usize * x_mult as usize]
                    .has(Module::ON);
                if curr == streak_v {
                    streak += 1;
                    if streak == 5 {
                        score += 3;
                    } else if streak > 5 {
                        score += 1;
                    }
                } else {
                    streak = 1;
                    streak_v = curr;
                }

                window <<= 1;
                window |= curr as u16;
                // 10 = pattern.len() - 1
                if x >= 10 {
                    window &= 0b111_1111_1111;
                    if window == pattern_1 || window == pattern_2 {
                        score += 40;
                    }
                }
            }
        }

        score
    }

    dark_proportion(matrix)
        + blocks(matrix)
        + line_patterns(matrix, true)
        + line_patterns(matrix, false)
}
