//! The debug screen (F3)

use common::{event::EventBus, Orient, Pos, System, SystemExecutor};
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

#[derive(Default)]
pub struct DebugData {
    pub adapter: Option<wgpu::AdapterInfo>,
    pub render_chunks: usize,
}

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
        let orient = game.player_ref().get::<Orient>().unwrap().0;
        let [orientx, orienty] = [orient.x, orient.y];

        let memory = utils::format_bytes(ALLOCATOR.allocated() as u64);

        let (adapter, backend) = game
            .debug_data
            .adapter
            .as_ref()
            .map(|info| {
                let backend = match info.backend {
                    wgpu::Backend::Empty => "Empty",
                    wgpu::Backend::Vulkan => "Vulkan",
                    wgpu::Backend::Metal => "Metal",
                    wgpu::Backend::Dx12 => "DirectX 12",
                    wgpu::Backend::Dx11 => "DirectX 11",
                    wgpu::Backend::Gl => "OpenGL",
                    wgpu::Backend::BrowserWebGpu => "WebGPU",
                };
                (info.name.as_str(), backend)
            })
            .unwrap_or_else(|| ("unknown", "Unknown"));

        let dt = game.dt() * 1000.;

        let loaded_chunks = game.main_zone().len();
        let render_chunks = game.debug_data.render_chunks;

        indoc::formatdoc! {"
            Voltz v{version}, protocol {protocol}
            X: {posx:.2}, Y: {posy:.2}, Z: {posz:.2}
            Yaw: {orientx:.2}, Pitch: {orienty:.2}

            Adapter: {adapter}
            Backend: {backend}

            Chunks loaded: {loaded_chunks}
            Chunks rendering: {render_chunks}
            Used memory: {memory}

            Frame time: {dt:.2}ms
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
                .push(Text::new(&text, self.font.as_arc()).size(30.));
        }
    }
}
