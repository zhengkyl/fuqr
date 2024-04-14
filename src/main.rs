use fuqr::{math::ANTILOG_TABLE, version::Version};

// 6 correct
fn main() {
    for i in 0..256 {
        println!("{}, {}", i, ANTILOG_TABLE[i]);
    }
    // for i in 1..=40 {
    //     let codewords = Version::new(i).num_codewords();
    //     println!("{}", codewords);
    //     // println!("{}", codewords * 21 / 100);
    //     // println!("{}", 2 * codewords * 15 / 100);
    //     // println!("{}", 2 * codewords * 25 / 100);
    //     // println!("{}\n", 2 * codewords * 30 / 100);
    // }
}
