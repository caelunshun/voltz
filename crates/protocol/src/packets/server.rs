//! Packets sent by the server.

use serde::{Deserialize, Serialize};

use super::shared::SharedPacket;

/// The union of all possible packets sent by the server.
#[derive(Debug, Serialize, Deserialize)]
pub enum ServerPacket {
    Shared(SharedPacket),
}
