//! Packets sent by the client.

use serde::{Deserialize, Serialize};

use super::shared::SharedPacket;

/// The union of all possible packets sent by the client.
#[derive(Debug, Serialize, Deserialize)]
pub enum ClientPacket {
    Shared(SharedPacket),
    ClientInfo(ClientInfo),
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
