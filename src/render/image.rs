use resvg::{tiny_skia, usvg};

use super::{svg::render_svg, RenderData};

/// Temporary implementation while api is up in the air
pub fn render_image(render: &RenderData) -> Vec<u8> {
    let svg = render_svg(render);

    let tree = usvg::Tree::from_data(svg.as_bytes(), &usvg::Options::default()).unwrap();
    let pixmap_size = tree.size().to_int_size();
    let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height()).unwrap();

    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

    pixmap.take()
}
