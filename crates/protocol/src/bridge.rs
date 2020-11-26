use flume::{Receiver, Sender};

use crate::packets::{client::ClientPacket, server::ServerPacket};

pub trait Side {
    type SendPacket;
    type RecvPacket;
}

#[derive(Debug)]
pub struct ToServer;

impl Side for ToServer {
    type SendPacket = ClientPacket;
    type RecvPacket = ServerPacket;
}

#[derive(Clone)]
pub struct ToClient;

impl Side for ToClient {
    type SendPacket = ServerPacket;
    type RecvPacket = ClientPacket;
}

/// The client-server bridge. Provides methods to communicate
/// with the peer.
///
/// This serves as an abstraction over singleplayer and multiplayer. In singleplayer,
/// the client and server run in the same process and different threads, so they
/// communicate via a channel. Whereas in multiplayer, client and server communicate
/// over the network via QUIC.
#[derive(Debug, Clone)]
pub struct Bridge<S: Side> {
    sender: Sender<S::SendPacket>,
    receiver: Receiver<S::RecvPacket>,
}

/// Creates a new singleplayer `Bridge` pair.
pub fn singleplayer() -> (Bridge<ToServer>, Bridge<ToClient>) {
    let (server_sender, server_receiver) = flume::unbounded();
    let (client_sender, client_receiver) = flume::unbounded();

    (
        Bridge {
            sender: client_sender,
            receiver: server_receiver,
        },
        Bridge {
            sender: server_sender,
            receiver: client_receiver,
        },
    )
}

impl<S> Bridge<S>
where
    S: Side,
{
    /// Returns an iterator over buffered packets.
    pub fn flush_received<'a>(&'a self) -> impl Iterator<Item = S::RecvPacket> + 'a {
        self.receiver.try_iter()
    }

    /// Sends a packet to the peer.
    pub fn send(&self, packet: S::SendPacket) {
        let _ = self.sender.send(packet);
    }

    /// Returns whether an error has occurred resulting in a
    /// disconnection from the peer.
    pub fn is_disconnected(&self) -> bool {
        self.receiver.is_disconnected() || self.sender.is_disconnected()
    }
}
