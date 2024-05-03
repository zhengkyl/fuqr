use fuqr::{encode, place, qr::Mode, version::Version, Segment};

fn main() {
    let c = encode(
        vec![Segment {
            mode: Mode::Alphanumeric,
            text: "GREETINGS TRAVELER",
        }],
        Version(1),
        Version(40),
    );
    let s = place(&c);
    print!("{}", s);
}
