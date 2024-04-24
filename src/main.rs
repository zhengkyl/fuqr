use fuqr::error_correction::{ECL, NUM_BLOCKS};

fn main() {
    // encode("");
    dbg!(NUM_BLOCKS[ECL::Low as usize]);
    dbg!(NUM_BLOCKS[ECL::Medium as usize]);
    dbg!(NUM_BLOCKS[ECL::Quartile as usize]);
    dbg!(NUM_BLOCKS[ECL::High as usize]);
}
