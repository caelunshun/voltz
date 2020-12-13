#![feature(type_name_of_val, allocator_api, format_args_capture)]
#![allow(dead_code)]

use std::{alloc::System, sync::Arc, thread, time::Instant};

use anyhow::{bail, Context};
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
use server::Server;
use simple_logger::SimpleLogger;
use utils::TrackAllocator;
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

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

    game: Game,

    conn: Connection,
}

impl Client {
    pub fn run(mut self, event_loop: EventLoop<()>) -> anyhow::Result<()> {
        let mut previous = Instant::now();
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;
            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    *control_flow = ControlFlow::Exit;
                }
                Event::MainEventsCleared => {
                    self.tick();
                    let elapsed = previous.elapsed();
                    self.game.set_dt(elapsed.as_secs_f32());

                    if elapsed.as_secs_f64() >= (1. / 60.) {
                        log::warn!("Frame took too long: {:?}", elapsed);
                    }

                    previous = Instant::now();

                    self.game.window_mut().set_cursor_visible(false);
                    if let Err(e) = self.game.window_mut().set_cursor_grab(true) {
                        log::error!("Failed to grab cursor: {:?}", e);
                    }
                }
                Event::WindowEvent { event, .. } => input::handle_event(&event, &mut self.game),
                _ => (),
            }
        });
    }

    fn tick(&mut self) {
        self.game.events().set_system(0);
        self.conn.handle_packets(&mut self.game);

        self.systems.run(&mut self.game, |game, system| {
            game.events().set_system(system + 1)
        });

        self.game.bump_mut().reset();
    }
}

fn main() -> anyhow::Result<()> {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Debug)
        .init()?;
    let assets = load_assets()?;
    let (window, event_loop) = init_window()?;
    let renderer = Renderer::new(&window, &assets).context("failed to intiailize wgpu renderer")?;

    let bridge = launch_server(&renderer)?;
    let (pos, orient, vel) = log_in(&bridge).context("failed to connect to integrated server")?;
    let conn = Connection::new(bridge.clone());
    let mut game = Game::new(bridge, (pos, orient, vel, PLAYER_BBOX), window, Bump::new());

    let mut systems = setup(&assets)?;
    renderer.setup(&mut systems, &mut game);

    let client = Client {
        assets,

        game,
        conn,

        systems,
    };
    client.run(event_loop)
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

fn init_window() -> anyhow::Result<(Window, EventLoop<()>)> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Voltz")
        .with_inner_size(LogicalSize::new(1920 / 2, 1080 / 2))
        .with_resizable(true)
        .build(&event_loop)
        .context("failed to create window")?;

    log::info!("Window scale factor: {}", window.scale_factor());

    Ok((window, event_loop))
}

fn launch_server(renderer: &Renderer) -> anyhow::Result<Bridge<ToServer>> {
    let (client_bridge, server_bridge) = bridge::singleplayer();

    let conn = server::Connection::new(server_bridge);

    let device = Arc::clone(renderer.device_arc());
    let queue = Arc::clone(renderer.queue_arc());

    thread::Builder::new()
        .name("integrated-server".to_owned())
        .spawn(move || {
            let mut server = Server::new(vec![conn], &device, &queue);
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

    camera::setup(&mut systems);
    entity::setup(&mut systems);
    debug::setup(&mut systems, assets)?;
    update_server::setup(&mut systems);

    Ok(systems)
}
