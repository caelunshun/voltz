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
    let dirt = BlockId::new(blocks::Dirt);
    let grass = BlockId::new(blocks::Grass);
    zone.par_chunks_mut()
        .for_each(|(pos, chunk): (ChunkPos, &mut Chunk)| {
            let x = pos.x * CHUNK_DIM as i32;
            let y = pos.y * CHUNK_DIM as i32;
            let z = pos.z * CHUNK_DIM as i32;
            let density = NoiseBuilder::fbm_3d_offset(
                x as f32, CHUNK_DIM, y as f32, CHUNK_DIM, z as f32, CHUNK_DIM,
            )
            .with_seed((settings.seed % i32::MAX as u64) as i32)
            .with_freq(0.1)
            .generate()
            .0;
            let threshold = 0.05;

            for x in 0..CHUNK_DIM {
                for y in 0..CHUNK_DIM {
                    for z in 0..CHUNK_DIM {
                        let index = z * CHUNK_DIM * CHUNK_DIM + y * CHUNK_DIM + x;
                        let density = density[index];
                        if density < threshold {
                            let block = if density < -0.1 { stone } else { dirt };
                            chunk.set(x, y, z, block);
                        }
                    }
                }
            }
        });
}
