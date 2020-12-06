use std::sync::Arc;

use fontdue::Font;
use voltzui::{
    widgets::{Container, Text},
    Canvas, Dimension, Ui,
};

fn main() {
    let mut ui = Ui::new();
    let font = Arc::new(
        Font::from_bytes(
            &include_bytes!("../../../assets/font/Play-Regular.ttf")[..],
            Default::default(),
        )
        .unwrap(),
    );
    ui.build()
        .begin(Container::column().with_style(|s| {
            s.size.width = Dimension::Percent(100.);
        }))
        .push(Text::new("Voltz v0.1.0 - Protocol 90", &font))
        .push(Text::new("400 FPS", &font))
        .push(Text::new("GPU: NVIDIA GeForce GTX 1060", &font))
        .push(Text::new("Backend: Vulkan", &font))
        .push(Text::new("Chunks: 190", &font))
        .push(Text::new("World memory: 191MiB", &font))
        .end();

    let mut cv = Canvas::new(1024, 1024, 1.);
    ui.render(&mut cv);
    cv.save_png("ui.png");
}
