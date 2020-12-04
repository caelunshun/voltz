use glam::Vec2;
use utils::{Color, Rect};

#[doc(inline)]
pub use tiny_skia::{BlendMode, FillRule, FilterQuality, LineCap, LineJoin};

fn tsk_color(c: Color) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba(c.r, c.b, c.g, c.a).expect("invalid color")
}

fn tsk_rect(r: Rect) -> tiny_skia::Rect {
    tiny_skia::Rect::from_xywh(r.pos.x, r.pos.y, r.size.x, r.size.y).expect("invalid rectangle")
}

#[derive(Default)]
pub struct PathBuilder(tiny_skia::PathBuilder);

impl PathBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn move_to(mut self, pos: Vec2) -> Self {
        self.0.move_to(pos.x, pos.y);
        self
    }

    pub fn line_to(mut self, pos: Vec2) -> Self {
        self.0.line_to(pos.x, pos.y);
        self
    }

    pub fn quad_to(mut self, control: Vec2, pos: Vec2) -> Self {
        self.0.quad_to(control.x, control.y, pos.x, pos.y);
        self
    }

    pub fn cubic_to(mut self, control1: Vec2, control2: Vec2, pos: Vec2) -> Self {
        self.0
            .cubic_to(control1.x, control1.y, control2.x, control2.y, pos.x, pos.y);
        self
    }

    pub fn finish(self) -> Path {
        Path(self.0.finish().expect("invalid path"))
    }
}

pub struct Path(tiny_skia::Path);

impl Path {
    pub fn builder() -> PathBuilder {
        PathBuilder::new()
    }

    pub fn circle(center: Vec2, radius: f32) -> Self {
        Self(
            tiny_skia::PathBuilder::from_circle(center.x, center.y, radius)
                .expect("circle with radius 0"),
        )
    }

    pub fn rect(rect: Rect) -> Self {
        Self(tiny_skia::PathBuilder::from_rect(tsk_rect(rect)))
    }
}

pub struct Paint<'a>(tiny_skia::Paint<'a>);

impl<'a> Default for Paint<'a> {
    fn default() -> Self {
        Self(tiny_skia::Paint {
            anti_alias: true,
            ..Default::default()
        })
    }
}

impl<'a> Paint<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn shade_solid(mut self, color: Color) -> Self {
        self.0.shader = tiny_skia::Shader::SolidColor(tsk_color(color));
        self
    }

    pub fn blend_mode(mut self, mode: BlendMode) -> Self {
        self.0.blend_mode = mode;
        self
    }

    pub fn no_anti_alias(mut self) -> Self {
        self.0.anti_alias = false;
        self
    }
}

#[derive(Default)]
pub struct Stroke(tiny_skia::Stroke);

impl Stroke {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn width(mut self, width: f32) -> Self {
        self.0.width = width;
        self
    }

    pub fn line_cap(mut self, cap: LineCap) -> Self {
        self.0.line_cap = cap;
        self
    }

    pub fn line_join(mut self, join: LineJoin) -> Self {
        self.0.line_join = join;
        self
    }

    pub fn miter_limit(mut self, limit: f32) -> Self {
        self.0.miter_limit = limit;
        self
    }
}

pub struct Canvas {
    target: tiny_skia::Canvas,
    scale: f32,
}

impl Canvas {
    pub fn new(pixel_width: u32, pixel_height: u32, scale: f32) -> Self {
        let target = tiny_skia::Canvas::new(pixel_width, pixel_height).expect("dimensions 0");
        let mut canvas = Self { target, scale };
        canvas.apply_scale();
        canvas
    }

    pub fn pixel_width(&self) -> u32 {
        self.target.pixmap.width()
    }

    pub fn pixel_height(&self) -> u32 {
        self.target.pixmap.height()
    }

    pub fn width(&self) -> f32 {
        self.pixel_width() as f32 / self.scale
    }

    pub fn height(&self) -> f32 {
        self.pixel_height() as f32 / self.scale
    }

    pub fn clear(&mut self, color: Color) {
        self.target.pixmap.fill(tsk_color(color));
    }

    pub fn resize(&mut self, new_pixel_width: u32, new_pixel_height: u32, new_scale: f32) {
        self.target =
            tiny_skia::Canvas::new(new_pixel_width, new_pixel_height).expect("dimensions 0");
        self.set_scale(new_scale);
    }

    pub fn fill_path(&mut self, path: &Path, paint: &Paint) -> &mut Self {
        self.target
            .fill_path(&path.0, &paint.0, FillRule::default());
        self
    }

    pub fn stroke_path(&mut self, path: &Path, paint: &Paint, stroke: &Stroke) -> &mut Self {
        self.target.stroke_path(&path.0, &paint.0, &stroke.0);
        self
    }

    pub fn data(&self) -> &[u8] {
        self.target.pixmap.data()
    }

    pub fn save_png(&self, path: &std::path::Path) {
        self.target
            .pixmap
            .save_png(path)
            .expect("failed to save PNG")
    }

    fn set_scale(&mut self, new_scale: f32) {
        self.scale = new_scale;
        self.apply_scale();
    }

    fn apply_scale(&mut self) {
        self.remove_scale();
        self.target.scale(self.scale, self.scale);
    }

    fn remove_scale(&mut self) {
        self.target.reset_transform();
    }
}
