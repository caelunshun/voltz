use std::{
    fmt::{self, Debug, Formatter},
    ops::Deref,
    sync::Arc,
};

use ahash::AHashMap;
use fontdue::{
    layout::{GlyphRasterConfig, Layout, LayoutSettings, TextStyle, WrapStyle},
    Font,
};
use glam::Vec2;
use tiny_skia::{ColorU8, Pixmap, PixmapPaint};
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

pub use fontdue::layout::{HorizontalAlign, VerticalAlign};

pub struct TextSettings {
    pub font: Arc<Font>,
    pub align_h: HorizontalAlign,
    pub align_v: VerticalAlign,
    pub size: f32,
    pub pos: Vec2,
    pub max_width: Option<f32>,
    pub max_height: Option<f32>,
}

impl Debug for TextSettings {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextSettings")
            .field("size", &self.size)
            .field("pos", &self.pos)
            .field("max_width", &self.max_width)
            .field("max_height", &self.max_height)
            .finish()
    }
}

impl TextSettings {
    pub fn layout(&self, text: &str, layout_engine: &mut Layout) {
        layout_engine.reset(&LayoutSettings {
            x: self.pos.x,
            y: self.pos.y,
            max_width: self.max_width,
            max_height: self.max_height,
            horizontal_align: self.align_h,
            vertical_align: self.align_v,
            wrap_style: WrapStyle::Word,
            wrap_hard_breaks: true,
        });
        layout_engine.append(
            &[&*self.font],
            &TextStyle {
                text,
                px: self.size,
                font_index: 0,
                user_data: (),
            },
        );
    }
}

pub struct Canvas {
    target: tiny_skia::Canvas,
    scale: f32,
    glyph_caches: AHashMap<*const Font, FontGlyphCache>,
    layout_engine: Layout,
}

impl Canvas {
    pub fn new(pixel_width: u32, pixel_height: u32, scale: f32) -> Self {
        let target = tiny_skia::Canvas::new(pixel_width, pixel_height).expect("dimensions 0");
        let mut canvas = Self {
            target,
            scale,
            glyph_caches: AHashMap::new(),
            layout_engine: Layout::new(fontdue::layout::CoordinateSystem::PositiveYDown),
        };
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

    pub fn fill_text(&mut self, text: &str, settings: &TextSettings) {
        settings.layout(text, &mut self.layout_engine);

        let glyph_cache = self
            .glyph_caches
            .entry(settings.font.deref() as *const Font)
            .or_default();
        for glyph in self.layout_engine.glyphs() {
            let pixmap = glyph_cache.glyph(&settings.font, glyph.key);
            if let Some(pixmap) = pixmap {
                self.target.draw_pixmap(
                    glyph.x as i32,
                    glyph.y as i32,
                    pixmap,
                    &PixmapPaint {
                        quality: FilterQuality::Bilinear,
                        ..Default::default()
                    },
                );
            }
        }
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

#[derive(Default)]
struct FontGlyphCache {
    glyphs: AHashMap<GlyphRasterConfig, Option<Pixmap>>,
}

impl FontGlyphCache {
    pub fn glyph(&mut self, font: &Font, key: GlyphRasterConfig) -> Option<&Pixmap> {
        self.glyphs
            .entry(key)
            .or_insert_with(|| {
                let (metrics, bitmap) = font.rasterize_config(key);
                if metrics.width == 0 || metrics.height == 0 {
                    None
                } else {
                    Some(coverage_to_pixmap(
                        &bitmap,
                        metrics.width as u32,
                        metrics.height as u32,
                    ))
                }
            })
            .as_ref()
    }
}

fn coverage_to_pixmap(coverage: &[u8], width: u32, height: u32) -> Pixmap {
    let mut pixmap = Pixmap::new(width, height).expect("pixmap of size 0");
    pixmap
        .pixels_mut()
        .iter_mut()
        .zip(coverage.iter().copied())
        .for_each(|(pixel, coverage)| {
            *pixel = ColorU8::from_rgba(u8::MAX, u8::MAX, u8::MAX, coverage).premultiply();
        });
    pixmap
}
