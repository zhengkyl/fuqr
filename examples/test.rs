use fuqr::{
    generate,
    render::{text::render_utf8, RenderData},
    QrOptions,
};

fn main() {
    let qr_code = generate("test", QrOptions::new()).unwrap();

    dbg!(&qr_code);
    let render = RenderData::new(&qr_code);
    print!("{}", render_utf8(&render));
}
