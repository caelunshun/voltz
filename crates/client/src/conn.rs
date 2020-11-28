use protocol::{
    bridge::ToServer,
    packets::server::{LoadChunk, UnloadChunk},
    packets::ServerPacket,
    Bridge,
};

use crate::{
    event::{ChunkLoaded, ChunkUnloaded},
    game::Game,
};

/// Handles packets received from the server.
///
/// _Sending_ packets is performed by the various systems
/// running each tick. This on
pub struct Connection {
    bridge: Bridge<ToServer>,
}

impl Connection {
    pub fn new(bridge: Bridge<ToServer>) -> Self {
        Self { bridge }
    }

    /// Handles all buffered packets and updates the game state accordingly.
    pub fn handle_packets(&mut self, game: &mut Game) {
        for packet in self.bridge.flush_received() {
            match packet {
                ServerPacket::Shared(_) => {}
                ServerPacket::ServerInfo(_) | ServerPacket::JoinGame(_) => {
                    log::warn!("Received login packet during game state?");
                }
                ServerPacket::LoadChunk(packet) => handle_load_chunk(game, packet),
                ServerPacket::UnloadChunk(packet) => handle_unload_chunk(game, packet),
            }
        }
    }
}

fn handle_load_chunk(game: &mut Game, packet: LoadChunk) {
    game.main_zone_mut().insert(packet.pos, packet.chunk);
    game.events().push(ChunkLoaded { pos: packet.pos });
    log::trace!("Received and loaded chunk {:?}", packet.pos);
}

fn handle_unload_chunk(game: &mut Game, packet: UnloadChunk) {
    let existed = game.main_zone_mut().remove(packet.pos).is_some();
    game.events().push(ChunkUnloaded { pos: packet.pos });
    log::trace!("Unloaded chunk {:?} (existed: {})", packet.pos, existed);
}
