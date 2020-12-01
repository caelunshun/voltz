use common::{
    entity::player::{Username, View},
    ChunkPos, Orient, Pos,
};
use glam::{Vec2, Vec3A};
use hecs::Entity;
use protocol::{
    bridge::ToClient,
    packets::ClientPacket,
    packets::ServerPacket,
    packets::{
        client::ClientInfo, server::JoinGame, server::ServerInfo, shared::Disconnect, SharedPacket,
    },
    Bridge, PROTOCOL_VERSION,
};

use crate::{event::PlayerJoined, game::Game, VIEW_DISTANCE};

/// A connection to a client.
pub struct Connection {
    bridge: Bridge<ToClient>,
    state: ConnectionState,
    disconnected: bool,
}

impl Connection {
    pub fn new(bridge: Bridge<ToClient>) -> Self {
        Self {
            bridge,
            state: ConnectionState::Login,
            disconnected: false,
        }
    }

    /// Polls for packets and invokes packet handlers.
    /// If we're in the Login state and we advance to the Game
    /// state, a new player will be added to the ECS.
    pub fn tick(&mut self, game: &mut Game) {
        if self.bridge.is_disconnected() {
            self.disconnect(Some("bridge died".to_owned()));
        }
        match self.state {
            ConnectionState::Login => self.advance_login(game),
            ConnectionState::Game { .. } => self.handle_packets(game),
        }
    }

    fn advance_login(&mut self, game: &mut Game) {
        for packet in self.bridge.flush_received() {
            match packet {
                ClientPacket::ClientInfo(client_info) => {
                    log::debug!("Received ClientInfo from client: {:?}", client_info);

                    let server_info = ServerInfo {
                        protocol_version: PROTOCOL_VERSION,
                        implementation: format!("voltz-server:{}", env!("CARGO_PKG_VERSION")),
                    };
                    self.bridge.send(ServerPacket::ServerInfo(server_info));

                    let pos = glam::vec3a(0., 66., 0.);
                    let orient = glam::vec2(0., 0.);
                    let vel = Vec3A::zero();
                    let join_game = JoinGame { pos, orient, vel };
                    self.bridge.send(ServerPacket::JoinGame(join_game));

                    self.spawn_player(game, pos, orient, vel, client_info);
                }
                _ => {
                    log::debug!(
                        "Received unexpected packet from client during login state. Disconnecting.",
                    );
                    self.disconnect(Some(String::from(
                        "received unexpected packet during the login state",
                    )));
                }
            }
        }
    }

    fn spawn_player(
        &mut self,
        game: &mut Game,
        pos: Vec3A,
        orient: Vec2,
        vel: Vec3A,
        client_info: ClientInfo,
    ) {
        log::info!("{} joined the game.", client_info.username);
        let pos = Pos(pos);
        let orient = Orient(orient);

        let player = game.ecs_mut().spawn((
            pos,
            orient,
            vel,
            Username(client_info.username),
            self.bridge.clone(),
            View::new(ChunkPos::from_pos(pos), VIEW_DISTANCE),
        ));
        game.events().push(PlayerJoined { player });

        self.state = ConnectionState::Game { player };
    }

    fn handle_packets(&mut self, game: &mut Game) {
        let player = match self.state {
            ConnectionState::Game { player } => player,
            _ => unreachable!(),
        };
        let entity = game.ecs().entity(player).unwrap();

        for packet in self.bridge.flush_received() {
            match packet {
                ClientPacket::Shared(shared) => match shared {
                    SharedPacket::Disconnect(disconnect) => {
                        log::info!("{} left the game.", entity.get::<Username>().unwrap().0);
                        if let Some(reason) = disconnect.reason {
                            log::debug!("Reason for disconnect: {}", reason);
                        }
                        game.ecs_mut().despawn(player).unwrap();
                        return;
                    }
                },
                ClientPacket::ClientInfo(_) => {
                    log::debug!(
                        "Received ClientInfo during game state from {}.",
                        entity.get::<Username>().unwrap().0
                    );
                    self.disconnect(Some("received ClientInfo during game state".to_owned()));
                }
                ClientPacket::UpdatePosition(pos) => {
                    entity.get_mut::<Pos>().unwrap().0 = pos.new_pos;
                    entity.get_mut::<Orient>().unwrap().0 = pos.new_orient;
                }
            }
        }
    }

    fn disconnect(&mut self, reason: Option<String>) {
        self.bridge
            .send(ServerPacket::Shared(SharedPacket::Disconnect(Disconnect {
                reason,
            })));
        self.disconnected = true;
    }
}

enum ConnectionState {
    /// We're in the login phase, still performing the handshake.
    Login,
    /// We're in the game phase, and the player exists.
    Game {
        /// The player's entity.
        player: Entity,
    },
}
