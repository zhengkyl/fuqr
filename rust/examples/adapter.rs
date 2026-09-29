// Takes [encoder, minVersion, maxVersion, minEcl, maxEcl, mask, content, svg]
// Gives version-ecl-mask-hash, then a space and the svg if svg is [margin, size, attributes], or an error code.
use std::io::{self, BufRead, Write};

use fuqr::extras::encoders::{AlphanumericEncoder, MixedEncoder, NumericEncoder};
use fuqr::{
    generate_with_encoder, render_svg_into, ByteEncoder, Encoder, FuqrError, GenerateOptions,
    Module, QrCode, SvgOptions,
};

type Request = (
    String,
    u8,
    u8,
    u8,
    u8,
    u8,
    String,
    Option<(i32, Option<String>, String)>,
);

fn generate(encoder: &mut dyn Encoder, options: GenerateOptions) -> Result<QrCode, FuqrError> {
    generate_with_encoder(encoder, options)
}

// FNV-1a 64 of dark modules as 0 or 1
fn hash(qr: &QrCode) -> u64 {
    let width = qr.version as usize * 4 + 17;
    qr.matrix[..width * width]
        .iter()
        .fold(0xcbf29ce484222325, |hash, &module| {
            (hash ^ (module & Module::ON) as u64).wrapping_mul(0x100000001b3)
        })
}

fn answer(
    (encoder, min_version, max_version, min_ecl, max_ecl, mask, content, svg): Request,
) -> String {
    let options = GenerateOptions {
        min_version,
        max_version,
        min_ecl,
        max_ecl,
        mask,
    };
    let mut scratch = vec![0; content.len()];
    let result = match encoder.as_str() {
        "numeric" => NumericEncoder::new(&content).and_then(|mut e| generate(&mut e, options)),
        "alphanumeric" => {
            AlphanumericEncoder::new(&content).and_then(|mut e| generate(&mut e, options))
        }
        "byte" => generate(&mut ByteEncoder::new(&content), options),
        "mixed" => generate(&mut MixedEncoder::new(&content, &mut scratch), options),
        _ => panic!("bad encoder {encoder}"),
    };
    let qr = match result {
        Ok(qr) => qr,
        Err(err) => return err.code().to_string(),
    };
    let mut answer = format!("{}-{}-{}-{:016x}", qr.version, qr.ecl, qr.mask, hash(&qr));
    if let Some((margin, size, attributes)) = svg {
        answer.push(' ');
        let options = SvgOptions {
            margin,
            size: size.as_deref(),
            attributes: &attributes,
        };
        render_svg_into(&qr, options, &mut answer).unwrap();
    }
    answer
}

fn main() {
    let mut out = io::BufWriter::new(io::stdout().lock());
    for line in io::stdin().lock().lines() {
        let request = serde_json::from_str(&line.unwrap()).unwrap();
        writeln!(out, "{}", serde_json::to_string(&answer(request)).unwrap()).unwrap();
        out.flush().unwrap();
    }
}
