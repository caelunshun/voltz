//! Systems that notify the server of client actions.

use common::{Orient, Pos, System, SystemExecutor};
use glam::{Vec2, Vec3A};
use protocol::packets::{client::UpdatePosition, ClientPacket};

use crate::game::Game;

pub fn setup(systems: &mut SystemExecutor<Game>) {
    systems.add(NotifyMovement::default());
}

/// Notifies the server of changes in position and orientation.
#[derive(Default)]
struct NotifyMovement {
    old_state: Option<(Vec3A, Vec2)>,
}

impl System<Game> for NotifyMovement {
    fn run(&mut self, game: &mut Game) {
        // Determine if position or orient has changed and if
        // so, send UpdatePosition.
        let pos = game.player_ref().get::<Pos>().unwrap().0;
        let orient = game.player_ref().get::<Orient>().unwrap().0;
        let changed = match self.old_state.replace((pos, orient)) {
            Some((old_pos, old_orient)) => pos != old_pos || orient != old_orient,
            None => true,
        };

        if changed {
            let packet = ClientPacket::UpdatePosition(UpdatePosition {
                new_pos: pos,
                new_orient: orient,
            });
            game.bridge().send(packet);
        }
    }
}
