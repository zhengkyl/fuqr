use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign};

use crate::{
    constants::{FORMAT_INFO, VERSION_INFO},
    qr_code::{Mask, Version, ECL},
};

#[derive(Debug)]
pub struct Matrix<T: Copy + From<Module> + Into<Module> + BitOrAssign<Module>> {
    pub value: Vec<T>,
    pub width: usize,
}

impl<T: Copy + From<Module> + Into<Module> + BitOrAssign<Module>> Matrix<T> {
    pub fn new(version: Version, init: T) -> Self {
        let width = version.0 * 4 + 17;
        Matrix {
            value: vec![init; (width) * (width)],
            width,
        }
    }
    pub fn get(&self, x: usize, y: usize) -> T {
        self.value[y * self.width + x]
    }
    pub fn get_mut(&mut self, x: usize, y: usize) -> &mut T {
        &mut (self.value[y * self.width + x])
    }
    pub fn set(&mut self, x: usize, y: usize, value: T) {
        self.value[y * self.width + x] = value;
    }

    pub fn set_finder(&mut self) {
        for (x, mut y) in [(0, 0), (0, self.width - 7), (self.width - 7, 0)] {
            for i in 0..7 {
                self.set(x + i, y, (Module::FINDER | Module::ON).into());
            }
            y += 1;

            self.set(x + 0, y, (Module::FINDER | Module::ON).into());
            for i in 1..6 {
                self.set(x + i, y, Module::FINDER.into());
            }
            self.set(x + 6, y, (Module::FINDER | Module::ON).into());
            y += 1;

            for _ in 0..3 {
                self.set(x + 0, y, (Module::FINDER | Module::ON).into());
                self.set(x + 1, y, (Module::FINDER).into());
                self.set(x + 2, y, (Module::FINDER_CENTER | Module::ON).into());
                self.set(x + 3, y, (Module::FINDER_CENTER | Module::ON).into());
                self.set(x + 4, y, (Module::FINDER_CENTER | Module::ON).into());
                self.set(x + 5, y, (Module::FINDER).into());
                self.set(x + 6, y, (Module::FINDER | Module::ON).into());
                y += 1;
            }

            self.set(x + 0, y, (Module::FINDER | Module::ON).into());
            for i in 1..6 {
                self.set(x + i, y, (Module::FINDER).into());
            }
            self.set(x + 6, y, (Module::FINDER | Module::ON).into());
            y += 1;

            for i in 0..7 {
                self.set(x + i, y, (Module::FINDER | Module::ON).into());
            }
        }
    }

    pub fn set_alignment(&mut self) {
        let version = (self.width - 17) / 4;
        if version == 1 {
            return;
        }

        let first = 6;
        let last = self.width - 7;
        let len = version / 7 + 2;
        let mut coords = Vec::with_capacity(len);

        coords.push(first);
        if version >= 7 {
            for i in (1..len - 1).rev() {
                coords.push((last - i * ALIGN_OFFSETS[version - 7]) as usize);
            }
        }
        coords.push(last);

        for i in 0..len {
            for j in 0..len {
                if (i == 0 && (j == 0 || j == len - 1)) || (i == len - 1 && j == 0) {
                    continue;
                }

                let col = coords[i] - 2;
                let row = coords[j] - 2;

                for i in 0..5 {
                    self.set(col, row + i, (Module::ALIGNMENT | Module::ON).into());
                }

                for i in 1..4 {
                    self.set(col + i, row + 0, (Module::ALIGNMENT | Module::ON).into());
                    self.set(col + i, row + 1, (Module::ALIGNMENT).into());
                    self.set(col + i, row + 2, (Module::ALIGNMENT).into());
                    self.set(col + i, row + 3, (Module::ALIGNMENT).into());
                    self.set(col + i, row + 4, (Module::ALIGNMENT | Module::ON).into());
                }

                self.set(
                    col + 2,
                    row + 2,
                    (Module::ALIGNMENT_CENTER | Module::ON).into(),
                );

                for i in 0..5 {
                    self.set(col + 4, row + i, (Module::ALIGNMENT | Module::ON).into())
                }
            }
        }
    }

    pub fn set_timing(&mut self) {
        // overlaps with alignment pattern so must |=
        let len = self.width - 16;
        for i in 0..len {
            let module = Module::TIMING | ((i as u8 & 1) ^ 1).into();
            *self.get_mut(8 + i, 6) |= module;
        }
        for i in 0..len {
            let module = Module::TIMING | ((i as u8 & 1) ^ 1).into();
            *self.get_mut(6, 8 + i) |= module;
        }
    }

