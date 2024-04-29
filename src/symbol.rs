use std::fmt;

#[derive(Clone, Copy, PartialEq)]
pub enum MODULE {
    OFF = 0,
    ON = 1,
    UNSET = 2,
}

pub struct Symbol {
    pub width: usize,
    pub modules: Vec<MODULE>,
}

impl Symbol {
    pub fn new(version: u8) -> Self {
        let width = version as usize * 4 + 17;
        Symbol {
            width: width,
            modules: vec![MODULE::UNSET; width * width],
        }
    }
    pub fn set(&mut self, x: usize, y: usize, on: bool) {
        // todo consider layout
        // Writing data means zigzag up and down, right to left
        let i = x * self.width + y;
        self.modules[i] = if on { MODULE::ON } else { MODULE::OFF };
    }
    pub fn get(&mut self, x: usize, y: usize) -> MODULE {
        // todo consider layout
        // Writing data means zigzag up and down, right to left
        let i = x * self.width + y;
        self.modules[i]
    }
}
impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Err(e) = writeln!(f) {
            return Err(e);
        }
        for y in (0..self.width).step_by(2) {
            for x in 0..self.width {
                let top = self.modules[x * self.width + y];
                let bot = if y < self.width - 1 {
                    self.modules[x * self.width + y + 1]
                } else {
                    MODULE::OFF
                };
                let c = match (top, bot) {
                    (MODULE::ON, MODULE::ON) => '█',
                    (MODULE::ON, MODULE::OFF) => '▀',
                    (MODULE::OFF, MODULE::ON) => '▄',
                    (MODULE::OFF, MODULE::OFF) => ' ',
                    _ => unreachable!("invalid symbol"),
                };
                if let Err(e) = write!(f, "{}", c) {
                    return Err(e);
                }
            }

            if let Err(e) = writeln!(f) {
                return Err(e);
            }
        }

        Ok(())
    }
}
