//! Packets sent by both the server and the client.

use serde::{Deserialize, Serialize};

/// The union of all possible shared packets.
#[derive(Debug, Serialize, Deserialize)]
pub enum SharedPacket {
    Disconnect(Disconnect),
}

/// Informs the peer that the connection is terminated.
#[derive(Debug, Serialize, Deserialize)]
pub struct Disconnect {
    /// An optional reason for the disconnect.
    pub reason: Option<String>,
}
