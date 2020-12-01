//! Defines the base components shared by entities
//! between client and server.

use glam::{Vec2, Vec3A};
use hecs::Bundle;

pub mod player;

/// The "base" bundle of components for an entity. All non-block
/// entities require these components.
#[derive(Bundle)]
pub struct BaseBundle {
    pub pos: Pos,
    pub orient: Orient,
}

/// The position of an entity. This is
/// the center of the bottom of its bounding box.
/// _Mandatory_ for all non-block entities.
#[derive(Copy, Clone, Debug)]
pub struct Pos(pub Vec3A);

/// The orientation of an entity.
///
/// Represented as (yaw, pitch), where `yaw` is the rotation
/// around the X-axis and `pitch` is the rotation around the Y-axis.
/// Both are measured in radians.
#[derive(Default, Copy, Clone, Debug)]
pub struct Orient(pub Vec2);

/// The velocity of an entity, measured in blocks
/// per second.
#[derive(Default, Copy, Clone, Debug)]
pub struct Vel(pub Vec3A);
