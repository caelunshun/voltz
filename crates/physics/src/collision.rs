//! Collision detection.

use std::{cmp::Ordering, f32::INFINITY, mem::swap, ops::Add};

use common::BlockPos;
use glam::{vec3a, Vec3A};

/// An axis-aligned bounding box.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Aabb {
    pub min: Vec3A,
    pub max: Vec3A,
}

impl Aabb {
    pub fn half_width(self) -> f32 {
        (self.max.x - self.min.x) / 2.
    }

    pub fn half_height(self) -> f32 {
        (self.max.y - self.min.y) / 2.
    }

    pub fn half_depth(self) -> f32 {
        (self.max.z - self.min.z) / 2.
    }

    pub fn corners(self) -> [Vec3A; 8] {
        let dist = self.max - self.min;
        let bottom = [
            self.min,
            self.min + Vec3A::unit_x() * dist,
            self.min + Vec3A::unit_z() * dist,
            self.min + Vec3A::unit_x() * dist + Vec3A::unit_x() * dist,
        ];
        let uy = Vec3A::unit_y() * dist;
        [
            bottom[0],
            bottom[1],
            bottom[2],
            bottom[3],
            bottom[0] + uy,
            bottom[1] + uy,
            bottom[2] + uy,
            bottom[3] + uy,
        ]
    }

    pub fn toi_with_ray(self, origin: Vec3A, dir: Vec3A) -> Option<f32> {
        let Aabb { min, max } = self;
        let mut tmin = (min.x - origin.x) / dir.x;
        let mut tmax = (max.x - origin.x) / dir.x;

        if tmin > tmax {
            swap(&mut tmin, &mut tmax);
        }

        let mut tymin = (min.y - origin.y) / dir.y;
        let mut tymax = (max.y - origin.y) / dir.y;

        if tymin > tymax {
            swap(&mut tymin, &mut tymax)
        }

        if (tymin > tymax) || (tymin > tmax) {
            return None;
        }

        if tymin > tmin {
            tmin = tymin;
        }

        if tymax < tmax {
            tmax = tymax;
        }

        let mut tzmin = (min.z - origin.z) / dir.z;
        let mut tzmax = (max.z - origin.z) / dir.z;

        if tzmin > tzmax {
            swap(&mut tzmin, &mut tzmax);
        }

        if (tmin > tzmax) || (tzmin > tmax) {
            return None;
        }

        if tzmin > tmin {
            tmin = tzmin;
        }

        if tmin.is_nan() {
            None
        } else {
            Some(tmin)
        }
    }
}

impl Add<Vec3A> for Aabb {
    type Output = Self;

    fn add(self, rhs: Vec3A) -> Self::Output {
        Self {
            min: self.min + rhs,
            max: self.max + rhs,
        }
    }
}

/// Return value from [`collide_with_zone`]. Contains
/// a collision vector for each of the six faces of the AABB.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CollisionWithZone {
    pub top: Option<f32>,
    pub bottom: Option<f32>,
    pub posx: Option<f32>,
    pub negx: Option<f32>,
    pub posz: Option<f32>,
    pub negz: Option<f32>,
}

/// Determines collisions between an AABB and a zone.
/// `is_solid` should take the position of a block and
/// output whether that block is solid. Currently we
/// do not support non-full block models.
///
/// Returns `None` if `is_solid` returns `None`. This
/// could happen if a chunk containing the necessary blocks
/// is not loaded, for example.
pub fn collide_with_zone(
    bounds: Aabb,
    pos: Vec3A,
    mut is_solid: impl FnMut(Vec3A) -> Option<bool>,
) -> Option<CollisionWithZone> {
    // Start at the center of the bounding box.
    let pos = pos + glam::vec3a(0., bounds.half_height(), 0.);

    // Compute collisions for each of the six faces.
    let top = collision_along_axis(
        pos,
        bounds.half_height(),
        Vec3A::unit_y(),
        &mut is_solid,
        true,
    )?;
    let bottom = collision_along_axis(
        pos,
        bounds.half_height(),
        -Vec3A::unit_y(),
        &mut is_solid,
        false,
    )?;
    let posx = collision_along_axis(
        pos,
        bounds.half_width(),
        Vec3A::unit_x(),
        &mut is_solid,
        true,
    )?;
    let negx = collision_along_axis(
        pos,
        bounds.half_width(),
        -Vec3A::unit_x(),
        &mut is_solid,
        false,
    )?;
    let posz = collision_along_axis(
        pos,
        bounds.half_depth(),
        Vec3A::unit_z(),
        &mut is_solid,
        true,
    )?;
    let negz = collision_along_axis(
        pos,
        bounds.half_depth(),
        -Vec3A::unit_z(),
        &mut is_solid,
        false,
    )?;

    Some(CollisionWithZone {
        top,
        bottom,
        posx,
        negx,
        posz,
        negz,
    })
}

