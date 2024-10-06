use fuqr::{
    data::Data,
    matrix::Module,
    qart::{QArtCode, WeightPixel},
    qr_code::{Mask, Mode, Version, ECL},
};
use image::{ImageBuffer, Rgb};

use ffmpeg::software::scaling::{context::Context, flag::Flags};
use ffmpeg_next as ffmpeg;

const QR_VERSION: usize = 13;
const X_ASPECT: usize = 4;
const Y_ASPECT: usize = 3;
// Increase padding for more control over error correction bits
const PAD_L: usize = 3;
const PAD_R: usize = 3;
const NTH_FRAME: u32 = 6;

const QR_WIDTH: usize = QR_VERSION * 4 + 17;
const IMG_WIDTH: usize = QR_WIDTH - (PAD_L + PAD_R);
const IMG_HEIGHT: usize = ((IMG_WIDTH * Y_ASPECT) / X_ASPECT) | 1; // force odd
const PAD_T: usize = (QR_WIDTH - IMG_HEIGHT as usize) / 2 - 2;
const PAD_B: usize = (QR_WIDTH - IMG_HEIGHT as usize) / 2 + 2;

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
            if frame_index > 300 {
                return;
            }
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
    // assert_eq!(frame.len(), QR_WIDTH * QR_WIDTH * 3);
    let data = Data::new_verbose(
        "https://github.com/zhengkyl/fuqr",
        Mode::Byte,
        Version(QR_VERSION),
        false,
        ECL::Low,
        true,
    )
    .unwrap();

    let qart = QArtCode::new(data, Mask::M0);

    let mut weights = vec![WeightPixel::new(false, 0); QR_WIDTH * QR_WIDTH];
    for y in 0..IMG_HEIGHT {
        for x in 0..IMG_WIDTH {
            let offset = (y * frame_stride) + x * 3;
            let r = frame[offset];

            // SOLID BLOCKS OF COLOR BREAK RECOGNITION
            // apply different pattern to white and black sections to help
            let value = if r < 127 {
                // black pattern
                (x + y) % 8 != 0 || (IMG_WIDTH - 1 - x + y) % 8 != 0
            } else {
                // white pattern
                x % 8 == (y % 8) || x % 8 == (IMG_WIDTH - 1 - y) % 8
                // (x % 3) + (y % 3) == 0
            };

            // NOTICE inverted x and swapped x and y
            // 90deg ccw rotation
            weights[(QR_WIDTH - 1 - (x + PAD_L)) * QR_WIDTH + (y + PAD_T)] =
                WeightPixel::new(value, 127);
        }
    }

    let qr_code = qart.to_qr_code(&weights);

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
            frame_index * NTH_FRAME
        ))
        .unwrap();
}
