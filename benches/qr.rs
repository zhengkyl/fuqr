// copied from https://github.com/erwanvivien/fast_qr/blob/master/benches/qr.rs

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use fuqr::QrOptions;
use std::hint::black_box;
use std::time::Duration;

fn bench_fastqr_qrcode(c: &mut Criterion) {
    let bytes: &[u8] = b"https://example.com/";

    for (
        id,
        fuqr_version,
        fuqr_level,
        fast_qr_version,
        fast_qr_level,
        qrcode_version,
        qrcode_level,
    ) in &[
        (
            "V03H",
            fuqr::qrcode::Version::new(3),
            fuqr::qrcode::ECL::High,
            fast_qr::Version::V03,
            fast_qr::ECL::H,
            qrcode::Version::Normal(3),
            qrcode::EcLevel::H,
        ),
        (
            "V10H",
            fuqr::qrcode::Version::new(10),
            fuqr::qrcode::ECL::High,
            fast_qr::Version::V10,
            fast_qr::ECL::H,
            qrcode::Version::Normal(10),
            qrcode::EcLevel::H,
        ),
        (
            "V40H",
            fuqr::qrcode::Version::new(40),
            fuqr::qrcode::ECL::High,
            fast_qr::Version::V40,
            fast_qr::ECL::H,
            qrcode::Version::Normal(40),
            qrcode::EcLevel::H,
        ),
    ] {
        let mut group = c.benchmark_group(*id);
        group.measurement_time(Duration::from_secs(10));
        group.throughput(Throughput::Bytes(bytes.len() as u64));
        group.sample_size(200);

        group.bench_function("fuqr", |b| {
            b.iter(|| {
                fuqr::get_matrix(
                    black_box("https://example.com/"),
                    QrOptions::new()
                        .min_ecl(*fuqr_level)
                        .min_version(*fuqr_version),
                )
            })
        });

        group.bench_function("qrcode", |b| {
            b.iter(|| {
                qrcode::QrCode::with_version(
                    black_box(b"https://example.com/"),
                    *qrcode_version,
                    *qrcode_level,
                )
                .unwrap()
            })
        });

        group.bench_function("fast_qr", |b| {
            b.iter(|| {
                fast_qr::QRBuilder::new(black_box("https://example.com/"))
                    .ecl(*fast_qr_level)
                    .version(*fast_qr_version)
                    .build()
                    .unwrap()
            })
        });

        group.finish();
    }
}

criterion_group!(benches, bench_fastqr_qrcode);
criterion_main!(benches);
