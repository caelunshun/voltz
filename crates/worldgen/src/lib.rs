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

#![allow(warnings)] // temp until I work on worldgen

use common::{blocks, chunk::CHUNK_DIM, BlockId, Chunk, ChunkPos, Zone};
use rayon::prelude::*;
use simdnoise::NoiseBuilder;

mod biomes;
mod density;
mod trilerp;

/// The Y coordinate of sea level.
pub const SEA_LEVEL: usize = 64;

/// Settings used for world generation. The same settings
/// will always produce the same generated blocks.
#[derive(Debug)]
pub struct Settings {
    /// The seed used to initialize the RNG, noise functions, etc.
    /// Different parts of the world generation pipeline need
    /// to add different offsets to this seed.
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
pub fn generate(zone: &mut Zone, settings: Settings) {
    let stone = BlockId::new(blocks::Stone);
    let grass = BlockId::new(blocks::Grass);

    let width = zone.x_dim() * CHUNK_DIM;
    let depth = zone.z_dim() * CHUNK_DIM;
    let heightmap = NoiseBuilder::fbm_2d_offset(0., width, 0., depth)
        .with_seed(settings.seed as i32)
        .generate()
        .0;
    let amplification = NoiseBuilder::fbm_2d_offset(0., width, 0., depth)
        .with_seed(settings.seed.wrapping_add(1) as i32)
        .with_freq(0.01)
        .generate()
        .0;
    let min = zone.min();

    zone.par_chunks_mut()
        .for_each(|(pos, chunk): (ChunkPos, &mut Chunk)| {
            for x in 0..CHUNK_DIM {
                for z in 0..CHUNK_DIM {
                    // sample from heightmap
                    let ix = x + (pos.x - min.x) as usize * CHUNK_DIM;
                    let iz = z + (pos.z - min.z) as usize * CHUNK_DIM;
                    let height = heightmap[iz * width + ix];
                    let mut height = (height + 2.) as i32 + 64;
                    let amplification = ((amplification[iz * width + ix] + 2.0) * 8.0) as i32;
                    height += amplification.max(0);

                    let chunk_height = pos.y * CHUNK_DIM as i32;
                    for y in chunk_height..height.min(chunk_height + CHUNK_DIM as i32) {
                        chunk.set(x, (y % CHUNK_DIM as i32) as usize, z, grass);
                    }
                }
            }
        });
}
