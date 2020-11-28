use crate::ChunkPos;

use super::BaseBundle;

/// Base components required for all players.
pub type PlayerBundle = BaseBundle;

/// A player's username.
#[derive(Debug)]
pub struct Username(pub String);

/// A view, encapsulating the set of chunks visible to a player.
///
/// A player's view is defined as a cube with the center equal
/// to the player's position.
///
/// Operates on _chunks, not blocks_.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct View {
    center: ChunkPos,
    distance: i32,
}

impl View {
    /// Creates a `View` from a center chunk (the position of the player)
    /// and the view distance.
    pub fn new(center: ChunkPos, distance: u32) -> Self {
        Self {
            center,
            distance: distance as i32,
        }
    }

    /// Creates an empty view containing no chunks.
    pub fn empty() -> Self {
        Self::new(ChunkPos::default(), 0)
    }

    pub fn center(self) -> ChunkPos {
        self.center
    }

    pub fn distance(self) -> u32 {
        self.distance as u32
    }

    /// Iterates over chunks visible to the player.
    pub fn iter(self) -> impl Iterator<Item = ChunkPos> {
        Self::iter_3d(
            self.min_x(),
            self.min_y(),
            self.min_z(),
            self.max_x(),
            self.max_y(),
            self.max_z(),
        )
    }

    /// Determines whether the given chunk is visible.
    pub fn contains(&self, pos: ChunkPos) -> bool {
        pos.x >= self.min_x()
            && pos.x <= self.max_x()
            && pos.y >= self.min_y()
            && pos.y <= self.max_y()
            && pos.z >= self.min_z()
            && pos.z <= self.max_z()
    }

    fn iter_3d(
        min_x: i32,
        min_y: i32,
        min_z: i32,
        max_x: i32,
        max_y: i32,
        max_z: i32,
    ) -> impl Iterator<Item = ChunkPos> {
        (min_x..=max_x)
            .flat_map(move |x| (min_y..=max_y).map(move |y| (x, y)))
            .flat_map(move |(x, y)| (min_z..=max_z).map(move |z| (x, y, z)))
            .map(|(x, y, z)| ChunkPos { x, y, z })
    }

    /// Returns the minimum X chunk coordinate.
    pub fn min_x(self) -> i32 {
        self.center.x - self.distance
    }

    /// Returns the minimum Y coordinate.
    pub fn min_y(self) -> i32 {
        self.center.y - self.distance
    }

    /// Returns the minimum Z coordinate.
    pub fn min_z(self) -> i32 {
        self.center.z - self.distance
    }

    /// Returns the maximum X coordinate.
    pub fn max_x(self) -> i32 {
        self.center.x + self.distance
    }

    /// Returns the maximum Y coordinate.
    pub fn max_y(self) -> i32 {
        self.center.y + self.distance
    }

    /// Returns the maximum Z coordinate.
    pub fn max_z(self) -> i32 {
        self.center.z + self.distance
    }
}