fn collision_along_axis(
    start: Vec3A,
    max_dist: f32,
    stride: Vec3A,
    is_solid: &mut impl FnMut(Vec3A) -> Option<bool>,
    floor_or_ceil: bool,
) -> Option<Option<f32>> {
    let mut pos = start;

    while pos.distance_squared(start) <= max_dist.powi(2) {
        if is_solid(pos)? {
            let dist = pos.distance(start);
            let collision = if floor_or_ceil {
                dist.floor()
            } else {
                dist.ceil()
            };
            return Some(Some(collision));
        }
        pos += stride;
    }

    Some(None)
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct RayImpact {
    pub distance: f32,
}

/// Ray traces into a zone to determine the first
/// block impacted by the given ray. Returns `None`
/// if the raytrace travels `max_distance_squared` without
/// encountering a block. Otherwise, returns `Some(distance)` with the
/// distance from `origin` to the block.
pub fn raytrace_in_zone(
    origin: Vec3A,
    dir: Vec3A,
    max_distance_squared: f32,
    mut is_solid: impl FnMut(BlockPos) -> bool,
) -> Option<RayImpact> {
    if dir == vec3a(0.0, 0.0, 0.0) {
        return None;
    }

    // Go along path of ray and find all points
    // where one or more coordinates are integers.
    // Any position with an integer component
    // is a block boundary, which means a block
    // could be found at the position.
    //
    // This algorithm is based on "A Fast Voxel Traversal Algorithm for Ray Tracing"
    // by John Amanatides and Andrew Woo and has been adapted
    // to our purposes.

    let direction = dir.normalize();

    let mut dist_traveled = Vec3A::zero();

    let mut step = Vec3A::zero();
    let mut delta = vec3a(INFINITY, INFINITY, INFINITY);
    let mut next = vec3a(INFINITY, INFINITY, INFINITY);

    match direction.x.partial_cmp(&0.0).unwrap() {
        Ordering::Greater => {
            step.x = 1.;
            delta.x = 1.0 / direction.x;
            next.x = ((origin.x + 1.0).floor() - origin.x) / direction.x; // Brings X position to next integer
        }
        Ordering::Less => {
            step.x = -1.;
            delta.x = (1.0 / direction.x).abs();
            next.x = ((origin.x - (origin.x - 1.0).ceil()) / direction.x).abs();
        }
        _ => (),
    }

    match direction.y.partial_cmp(&0.0).unwrap() {
        Ordering::Greater => {
            step.y = 1.;
            delta.y = 1.0 / direction.y;
            next.y = ((origin.y + 1.0).floor() - origin.y) / direction.y;
        }
        Ordering::Less => {
            step.y = -1.;
            delta.y = (1.0 / direction.y).abs();
            next.y = ((origin.y - (origin.y - 1.0).ceil()) / direction.y).abs();
        }
        _ => (),
    }

    match direction.z.partial_cmp(&0.0).unwrap() {
        Ordering::Greater => {
            step.z = 1.;
            delta.z = 1.0 / direction.z;
            next.z = ((origin.z + 1.0).floor() - origin.z) / direction.z;
        }
        Ordering::Less => {
            step.z = -1.;
            delta.z = (1.0 / direction.z).abs();
            next.z = ((origin.z - (origin.z - 1.0).ceil()) / direction.z).abs();
        }
        _ => (),
    }

    let mut current_pos = BlockPos::from_pos(origin);

    while dist_traveled.length_squared() < max_distance_squared {
        if is_solid(current_pos) {
            // Calculate world-space position of impact.
            let bounds = Aabb {
                min: Vec3A::zero(),
                max: vec3a(1., 1., 1.),
            } + vec3a(
                current_pos.x as f32,
                current_pos.y as f32,
                current_pos.z as f32,
            );
            if let Some(distance) = bounds.toi_with_ray(origin, dir) {
                return Some(RayImpact { distance });
            }
        }

        if next.x < next.y {
            if next.x < next.z {
                next.x += delta.x;
                current_pos.x += step.x as i32;
                dist_traveled.x += 1.0;
            } else {
                next.z += delta.z;
                current_pos.z += step.z as i32;
                dist_traveled.z += 1.0;
            }
        } else if next.y < next.z {
            next.y += delta.y;
            current_pos.y += step.y as i32;
            dist_traveled.y += 1.0;
        } else {
            next.z += delta.z;
            current_pos.z += step.z as i32;
            dist_traveled.z += 1.0;
        }
    }

    None
}

/// Given:
/// * A bounding box
/// * The initial position of the bounding box
/// * The target position of the bounding box
/// returns a new target position accounting for
/// collisions on the path between the two position.
pub fn resolve_collisions(
    bounds: Aabb,
    start: Vec3A,
    end: Vec3A,
    mut is_solid: impl FnMut(BlockPos) -> bool,
) -> Vec3A {
    let mut pos = end;
    let bottom = BlockPos::from_pos(end);
    if is_solid(bottom) {
        pos.y = pos.y.ceil();
    }

    pos
}

/*
pub fn resolve_collisions(
    bounds: Aabb,
    start: Vec3A,
    end: Vec3A,
    mut is_solid: impl FnMut(BlockPos) -> bool,
) -> Vec3A {
    if end == start {
        return end;
    }

    // We take the eight corner points of the bbox
    // and all the lattice points on the bbox faces.
    // We then raytrace these points.
    let bounds = bounds + start;
    let dir = (end - start).normalize();
    let dist_squared = (end - start).length_squared();
    let corners = bounds.corners();

    let mut min_distance = dist_squared.sqrt();
    for &corner in &corners {
        let impact = raytrace_in_zone(corner, dir, dist_squared, &mut is_solid);
        if let Some(impact) = impact {
            if min_distance > impact.distance {
                min_distance = impact.distance;
            }
        }
    }

    start + dir * min_distance
}
*/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_collisions() {
        let collisions = collide_with_zone(
            Aabb {
                min: Vec3A::new(0., 0., 0.),
                max: Vec3A::new(1., 2., 1.),
            },
            Vec3A::zero(),
            |_| Some(false),
        )
        .unwrap();

        assert_eq!(
            collisions,
            CollisionWithZone {
                top: None,
                bottom: None,
                posx: None,
                negx: None,
                posz: None,
                negz: None
            }
        )
    }

    #[test]
    fn collision_bottom() {
        let collisions = collide_with_zone(
            Aabb {
                min: Vec3A::new(0., 0., 0.),
                max: Vec3A::new(1., 2., 1.),
            },
            Vec3A::zero(),
            |pos| Some(pos.y >= 2.),
        )
        .unwrap();

        assert_eq!(
            collisions,
            CollisionWithZone {
                top: Some(1.),
                bottom: None,
                posx: None,
                negx: None,
                posz: None,
                negz: None
            }
        );
    }

    #[test]
    fn aabb_toi() {
        let toi = Aabb {
            min: Vec3A::zero(),
            max: vec3a(1., 1., 1.),
        }
        .toi_with_ray(vec3a(0.5, 100., 0.5), -Vec3A::unit_y());
        assert_eq!(toi, Some(99.));
    }

    #[test]
    fn raytrace_empty() {
        let impact = raytrace_in_zone(Vec3A::zero(), Vec3A::unit_y(), 100., |_| false);
        assert_eq!(impact, None);
    }

    #[test]
    fn raytrace_to_block() {
        let impact = raytrace_in_zone(vec3a(0.5, 0., 0.5), Vec3A::unit_y(), 100., |pos| pos.y == 2);
        assert_eq!(impact, Some(RayImpact { distance: 2. }));
    }
}
