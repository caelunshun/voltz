//! Packets sent by the server.

use common::{Chunk, ChunkPos};
use derivative::Derivative;
use glam::{Vec2, Vec3A};
use serde::{Deserialize, Serialize};

use super::shared::SharedPacket;

/// The union of all possible packets sent by the server.
#[derive(Debug, Serialize, Deserialize)]
pub enum ServerPacket {
    Shared(SharedPacket),

    ServerInfo(ServerInfo),
    JoinGame(JoinGame),

    LoadChunk(LoadChunk),
    UnloadChunk(UnloadChunk),
}

/// Login phase: the server's properties.
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerInfo {
    /// The protocol version implemented by the server.
    pub protocol_version: u32,
    /// An arbitrary name for the server.
    pub implementation: String,
}

/// Login phase: the player's initial state. Switches
/// state to Game.
#[derive(Debug, Serialize, Deserialize)]
pub struct JoinGame {
    /// The player's initial position.
    pub pos: Vec3A,
    /// The player's initial orientation.
    pub orient: Vec2,
}

/// Loads a chunk on the client.
///
/// Replaces the chunk if it was already loaded. This behavior
/// can be utilized to optimize bulk block updates by replacing
/// entire chunks.
#[derive(Derivative, Serialize, Deserialize)]
#[derivative(Debug)]
pub struct LoadChunk {
    /// The position of the chunk.
    pub pos: ChunkPos,
    /// The chunk.
    #[derivative(Debug = "ignore")]
    pub chunk: Chunk,
}

/// Unloads a chunk on the client.
///
/// Does nothing when the chunk is not already loaded.
#[derive(Debug, Serialize, Deserialize)]
pub struct UnloadChunk {
    /// The position of the chunk to unload.
    pub pos: ChunkPos,
}
