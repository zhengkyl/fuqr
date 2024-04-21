use fuqr::{
    ecc::num_blocks,
    encode,
    math::ANTILOG_TABLE,
    qr::{QRCode, ECL},
    version::Version,
};

// 6 correct
fn main() {
    encode("");
    // let mut qrcode = QRCode {
    //     data: Vec::new(),
    //     ecc: ECL::Low,
    //     mask: 0,
    //     version: Version(0),
    // };
    // for i in 1..=40 {
    //     qrcode.version = Version(i);
    //     println!("{}", i);
    //     for ecl in [ECL::Low, ECL::Medium, ECL::Quartile, ECL::High] {
    //         qrcode.ecc = ecl;
    //         let blocks = num_blocks(&qrcode);
    //         let group_2_diff = qrcode.version.num_codewords() as u32 % (blocks);
    //         println!("\t{}", blocks - group_2_diff);
    //         if group_2_diff > 0 {
    //             println!("\t{}", group_2_diff);
    //         }
    //     }
    // }
}
