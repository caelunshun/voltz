use glam::vec2;
use utils::Color;
use voltzui::{
    widgets::{Container, Rectangle},
    Ui,
};

fn main() {
    let mut ui = Ui::new(1024, 1024, 1.);
    ui.build()
        .push(Rectangle::new(vec2(100., 100.), Color::rgb(0.5, 0.6, 0.8)))
        .push(Rectangle::new(vec2(150., 50.), Color::rgb(0.9, 0.7, 0.4)))
        .begin(Container::column().with_style(|s| {
            s.justify_content = voltzui::JustifyContent::Center;
        }))
        .push(Rectangle::new(vec2(500., 500.), Color::rgb(0.8, 0.4, 0.3)))
        .push(Rectangle::new(vec2(50., 300.), Color::rgb(0.3, 0.4, 0.8)))
        .end();

    ui.render();
    ui.save_png("ui.png");
}
