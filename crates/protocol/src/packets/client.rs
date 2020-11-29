//! Packets sent by the client.

use glam::{Vec2, Vec3A};
use serde::{Deserialize, Serialize};

use super::shared::SharedPacket;

/// The union of all possible packets sent by the client.
#[derive(Debug, Serialize, Deserialize)]
pub enum ClientPacket {
    Shared(SharedPacket),
    ClientInfo(ClientInfo),
    UpdatePosition(UpdatePosition),
}

/// Login state: initial data sent by the client.
#[derive(Debug, Serialize, Deserialize)]
pub struct ClientInfo {
    /// The protocol version implemented by the client.
    pub protocol_version: u32,
    /// An arbitrary name for the client implementation.
    pub implementation: String,

    /// The player's username.
    pub username: String,
}

/// Updates the client's position on the server.
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdatePosition {
    /// The new position.
    pub new_pos: Vec3A,
    /// The new orientation.
    pub new_orient: Vec2,
}
