//! Systems for miscallaneous entity functionality.

use common::{blocks, entity::Vel, BlockId, Pos, SystemExecutor};
use physics::Aabb;

use crate::game::Game;

pub fn setup(systems: &mut SystemExecutor<Game>) {
    systems.add(physics_system);
}

fn physics_system(game: &mut Game) {
    for (_, (pos, vel, &bounds)) in game.ecs().query::<(&mut Pos, &mut Vel, &Aabb)>().iter() {
        physics::do_tick(bounds, &mut pos.0, &mut vel.0, game.dt(), |pos| {
            game.main_zone().block(pos) != Some(BlockId::new(blocks::Air))
        });
    }
}
