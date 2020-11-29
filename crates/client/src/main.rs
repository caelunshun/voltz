#![feature(type_name_of_val, allocator_api)]
#![allow(dead_code)]

use std::thread;

use anyhow::{anyhow, bail, Context};
use asset::{model::YamlModel, shader::SpirvLoader, texture::PngLoader, Assets, YamlLoader};
use bumpalo::Bump;
use camera::CameraController;
use common::{entity::player::PlayerBundle, Orient, Pos, SystemExecutor};
use conn::Connection;
use game::Game;
use protocol::{
    bridge::{self, ToServer},
    packets::client::ClientInfo,
    packets::ClientPacket,
    packets::ServerPacket,
    Bridge, PROTOCOL_VERSION,
};
use renderer::Renderer;
use sdl2::{
    event::Event, event::WindowEvent, keyboard::KeyboardState, video::Window, EventPump, Sdl,
};
use server::Server;
use simple_logger::SimpleLogger;

mod asset;
mod camera;
mod conn;
mod event;
mod game;
mod renderer;
mod update_server;

pub struct Client {
    assets: Assets,
    renderer: Renderer,
    camera: CameraController,

    // SDL2 state
    window: Window,
    event_pump: EventPump,
    sdl: Sdl,

    open: bool,

    game: Game,
    conn: Connection,

    systems: SystemExecutor<Game>,
}

impl Client {
    pub fn run(mut self) -> anyhow::Result<()> {
        while self.open {
            self.sdl.mouse().set_relative_mouse_mode(true);
            self.handle_events();
            self.tick();
        }
        Ok(())
    }

    fn handle_events(&mut self) {
        self.camera
            .tick_keyboard(&mut self.game, KeyboardState::new(&self.event_pump));
        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => self.open = false,
                Event::Window { win_event, .. } => match win_event {
                    WindowEvent::Resized(_, _) => self
                        .renderer
                        .on_resize(self.window.size().0, self.window.size().1),
                    _ => (),
                },
                Event::MouseMotion { xrel, yrel, .. } => {
                    self.camera.on_mouse_move(&mut self.game, xrel, yrel)
                }
                _ => (),
            }
        }
    }

    fn tick(&mut self) {
        self.game.events().set_system(0);
        self.conn.handle_packets(&mut self.game);

        self.systems.run(&mut self.game, |game, system| {
            game.events().set_system(system + 1)
        });

        self.game.events().set_system(self.systems.len() + 1);
        self.render();
    }

    fn render(&mut self) {
        let (width, height) = self.window.size();
        let aspect_ratio = width as f32 / height as f32;
        let matrices = self.camera.matrices(&mut self.game, aspect_ratio);
        self.renderer.render(&mut self.game, matrices);
    }
}

fn main() -> anyhow::Result<()> {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Debug)
        .init()?;
    let assets = load_assets()?;
    let (sdl, window, event_pump) =
        init_sdl2().map_err(|e| anyhow!("failed to initialize SDL2: {}", e))?;
    let renderer = Renderer::new(&window, &assets).context("failed to intiailize wgpu renderer")?;
    let camera = CameraController;

    let bridge = launch_server()?;
    let (pos, orient) = log_in(&bridge).context("failed to connect to integrated server")?;
    let conn = Connection::new(bridge.clone());
    let game = Game::new(bridge, PlayerBundle { pos, orient }, Bump::new());
    let systems = setup();

    let client = Client {
        assets,
        renderer,
        camera,

        window,
        event_pump,
        sdl,

        open: true,

        game,
        conn,

        systems,
    };
    client.run()
}

fn load_assets() -> anyhow::Result<Assets> {
    let mut assets = Assets::new();
    assets
        .add_loader("YamlModel", YamlLoader::<YamlModel>::new())
        .add_loader("Png", PngLoader::new())
        .add_loader("Spirv", SpirvLoader::new());
    assets.load_dir("assets").context("failed to load assets")?;
    Ok(assets)
}

fn init_sdl2() -> Result<(Sdl, Window, EventPump), String> {
    let sdl2 = sdl2::init()?;
    let video = sdl2.video()?;

    let title = "Voltz";
    let width = 1920 / 2;
    let height = 1080 / 2;

    let window = video
        .window(title, width, height)
        .allow_highdpi()
        .resizable()
        .build()
        .map_err(|e| e.to_string())?;
    let event_pump = sdl2.event_pump()?;

    Ok((sdl2, window, event_pump))
}

fn launch_server() -> anyhow::Result<Bridge<ToServer>> {
    let (client_bridge, server_bridge) = bridge::singleplayer();

    let conn = server::Connection::new(server_bridge);

    thread::Builder::new()
        .name("integrated-server".to_owned())
        .spawn(move || {
            let mut server = Server::new(vec![conn]);
            server.run();
        })?;

    Ok(client_bridge)
}

fn log_in(bridge: &Bridge<ToServer>) -> anyhow::Result<(Pos, Orient)> {
    log::info!("Connecting to server");
    bridge.send(ClientPacket::ClientInfo(ClientInfo {
        protocol_version: PROTOCOL_VERSION,
        implementation: format!("voltz-client:{}", env!("CARGO_PKG_VERSION")),
        username: "caelunshun".to_owned(),
    }));

    let server_info = match bridge.wait_received() {
        Some(ServerPacket::ServerInfo(info)) => info,
        Some(_) => bail!("invalid packet received during login state"),
        None => bail!("disconnected"),
    };

    log::info!(
        "Connected to server '{}' implementing protocol {}.",
        server_info.implementation,
        server_info.protocol_version
    );

    let join_game = match bridge.wait_received() {
        Some(ServerPacket::JoinGame(join_game)) => join_game,
        Some(_) => bail!("invalid packet received during login state"),
        None => bail!("disconnected"),
    };

    log::info!("Received JoinGame: {:?}", join_game);
    Ok((Pos(join_game.pos), Orient(join_game.orient)))
}

fn setup() -> SystemExecutor<Game> {
    let mut systems = SystemExecutor::new();

    update_server::setup(&mut systems);

    systems
}
