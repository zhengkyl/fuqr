use fuqr::{
    generate_qart,
    matrix::Module,
    qart::WeightPixel,
    qr_code::{Mode, Version},
    QrOptions,
};
use image::{ImageBuffer, Rgb};

use ffmpeg::software::scaling::{context::Context, flag::Flags};
use ffmpeg_next as ffmpeg;

const USE_PATTERN: bool = true;
const QR_VERSION: usize = 13;
const X_ASPECT: usize = 4;
const Y_ASPECT: usize = 3;
// Increase padding for more control over error correction bits
const PAD_L: usize = 2;
const PAD_R: usize = 2;
const FPS: u32 = 5;
const NTH_FRAME: u32 = 30 / FPS;

const QR_WIDTH: usize = QR_VERSION * 4 + 17;
const IMG_WIDTH: usize = QR_WIDTH - (PAD_L + PAD_R);
const IMG_HEIGHT: usize = ((IMG_WIDTH * Y_ASPECT) / X_ASPECT) | 1; // force odd
const PAD_T: usize = (QR_WIDTH - IMG_HEIGHT as usize) / 2 - 1;
const PAD_B: usize = (QR_WIDTH - IMG_HEIGHT as usize) / 2 + 1;

fn main() {
    ffmpeg::init().unwrap();

    // video excluded in .gitignore
    let mut format_context = ffmpeg::format::input("examples/bad_apple/bad_apple.mp4").unwrap();

    let stream = format_context
        .streams()
        .best(ffmpeg::media::Type::Video)
        .ok_or(ffmpeg::Error::StreamNotFound)
        .unwrap();

    let video_stream_index = stream.index();
    let decoder_context =
        ffmpeg::codec::context::Context::from_parameters(stream.parameters()).unwrap();
    let mut decoder = decoder_context.decoder().video().unwrap();

    let mut scaler = Context::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        ffmpeg::format::Pixel::RGB24,
        IMG_WIDTH as u32,
        IMG_HEIGHT as u32,
        Flags::BILINEAR,
    )
    .unwrap();

    let mut frame_index = 0;

    let mut receive_and_process = |decoder: &mut ffmpeg::decoder::Video| {
        let mut decoded = ffmpeg::frame::Video::empty();
        while decoder.receive_frame(&mut decoded).is_ok() {
            if frame_index % NTH_FRAME == 0 {
                let mut rgb_frame = ffmpeg::frame::Video::empty();
                scaler.run(&decoded, &mut rgb_frame).unwrap();
                save_qr_frame(rgb_frame.data(0), rgb_frame.stride(0), frame_index);
            }
            frame_index += 1;
        }
    };

    for (stream, packet) in format_context.packets() {
        if stream.index() != video_stream_index {
            continue;
        }
        decoder.send_packet(&packet).unwrap();
        receive_and_process(&mut decoder);
    }
    decoder.send_eof().unwrap();
    receive_and_process(&mut decoder);
}

fn save_qr_frame(frame: &[u8], frame_stride: usize, frame_index: u32) {
    let mut weights = vec![WeightPixel::new(false, 0); QR_WIDTH * QR_WIDTH];
    for y in 0..IMG_HEIGHT {
        for x in 0..IMG_WIDTH {
            let offset = (y * frame_stride) + x * 3;
            let r = frame[offset];

            // SOLID BLOCKS OF COLOR BREAK RECOGNITION
            // apply different pattern to white and black sections to help
            let value = if USE_PATTERN {
                if r < 127 {
                    // black pattern
                    (x + y) % 6 != 0 || (IMG_WIDTH - 1 - x + y) % 6 != 0
                } else {
                    // white pattern
                    x % 6 == (y % 6) || x % 6 == (IMG_WIDTH - 1 - y) % 6
                }
            } else {
                r < 127
            };

            // NOTICE inverted x and swapped x and y
            // 90deg ccw rotation
            weights[(QR_WIDTH - 1 - (x + PAD_L)) * QR_WIDTH + (y + PAD_T)] =
                WeightPixel::new(value, 127);
        }
    }

    let qr_options = QrOptions::new()
        .mode(Some(Mode::Byte))
        .min_version(Version(QR_VERSION))
        .strict_version(true)
        .strict_ecl(true);
    let qr_code = generate_qart(&get_lyric(frame_index / 3), &qr_options, &weights).unwrap();

    let margin = 2;
    let out_width = QR_WIDTH + 2 * margin;
    let img_buf = ImageBuffer::from_fn(out_width as u32, out_width as u32, |rot_x, rot_y| {
        // NOTICE
        // undo rotation with 90deg cw rotation
        let x = rot_y as usize;
        let y = out_width - 1 - rot_x as usize;

        if x < margin || y < margin || x > out_width - 1 - margin || y > out_width - 1 - margin {
            return Rgb([255, 255, 255]);
        }
        let qr_x = (x - margin) as usize;
        let qr_y = (y - margin) as usize;
        let module = qr_code.matrix.get(qr_x, qr_y);

        // NOTICE PAD IS FOR ROTATED IMAGE
        let on = if (module.has(Module::TIMING) || module.has(Module::ALIGNMENT))
            && (qr_x >= PAD_T
                && qr_x <= QR_WIDTH - 1 - PAD_B
                && qr_y >= PAD_L
                && qr_y <= QR_WIDTH - 1 - PAD_R)
        {
            weights[(qr_y * QR_WIDTH) + qr_x].value().clone()
        } else {
            module.has(Module::ON)
        };
        if on {
            Rgb([0 as u8, 0, 0])
        } else {
            Rgb([255, 255, 255])
        }
    });

    img_buf
        .save(format!(
            "examples/bad_apple/frames/frame_{:05}.png",
            frame_index
        ))
        .unwrap();
}

