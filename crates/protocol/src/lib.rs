//! The client-server protocol. Defines all packets
//! as well as the [`Bridge`] type.
//!
//! Packets are serialized using `serde` and `bincode`.
//!
//! # Protocol overview
//! The protocol consists of several _states_ representing the
//! status of the current connection.
//! * The [`Login`](State::Login) state is the initial state.
//! The client-server handshake takes place during this time.
//! The client has not yet joined the game.
//! * The [`Game`](State::Game) state is initiated after the client
//! receives [`JoinGame`](packets::server::JoinGame). The connection remains
//! in this state until termination.
//!
//! In multiplayer mode, packets are serialized to bytes and transferred using
//! QUIC. In the `Login` state, packets are sent over the same QUIC stream. Afterward,
//! in the `Game` state, each packet is sent using its own QUIC stream. As a result, there is
//! no order defined between packets, though packets are reliable. In other words, in the
//! `Game` state all packets are sent _reliable_ but _unordered_.
//!
//! When QUIC is enabled, the server must provide a valid certificate granted by
//! a CA with a root in the `webpki-roots` crate.
//!
//! The initial stream of events looks like this:
//! * Client connects to server.
//! * Client sends [`ClientInfo`](packets::client::ClientInfo).
//! * Server sends [`ServerInfo`](packets::server::ServerInfo).
//! * Server sends [`JoinGame`](packets::server::JoinGame). State switches to `Game`.
//! * Server sends local chunks, entities, etc. and continues sending these
//! as the client moves.
//! * Either peer disconnects and sends [`Disconnect`](packets::shared::Disconnect)
//! before doing so.

/// Current protocol version. Increment when a new release is made
/// with a change in the protocol.
pub const PROTOCOL_VERSION: u32 = 0;

pub mod bridge;
pub mod packets;

#[doc(inline)]
pub use bridge::Bridge;

/// A state in the protocol.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum State {
    Login,
    Game,
}
