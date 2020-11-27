use common::{Chunk, ChunkPos};
use protocol::{
    bridge::ToClient,
    packets::server::{LoadChunk, UnloadChunk},
    packets::ServerPacket,
    Bridge,
};

/// A player's "mailbox," encapsulating the sending
/// of packets to notify a client of updated state.
pub struct Mailbox {
    bridge: Bridge<ToClient>,
}

impl Mailbox {
    pub fn from_bridge(bridge: Bridge<ToClient>) -> Self {
        Self { bridge }
    }

    pub fn send_chunk(&self, pos: ChunkPos, chunk: &Chunk) {
        let packet = ServerPacket::LoadChunk(LoadChunk {
            pos,
            chunk: chunk.clone(), // consider how we might remove this clone?
        });
        self.bridge.send(packet);
    }

    pub fn unload_chunk(&self, pos: ChunkPos) {
        let packet = ServerPacket::UnloadChunk(UnloadChunk { pos });
        self.bridge.send(packet);
    }
}