pub fn get_lyric(deca_second: u32) -> &'static str {
    // each tenth of second
    match deca_second {
        0..=184 => "🎶",
        185..=224 => "We're no strangers to love",
        225..=239 => "You know the rules",
        240..=267 => "and so do I (do I)",
        268..=289 => "A full commitment's what I'm",
        290..=309 => "thinking of",
        310..=329 => "You wouldn't get this from",
        330..=354 => "any other guy",
        355..=377 => "I just want to tell you",
        378..=399 => "how I'm feeling",
        400..=429 => "Gotta make you understand",

        430..=449 => "Never gonna give you up",
        450..=469 => "Never gonna let you down",
        470..=489 => "Never gonna run around",
        490..=514 => "and desert you",
        515..=534 => "Never gonna make you cry",
        535..=554 => "Never gonna say goodbye",
        555..=579 => "Never gonna tell a lie",
        580..=604 => "and hurt you",

        605..=624 => "We've known each other",
        625..=647 => "for so long",
        648..=669 => "Your heart's been aching but",
        670..=689 => "you're too shy to say it (say it)",
        690..=709 => "Inside, we both know what's been",
        710..=729 => "going on (going on)",
        730..=749 => "We know the game and we're",
        750..=769 => "gonna play it",
        770..=799 => "And if you ask me",
        800..=819 => "how I'm feeling",
        820..=839 => "Don't tell me you're too",
        840..=849 => "blind to see",

        850..=869 => "Never gonna give you up",
        870..=889 => "Never gonna let you down",
        890..=909 => "Never gonna run around",
        910..=934 => "and desert you",
        935..=954 => "Never gonna make you cry",
        955..=974 => "Never gonna say goodbye",
        975..=999 => "Never gonna tell a lie",
        1000..=1019 => "and hurt you",

        1020..=1039 => "Never gonna give you up",
        1040..=1059 => "Never gonna let you down",
        1060..=1079 => "Never gonna run around",
        1080..=1104 => "and desert you",
        1105..=1124 => "Never gonna make you cry",
        1125..=1144 => "Never gonna say goodbye",
        1145..=1169 => "Never gonna tell a lie",
        1170..=1194 => "and hurt you",

        1195..=1214 => "🎶(hoooooo)🎶",
        1215..=1234 => "(give you up)",
        1235..=1259 => "🎶(hoooooo)🎶",
        1260..=1279 => "(give you up)",
        1280..=1299 => "Never gonna give, never gonna give",
        1300..=1319 => "(give you up)",
        1320..=1339 => "Never gonna give, never gonna give",
        1340..=1364 => "(give you up)",

        1365..=1384 => "We've known each other",
        1385..=1407 => "for so long",
        1408..=1429 => "Your heart's been aching but",
        1430..=1449 => "you're too shy to say it (say it)",
        1450..=1469 => "Inside, we both know what's been",
        1470..=1489 => "going on (going on)",
        1490..=1509 => "We know the game and we're",
        1510..=1529 => "gonna play it",

        1530..=1559 => "I just want to tell you",
        1560..=1581 => "how I'm feeling",
        1582..=1609 => "Gotta make you understand",

        1610..=1629 => "Never gonna give you up",
        1630..=1649 => "Never gonna let you down",
        1650..=1669 => "Never gonna run around",
        1670..=1694 => "and desert you",
        1695..=1714 => "Never gonna make you cry",
        1715..=1734 => "Never gonna say goodbye",
        1735..=1759 => "Never gonna tell a lie",
        1760..=1779 => "and hurt you",

        1780..=1799 => "Never gonna give you up",
        1800..=1819 => "Never gonna let you down",
        1820..=1839 => "Never gonna run around",
        1840..=1864 => "and desert you",
        1865..=1884 => "Never gonna make you cry",
        1885..=1904 => "Never gonna say goodbye",
        1905..=1929 => "Never gonna tell a lie",
        1930..=1949 => "and hurt you",

        1950..=1969 => "Never gonna give you up",
        1970..=1989 => "Never gonna let you down",
        1990..=2009 => "Never gonna run around",
        2010..=2034 => "and desert you",
        2035..=2054 => "Never gonna make you cry",
        2055..=2074 => "Never gonna say goodbye",
        2075..=2099 => "Never gonna tell a lie",
        2100..=2200 => "and hurt you",

        _ => unreachable!("invalid frame"),
    }
}
