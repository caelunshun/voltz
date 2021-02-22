#![feature(allocator_api)]

use std::{
    panic,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use common::{world::ZoneBuilder, ChunkPos, SystemExecutor, Zone};
pub use conn::Connection;
use game::Game;
use panic::AssertUnwindSafe;
use protocol::{bridge::ToClient, Bridge};
use worldgen::WorldGenerator;

mod conn;
mod event;
mod game;
mod view;

pub type Mailbox = Bridge<ToClient>;

/// The number of ticks executed per second.
pub const TPS: u32 = 20;
/// The number of milliseconds in between each tick.
pub const TICK_LENGTH: u32 = 1000 / TPS;

/// The number of chunks visible from a player's current
/// position. Fixed for now.
pub const VIEW_DISTANCE: u32 = 8;
pub const WORLD_SIZE: i32 = 16;

/// The top-level server state.
pub struct Server {
    clients: Vec<Connection>,
    game: Game,
    systems: SystemExecutor<Game>,

    world_generator: Arc<WorldGenerator>,
}

impl Server {
    /// Creates a new `Server` with the given set of initial clients.
    ///
    /// This is an expensive operation: we have to generate the world.
    pub fn new(clients: Vec<Connection>) -> anyhow::Result<Self> {
        let (device, queue, _) =
            common::gpu::init(wgpu::Instance::new(wgpu::BackendBit::PRIMARY), None)?;
        let device = Arc::new(device);
        common::gpu::launch_poll_thread(&device);
        let world_generator = Arc::new(WorldGenerator::new(&device, &Arc::new(queue)));
        log::info!("Generating world...");
        let start = Instant::now();
        let main_zone = generate_world(&world_generator);
        log::info!("World generated in {:?}", start.elapsed());

        let game = Game::new(main_zone);
        let systems = setup();

        Ok(Self {
            clients,
            game,
            systems,
            world_generator,
        })
    }

    /// Runs the server.
    pub fn run(&mut self) {
        loop {
            let start = Instant::now();

            if let Err(e) = panic::catch_unwind(AssertUnwindSafe(|| {
                self.tick();
            })) {
                log::error!("The server panicked while ticking: {:?}", e);
                log::error!("This is a bug. Please report it.");
                log::error!("We will try to recover, but the game state may have become corrupted. We advise that you restart the server.");
            }

            let elapsed = start.elapsed().as_millis() as u32;
            if elapsed > TICK_LENGTH {
                log::warn!("Tick took too long! ({}ms)", elapsed);
                continue;
            } else {
                let remaining = TICK_LENGTH - elapsed;
                thread::sleep(Duration::from_millis(remaining as u64));
            }
        }
    }

    fn tick(&mut self) {
        self.game.events().set_system(0);
        self.poll_connections();

        self.systems.run(&mut self.game, |game, system| {
            game.events().set_system(system + 1);
        });

        self.game.bump_mut().reset();
    }

    fn poll_connections(&mut self) {
        for conn in &mut self.clients {
            conn.tick(&mut self.game);
        }
    }
}

fn generate_world(world_generator: &WorldGenerator) -> Zone {
    let mut builder = ZoneBuilder::new(
        ChunkPos { x: 0, y: 0, z: 0 },
        ChunkPos {
            x: WORLD_SIZE - 1,
            y: 15,
            z: WORLD_SIZE - 1,
        },
    );
    world_generator.generate_into_zone(&mut builder, 6256);
    builder.build().ok().expect("failed to create all chunks")
}

fn setup() -> SystemExecutor<Game> {
    let mut systems = SystemExecutor::new();

    view::setup(&mut systems);

    systems
}
