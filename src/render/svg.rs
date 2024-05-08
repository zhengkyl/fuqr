use crate::matrix::{Matrix, Module};

pub struct RenderOptions<'a> {
    margin: f64,
    scale: f64,
    module_size: f64,
    // module_roundness: f64,
    finder_pattern: FinderPattern,
    finder_roundness: f64,
    foreground: &'a str,
    background: &'a str,
}

impl<'a> RenderOptions<'a> {
    pub fn new() -> Self {
        RenderOptions {
            margin: 2.0,
            scale: 1.0,
            module_size: 1.0,
            finder_pattern: FinderPattern::Square,
            finder_roundness: 0.0,
            foreground: "#000",
            background: "#fff",
        }
    }
    pub fn margin(mut self, margin: f64) -> Self {
        self.margin = margin;
        self
    }
    pub fn scale(mut self, scale: f64) -> Self {
        self.scale = scale;
        self
    }
    pub fn module_size(mut self, module_size: f64) -> Self {
        self.module_size = module_size;
        self
    }
    pub fn finder_pattern(mut self, finder_pattern: FinderPattern) -> Self {
        self.finder_pattern = finder_pattern;
        self
    }
    pub fn finder_roundness(mut self, finder_roundness: f64) -> Self {
        self.finder_roundness = finder_roundness;
        self
    }
    pub fn foreground(mut self, foreground: &'a str) -> RenderOptions<'a> {
        self.foreground = foreground;
        self
    }
    pub fn background(mut self, background: &'a str) -> RenderOptions<'a> {
        self.background = background;
        self
    }
}
// pub fn background(self, background: &str) -> Self {
//     Self { background, ..self }
// }
#[derive(Clone, Copy)]
pub enum FinderPattern {
    Square,
    Circle,
    Cross,
}

pub fn render_svg(matrix: &Matrix, options: RenderOptions) -> String {
    let full_width = matrix.width as f64 * options.scale + (2.0 * options.margin);

    // todo better initial capacity
    // guestimate, roughly half of pixels are black
    let mut result = String::with_capacity(40 * matrix.width * matrix.width / 2);
    result.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {0} {0}" fill="{1}">"#,
        full_width, options.foreground
    ));
    result.push_str(&format!(
        r#"<rect height="{}" width="{}" fill="{}"/>"#,
        full_width, full_width, options.background
    ));

    let outer = 7.0 * options.scale;
    let middle = 5.0 * options.scale;
    let inner = 3.0 * options.scale;
    let finder_offset = full_width - outer - options.margin;
    let background = options.background;
    for (x, y) in [
        (options.margin, options.margin),
        (options.margin, finder_offset),
        (finder_offset, options.margin),
    ] {
        match options.finder_pattern {
            FinderPattern::Square => {
                result.push_str(&format!(
                    r#"<rect x="{x}" y="{y}" width="{outer}" height="{outer}" rx="{}"/>"#,
                    outer * options.finder_roundness / 2.0
                ));
                result.push_str(&format!(
                    r#"<rect x="{}" y="{}" width="{middle}" height="{middle}" rx="{}" fill="{background}"/>"#,
                    x + 1.0 * options.scale, y + 1.0 * options.scale, middle * options.finder_roundness / 2.0
                ));
                result.push_str(&format!(
                    r#"<rect x="{}" y="{}" width="{inner}" height="{inner}" rx="{}"/>"#,
                    x + 2.0 * options.scale,
                    y + 2.0 * options.scale,
                    inner * options.finder_roundness / 2.0
                ));
            }
            FinderPattern::Circle => {
                let cx = x + outer / 2.0;
                let cy = y + outer / 2.0;
                let outer = outer / 2.0;
                let middle = middle / 2.0;
                let inner = inner / 2.0;
                result.push_str(&format!(r#"<circle cx="{cx}" cy="{cy}" r="{outer}"/>"#,));
                result.push_str(&format!(
                    r#"<circle cx="{cx}" cy="{cy}" r="{middle}" fill="{background}"/>"#,
                ));
                result.push_str(&format!(r#"<circle cx="{cx}" cy="{cy}" r="{inner}"/>"#,));
            }
            FinderPattern::Cross => {
                let short = options.scale;
                let long = 3.0 * options.scale;
                let round = long * options.finder_roundness / 2.0;
                result.push_str(&format!(
                    r#"<rect x="{}" y="{}" width="{long}" height="{short}" rx="{round}"/>"#,
                    x + 2.0 * options.scale,
                    y
                ));
                result.push_str(&format!(
                    r#"<rect x="{}" y="{}" width="{long}" height="{short}" rx="{round}"/>"#,
                    x + 2.0 * options.scale,
                    y + 6.0 * options.scale,
                ));
                result.push_str(&format!(
                    r#"<rect x="{}" y="{}" width="{short}" height="{long}" rx="{round}"/>"#,
                    x,
                    y + 2.0 * options.scale,
                ));
                result.push_str(&format!(
                    r#"<rect x="{}" y="{}" width="{short}" height="{long}" rx="{round}"/>"#,
                    x + 6.0 * options.scale,
                    y + 2.0 * options.scale,
                ));
                result.push_str(&format!(
                    r#"<rect x="{}" y="{}" height="{inner}" width="{inner}" rx="{round}"/>"#,
                    x + 2.0 * options.scale,
                    y + 2.0 * options.scale,
                ));
            }
        }
    }

    for module_type in [
        Module::AlignmentON,
        Module::TimingON,
        Module::FormatON,
        Module::VersionON,
        Module::DataON,
    ] {
        if module_type == Module::AlignmentON && matrix.width == 1 * 4 + 17 {
            continue;
        }
        if module_type == Module::VersionON && matrix.width < 7 * 4 + 17 {
            continue;
        }
        result.push_str("<path d=\"");

        for x in 0..matrix.width {
            for y in 0..matrix.width {
                // match type and be ON
                if (matrix.get(x, y)) != module_type {
                    continue;
                }

                // keep module centered if size != scale
                result.push_str(&format!(
                    r#"M{},{}h{2}v{2}h-{2}z"#,
                    x as f64 * options.scale
                        + options.margin
                        + (options.scale - options.module_size) / 2.0,
                    y as f64 * options.scale
                        + options.margin
                        + (options.scale - options.module_size) / 2.0,
                    options.module_size
                ));
            }
        }

        result.push_str("\"/>");
    }
    result.push_str("</svg>");

    result
}
