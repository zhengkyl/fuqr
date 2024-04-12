use fuqr::version::Version;

// 6 correct
fn main() {
    for i in 1..=40 {
        let codewords = Version::new(i).num_codewords();
        println!("{}: {}", i, codewords);
        // println!("{}", codewords * 7 / 100);
        // println!("{}", codewords * 15 / 100);
        // println!("{}", codewords * 25 / 100);
        // println!("{}\n", codewords * 30 / 100);
    }
}
