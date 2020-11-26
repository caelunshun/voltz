//! All packets. Packets are defined
//! in the submodule corresponding to the
//! peer which sends them.

pub mod client;
pub mod server;
pub mod shared;

#[doc(inline)]
pub use self::{client::ClientPacket, server::ServerPacket, shared::SharedPacket};
