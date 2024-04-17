use fuqr::{math::ANTILOG_TABLE, version::Version};

// 6 correct
fn main() {
    let percentages = [0.2f64, 0.37f64, 0.55f64, 0.655f64];
    for i in 1..=40 {
        let codewords = Version::new(i).num_codewords();
        for percentage in percentages {
            let lwords = codewords as f64 * percentage;
            println!(
                "{}: {:5} {:5} | {:.5}",
                i,
                codewords,
                lwords,
                lwords as f64 / codewords as f64
            );

            let mut size = 0;
            let mut blocks = 0;
            for correctable in (8..=15).rev() {
                let bytes = correctable * 2;
                let num_blocks = (lwords / bytes as f64) as u32;
                let under = num_blocks * bytes;
                let over = (num_blocks + 1) * bytes;
                let under_p = under as f64 / codewords as f64;
                let over_p = over as f64 / codewords as f64;

                // pick lesser diff, unless 3 or 5 num_blocks
                let over_diff = over_p - percentage;
                let under_diff = percentage - under_p;
                blocks = num_blocks;

                if under_diff <= 0.00369 {
                    size = under;
                    break;
                } else if over_diff <= 0.00690 {
                    size = over;
                    break;
                }

                // println!(
                //
                //     "{:2}\t{:5} | {:.5} | {:5} | {:.5}",
                //     correctable,
                //     under,
                //     (under as f64 / codewords as f64),
                //     over,
                //     (over as f64 / codewords as f64)
                // );
            }

            println!("{:5} {}", size, blocks);
        }
        println!("\n");
    }
}
