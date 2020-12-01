//! Utilities for physics and collision detection.

pub mod collision;

pub use collision::Aabb;
use common::BlockPos;
use glam::{vec3a, Vec3A};

/// Ticks an entity for physics.
pub fn do_tick(
    bounds: Aabb,
    pos: &mut Vec3A,
    vel: &mut Vec3A,
    mut is_solid: impl FnMut(BlockPos) -> bool,
) {
    let drag_factor = 0.98;
    *vel *= drag_factor;

    let new_pos = *pos + *vel;
    let new_pos = collision::resolve_collisions(bounds, *pos, new_pos, &mut is_solid);
    *pos = new_pos;

    let on_ground = is_on_ground(*pos, &mut is_solid);

    let gravity = -0.04;
    if !on_ground {
        vel.y += gravity;
    }

    let friction_factor = 0.6;
    if on_ground {
        *vel *= friction_factor;
    }
}

/// Determines if an entity is standing on the ground.
pub fn is_on_ground(pos: Vec3A, mut is_solid: impl FnMut(BlockPos) -> bool) -> bool {
    if pos.y % 1.0 <= 0.05 {
        is_solid(BlockPos::from_pos(pos - vec3a(0., 1., 0.)))
    } else {
        false
    }
}
