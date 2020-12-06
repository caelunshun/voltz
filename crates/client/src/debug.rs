//! The debug screen (F3)

use common::{event::EventBus, Pos, System, SystemExecutor};
use fontdue::Font;
use glam::Vec2;
use protocol::PROTOCOL_VERSION;
use sdl2::keyboard::Keycode;
use voltzui::widgets::Text;

use crate::{
    asset::{Asset, Assets},
    event::KeyPressed,
    game::Game,
    ui::Length,
    ALLOCATOR,
};

pub fn setup(systems: &mut SystemExecutor<Game>, assets: &Assets) -> anyhow::Result<()> {
    let font = assets.get("font/Play-Regular.ttf")?;
    systems.add(DebugSystem {
        enabled: false,
        font,
    });
    Ok(())
}

struct DebugSystem {
    enabled: bool,
    font: Asset<Font>,
}

impl DebugSystem {
    fn update_enabled(&mut self, events: &mut EventBus) {
        for key_pressed in events.iter::<KeyPressed>() {
            if key_pressed.key == Keycode::F3 {
                self.enabled = !self.enabled;
            }
        }
    }

    fn text(&self, game: &Game) -> String {
        let version = env!("CARGO_PKG_VERSION");
        let protocol = PROTOCOL_VERSION;

        let pos = game.player_ref().get::<Pos>().unwrap().0;
        let [posx, posy, posz] = [pos.x, pos.y, pos.z];

        let memory = utils::format_bytes(ALLOCATOR.allocated() as u64);

        indoc::formatdoc! {"
            Voltz v{version} - protocol {protocol}
            Position: {posx:.2}, {posy:.2}, {posz:.2}
            Used memory: {memory}
        "}
    }
}

impl System<Game> for DebugSystem {
    fn run(&mut self, game: &mut Game) {
        self.update_enabled(&mut *game.events());

        if self.enabled {
            let mut ui_store = game.ui_store();
            let ui = ui_store.get(
                "debug",
                Length::Percent(100.),
                Length::Percent(100.),
                Vec2::zero(),
            );
            let text = self.text(game);
            ui.build()
                .push(Text::new(&text, self.font.as_arc()).size(20.));
        }
    }
}
