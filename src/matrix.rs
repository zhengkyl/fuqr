use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign};

use crate::qrcode::Version;

#[derive(Debug)]
pub struct Matrix {
    pub value: Vec<Module>,
    pub width: u8,
}

impl Matrix {
    pub fn new(version: Version) -> Self {
        let width = version.0 * 4 + 17;
        Matrix {
            value: vec![Module(0); (width as usize) * (width as usize)],
            width,
        }
    }
    pub fn get(&self, x: u8, y: u8) -> Module {
        self.value[y as usize * self.width as usize + x as usize]
    }
    pub fn get_mut(&mut self, x: u8, y: u8) -> &mut Module {
        &mut (self.value[y as usize * self.width as usize + x as usize])
    }
    pub fn set(&mut self, x: u8, y: u8, module: Module) {
        self.value[y as usize * self.width as usize + x as usize] = module;
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
    const MODIFIER: Module = Module(1 << 7);

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
