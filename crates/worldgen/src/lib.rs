//! Voxel world generator for Voltz.
//!
//! This world generator currently generates the entire world in one go,
//! in comparison to Minecraft's worldgen which does one chunk column at a time.
//! Most algorithms, however, are parallelized.
//!
//! # Pipeline
//! Data is fed through a number of stages before
//! the final region is produced:
//!
//! `Biome grid => density grid => composition => post-processing`
//!
//! The biome grid generates a 2D grid of biomes, one for each block column. The density
//! grid generates a 3D bitset where bits are set for non-air blocks. Composition takes
//! the density and biome grids and generates chunks with actual blocks. Finally, post-processing
//! adds features, such as trees and caves.

use common::Zone;

mod biomes;

/// The Y coordinate of sea level.
pub const SEA_LEVEL: usize = 100;

/// Settings used for world generation. The same settings
/// will always produce the same generated blocks.
#[derive(Debug)]
pub struct Settings {
    /// The seed used to initialize the RNG, noise functions, etc.
    pub seed: u64,
}

/// Generates a zone of blocks, given a seed.
///
/// Each block in the zone will be overriden with the generated blocks.
/// The zone is expected to be initialized with air.
///
/// This function is deterministic with respect to the seed.
/// Given the same seed, the exact same blocks will be generated.
///
/// This is an expensive function and should not be used on the main thread.
///
/// This function uses `rayon` to parallelize generation.
pub fn generate_into(zone: &mut Zone, settings: Settings) {}
