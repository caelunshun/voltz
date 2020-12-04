use glam::vec2;
use utils::Color;
use voltzui::{widgets::Rectangle, Ui};

fn main() {
    let mut ui = Ui::new(1024, 1024, 1.);
    ui.build()
        .push(Rectangle::new(vec2(100., 100.), Color::rgb(0.5, 0.6, 0.8)))
        .push(Rectangle::new(vec2(150., 50.), Color::rgb(0.9, 0.7, 0.4)));

    ui.render();
    ui.save_png("ui.png");
}
