use std::{
    panic, thread,
    time::{Duration, Instant},
};

use common::{blocks, chunk::CHUNK_DIM, world::ZoneBuilder, BlockId, Chunk, ChunkPos, Zone};
pub use conn::Connection;
use game::Game;
use panic::AssertUnwindSafe;

mod conn;
mod game;
mod mailbox;

/// The number of ticks executed per second.
pub const TPS: u32 = 20;
/// The number of milliseconds in between each tick.
pub const TICK_LENGTH: u32 = 1000 / TPS;

/// The top-level server state.
pub struct Server {
    clients: Vec<Connection>,
    game: Game,
}

impl Server {
    /// Creates a new `Server` with the given set of initial clients.
    ///
    /// This is an expensive operation: we have to generate the world.
    pub fn new(clients: Vec<Connection>) -> Self {
        log::info!("Generating world...");
        let start = Instant::now();
        let main_zone = generate_world();
        log::info!("World generated in {:?}", start.elapsed());

        let game = Game::new(main_zone);

        Self { clients, game }
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
        self.poll_connections();
    }

    fn poll_connections(&mut self) {
        for conn in &mut self.clients {
            conn.tick(&mut self.game);
        }
    }
}

fn generate_world() -> Zone {
    const WORLD_SIZE: i32 = 4;
    let mut builder = ZoneBuilder::new(
        ChunkPos {
            x: -WORLD_SIZE,
            y: 0,
            z: -WORLD_SIZE,
        },
        ChunkPos {
            x: WORLD_SIZE,
            y: 15,
            z: WORLD_SIZE,
        },
    );

    for x in -WORLD_SIZE..=WORLD_SIZE {
        for y in 0..16 {
            for z in -WORLD_SIZE..=WORLD_SIZE {
                let mut chunk = Chunk::new();

                if y <= 4 {
                    for bx in 0..CHUNK_DIM {
                        for by in 0..CHUNK_DIM {
                            for bz in 0..CHUNK_DIM {
                                chunk.set(bx, by, bz, BlockId::new(blocks::Stone));
                            }
                        }
                    }
                }
                builder.add_chunk(ChunkPos { x, y, z }, chunk).unwrap();
            }
        }
    }

    builder.build().ok().expect("failed to generate all chunks")
}
