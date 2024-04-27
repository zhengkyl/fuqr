use fuqr::{
    encode,
    error_correction::{ECL, NUM_BLOCKS},
    place,
};

fn main() {
    let c = encode("");
    let s = place(&c);
    print!("{}", s);
}
