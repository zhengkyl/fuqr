use fuqr::{
    encode,
    qrcode::{Mask, Mode, Version},
    symbol::place,
    Segment,
};

fn main() {
    let c = encode(
        vec![Segment {
            mode: Mode::Alphanumeric,
            text: "GREETINGS TRAVELER",
        }],
        Version(1),
    );

    for i in 0..8 {
        let s = place(&c, Mask::new(i));
        print!("{}", i);
        print!("{}", s);
        println!();
        println!();
        println!();
        println!();
    }
}
