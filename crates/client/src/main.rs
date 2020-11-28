#![feature(type_name_of_val, allocator_api)]
#![allow(dead_code)]

use std::thread;

use anyhow::{anyhow, bail, Context};
use asset::{model::YamlModel, shader::SpirvLoader, texture::PngLoader, Assets, YamlLoader};
use bumpalo::Bump;
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
use sdl2::{event::Event, video::Window, EventPump};
use server::Server;
use simple_logger::SimpleLogger;

mod asset;
mod conn;
mod event;
mod game;
mod renderer;

pub struct Client {
    assets: Assets,
    renderer: Renderer,

    // SDL2 state
    window: Window,
    event_pump: EventPump,

    open: bool,

    game: Game,
    conn: Connection,

    systems: SystemExecutor<Game>,
}

impl Client {
    pub fn run(mut self) -> anyhow::Result<()> {
        while self.open {
            self.handle_events();
            self.tick();
        }
        Ok(())
    }

    fn handle_events(&mut self) {
        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => self.open = false,
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
        self.renderer.render(&mut self.game);
    }
}

fn main() -> anyhow::Result<()> {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Debug)
        .init()?;
    let assets = load_assets()?;
    let (window, event_pump) =
        init_sdl2().map_err(|e| anyhow!("failed to initialize SDL2: {}", e))?;
    let renderer = Renderer::new(&window, &assets).context("failed to intiailize wgpu renderer")?;

    let bridge = launch_server()?;
    let pos = log_in(&bridge).context("failed to connect to integrated server")?;
    let conn = Connection::new(bridge.clone());
    let game = Game::new(
        bridge,
        PlayerBundle {
            pos,
            orient: Orient::default(),
        },
        Bump::new(),
    );
    let systems = setup();

    let client = Client {
        assets,
        renderer,

        window,
        event_pump,

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

fn init_sdl2() -> Result<(Window, EventPump), String> {
    let sdl2 = sdl2::init()?;
    let video = sdl2.video()?;

    let title = "Voltz";
    let width = 1920 / 2;
    let height = 1080 / 2;

    let window = video
        .window(title, width, height)
        .allow_highdpi()
        .build()
        .map_err(|e| e.to_string())?;
    let event_pump = sdl2.event_pump()?;

    Ok((window, event_pump))
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

fn log_in(bridge: &Bridge<ToServer>) -> anyhow::Result<Pos> {
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
    Ok(Pos(join_game.pos))
}

fn setup() -> SystemExecutor<Game> {
    let systems = SystemExecutor::new();

    systems
}
