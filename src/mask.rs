use crate::matrix::{Matrix, Module};

// todo UNTESTED CODE: HERE BE DRAGONS
// if score is wrong for all masks, then this still works
pub fn score(matrix: &Matrix) -> usize {
    // todo what are perf implications of scoring all masks
    // 8 masks * 5 iterations (blocks + rows are non sequential access)

    fn dark_proportion(matrix: &Matrix) -> usize {
        let dark = matrix
            .value
            .iter()
            .filter(|m| **m == Module::DataON)
            .count();

        let percent = (dark * 20) / (20 * matrix.width * matrix.width);
        let middle = 50;
        let diff = if percent < middle {
            middle - percent
        } else {
            percent - middle
        };
        let k = (diff) / 5;
        10 * k
    }

    fn blocks(matrix: &Matrix) -> usize {
        let mut score = 0;
        for i in 0..matrix.width - 1 {
            for j in 0..matrix.width - 1 {
                let curr = matrix.get(i, j) as u8 & 1;
                let tr = matrix.get(i + 1, j) as u8 & 1;
                let bl = matrix.get(i, j + 1) as u8 & 1;
                let br = matrix.get(i + 1, j + 1) as u8 & 1;
                if curr == tr && curr == bl && curr == br {
                    score += 3;
                }
            }
        }
        score
    }

    // detects streaks >= 5 and finder patterns
    fn line_patterns(matrix: &Matrix, col: bool) -> usize {
        let mut score = 0;
        let (i_mult, j_mult) = match col {
            true => (matrix.width, 1),
            false => (1, matrix.width),
        };

        let pattern_1 = 0b0000_1011101;
        let pattern_2 = 0b1011101_0000;

        for i in 0..matrix.width {
            let mut streak = 1;
            let mut streak_v = matrix.value[i * i_mult + 0] as u8 & 1;

            let mut window: u16 = streak_v as u16;

            for j in 1..matrix.width {
                let curr = matrix.value[i * i_mult + j * j_mult] as u8 & 1;
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
                if j >= 10 {
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
