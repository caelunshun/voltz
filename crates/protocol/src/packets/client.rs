//! Packets sent by the client.

use serde::{Deserialize, Serialize};

use super::shared::SharedPacket;

/// The union of all possible packets sent by the client.
#[derive(Debug, Serialize, Deserialize)]
pub enum ClientPacket {
    Shared(SharedPacket),
}