    pub fn set_format(&mut self, ecl: ECL, mask: Mask) {
        let format_info = FORMAT_INFO[ecl as usize][mask as usize];
        for i in 0..15 {
            let on = ((format_info >> i) as u8 & 1).into();

            let y = match i {
                i if i < 6 => i,
                6 => 7,
                _ => 8,
            };
            let x = match i {
                i if i < 8 => 8,
                8 => 7,
                _ => 14 - i,
            };
            self.set(x, y, (Module::FORMAT | on).into());

            let y = match i {
                i if i < 8 => 8,
                _ => self.width - (15 - i),
            };
            let x = match i {
                i if i < 8 => self.width - (i + 1),
                _ => 8,
            };
            self.set(x, y, (Module::FORMAT_COPY | on).into());
        }

        // always set bit, not part of format info
        self.set(8, self.width - 8, (Module::FORMAT_COPY | Module::ON).into());
    }

    pub fn set_version(&mut self) {
        let version = (self.width - 17) / 4;
        if version < 7 {
            return;
        }
        let info = VERSION_INFO[version];

        for i in 0..18 {
            let on = ((info >> i) as u8 & 1).into();

            let x = i / 3;
            let y = i % 3;

            self.set(x, y + self.width - 11, (Module::VERSION | on).into());
            self.set(y + self.width - 11, x, (Module::VERSION_COPY | on).into());
        }
    }

    /// This must run AFTER alignment, timing, version placed
    pub fn set_data(&mut self, mut get_value: impl FnMut() -> T) {
        let mut col = self.width - 1;
        let mut row = self.width - 1;

        let mut row_dir = -1;
        let mut row_end = 9;

        let mut row_len = (self.width - 10) as isize;

        loop {
            loop {
                if self.get(col, row).into() == Module(0) {
                    self.set(col, row, get_value());
                }
                if self.get(col - 1, row).into() == Module(0) {
                    self.set(col - 1, row, get_value());
                }
                if row == row_end {
                    break;
                }
                row = ((row as isize) + row_dir) as usize;
            }

            if col == 1 {
                break;
            }

            col -= 2;
            row_dir *= -1;

            // passed first finder
            if col == self.width - 9 {
                row_len = (self.width - 1) as isize;
                row_end = 0;
            }
            // between left finders
            else if col == 8 {
                row_len = (self.width - 18) as isize;
                row_end = 9;
                row = self.width - 9;
            } else {
                // vertical timing belt
                if col == 6 {
                    col -= 1;
                }
                row_end = (row_end as isize + row_len * row_dir) as usize;
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct Module(pub u8);

impl Module {
    // bit flags
    pub const ON: Module = Module(1 << 0);
    pub const DATA: Module = Module(1 << 1);
    pub const FINDER: Module = Module(1 << 2);
    pub const ALIGNMENT: Module = Module(1 << 3);
    pub const TIMING: Module = Module(1 << 4);
    pub const FORMAT: Module = Module(1 << 5);
    pub const VERSION: Module = Module(1 << 6);
    pub const MODIFIER: Module = Module(1 << 7);

    // modified flags
    pub const FINDER_CENTER: Module = Module(Module::FINDER.0 | Module::MODIFIER.0);
    pub const ALIGNMENT_CENTER: Module = Module(Module::ALIGNMENT.0 | Module::MODIFIER.0);
    pub const FORMAT_COPY: Module = Module(Module::FORMAT.0 | Module::MODIFIER.0);
    pub const VERSION_COPY: Module = Module(Module::VERSION.0 | Module::MODIFIER.0);

    /// Returns true if self contains all flags set in `flags`, aka a superset
    pub fn has(self, flags: Module) -> bool {
        (self & flags) == flags
    }
    /// Returns true if self contains any flag set in `flags`, aka an intersection
    pub fn any(self, flag: Module) -> bool {
        (self & flag) != Module(0)
    }
    pub fn set(&mut self, flags: Module) {
        *self |= flags;
    }
}

impl From<u8> for Module {
    fn from(value: u8) -> Self {
        Module(value)
    }
}

impl BitAnd for Module {
    type Output = Module;

    fn bitand(self, rhs: Self) -> Self::Output {
        Module(self.0 & rhs.0)
    }
}

impl BitAndAssign for Module {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitOr for Module {
    type Output = Module;

    fn bitor(self, rhs: Self) -> Self::Output {
        Module(self.0 | rhs.0)
    }
}

impl BitOrAssign for Module {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitXor for Module {
    type Output = Module;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Module(self.0 ^ rhs.0)
    }
}

impl BitXorAssign for Module {
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}

const ALIGN_OFFSETS: [usize; 34] = [
    16, 18, 20, 22, 24, 26, 28, // 7-13
    20, 22, 24, 24, 26, 28, 28, // 14-20
    22, 24, 24, 26, 26, 28, 28, // 21-27
    24, 24, 26, 26, 26, 28, 28, // 28-34
    24, 26, 26, 26, 28, 28, // 35-40
];
