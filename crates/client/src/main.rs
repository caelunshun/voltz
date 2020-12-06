#![feature(type_name_of_val, allocator_api, format_args_capture)]
#![allow(dead_code)]

use std::{alloc::System, thread, time::Instant};

use anyhow::{anyhow, bail, Context};
use asset::{
    font::FontLoader, model::YamlModel, shader::SpirvLoader, texture::PngLoader, Assets, YamlLoader,
};
use bumpalo::Bump;
use common::{entity::Vel, Orient, Pos, SystemExecutor};
use conn::Connection;
use game::Game;
use glam::Vec3A;
use physics::Aabb;
use protocol::{
    bridge::{self, ToServer},
    packets::client::ClientInfo,
    packets::ClientPacket,
    packets::ServerPacket,
    Bridge, PROTOCOL_VERSION,
};
use renderer::Renderer;
use sdl2::{video::Window, EventPump, Sdl};
use server::Server;
use simple_logger::SimpleLogger;
use utils::TrackAllocator;

const PLAYER_BBOX: Aabb = Aabb {
    min: Vec3A::zero(),
    max: glam::const_vec3a!([0.5, 2., 0.5]),
};

mod asset;
mod camera;
mod conn;
mod debug;
mod entity;
mod event;
mod game;
mod input;
mod renderer;
mod ui;
mod update_server;

#[global_allocator]
pub static ALLOCATOR: TrackAllocator<System> = TrackAllocator::new(System);

pub struct Client {
    assets: Assets,

    systems: SystemExecutor<Game>,

    sdl: Sdl,
    game: Game,

    conn: Connection,
}

impl Client {
    pub fn run(mut self) -> anyhow::Result<()> {
        while !self.game.should_close() {
            let start = Instant::now();
            self.sdl.mouse().set_relative_mouse_mode(true);
            self.tick();
            let elapsed = start.elapsed();
            self.game.set_dt(elapsed.as_secs_f32());
        }
        Ok(())
    }

    fn tick(&mut self) {
        self.game.events().set_system(0);
        self.conn.handle_packets(&mut self.game);

        self.systems.run(&mut self.game, |game, system| {
            game.events().set_system(system + 1)
        });
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

    let bridge = launch_server()?;
    let (pos, orient, vel) = log_in(&bridge).context("failed to connect to integrated server")?;
    let conn = Connection::new(bridge.clone());
    let mut game = Game::new(
        bridge,
        (pos, orient, vel, PLAYER_BBOX),
        window,
        event_pump,
        Bump::new(),
    );

    let mut systems = setup(&assets)?;
    renderer.setup(&mut systems, &mut game);

    let client = Client {
        assets,

        sdl,

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
        .add_loader("Spirv", SpirvLoader::new())
        .add_loader("Font", FontLoader::new());
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

fn log_in(bridge: &Bridge<ToServer>) -> anyhow::Result<(Pos, Orient, Vel)> {
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
    Ok((
        Pos(join_game.pos),
        Orient(join_game.orient),
        Vel(join_game.vel),
    ))
}

fn setup(assets: &Assets) -> anyhow::Result<SystemExecutor<Game>> {
    let mut systems = SystemExecutor::new();

    input::setup(&mut systems);
    camera::setup(&mut systems);
    entity::setup(&mut systems);
    debug::setup(&mut systems, assets)?;
    update_server::setup(&mut systems);

    Ok(systems)
}
