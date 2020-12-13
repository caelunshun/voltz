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

use std::{mem::take, sync::Arc};

use biomes::BiomeGenerator;
use common::{world::ZoneBuilder, ChunkPos};
use futures_executor::block_on;
use region::{Region, RegionGenerator, REGION_CHUNKS, REGION_DIM};

pub mod biomes;
pub mod region;

pub struct WorldGenerator {
    biome_generator: BiomeGenerator,
    region_generator: RegionGenerator,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
}

impl WorldGenerator {
    pub fn new(device: &Arc<wgpu::Device>, queue: &Arc<wgpu::Queue>) -> Self {
        let device = Arc::clone(device);
        let queue = Arc::clone(queue);
        let biome_generator = BiomeGenerator::new(&device);
        let region_generator = RegionGenerator::new(&device);
        Self {
            biome_generator,
            region_generator,
            device,
            queue,
        }
    }

    /// Fills a zone with generated blocks.
    /// This function is expensive and will block on GPU operations.
    pub fn generate_into_zone(&self, zone: &mut ZoneBuilder, seed: u32) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        let biome_payload = self
            .biome_generator
            .prepare(&self.device, seed, REGION_DIM as u32);
        let biome_grid = biome_payload.output_texture();
        let region_payload = self.region_generator.prepare(&self.device, biome_grid);

        {
            let mut pass = encoder.begin_compute_pass();
            self.biome_generator
                .execute(&biome_payload, &mut pass, &self.queue);
            self.region_generator.execute(&region_payload, &mut pass);
        }

        let region = block_on(self.region_generator.load_region_from_gpu(
            &region_payload,
            &self.device,
            &self.queue,
            encoder,
        ));
        self.move_region_into_zone(region, zone, [0, 0, 0]);
    }

    fn move_region_into_zone(
        &self,
        mut region: Region,
        zone: &mut ZoneBuilder,
        offset_in_chunks: [i32; 3],
    ) {
        for chunk_x in 0..REGION_CHUNKS as i32 {
            for chunk_y in 0..REGION_CHUNKS as i32 {
                for chunk_z in 0..REGION_CHUNKS as i32 {
                    let x = chunk_x + offset_in_chunks[0];
                    let y = chunk_y + offset_in_chunks[1];
                    let z = chunk_z + offset_in_chunks[2];
                    let pos = ChunkPos { x, y, z };
                    let chunk = take(
                        &mut region.chunks[chunk_x as usize][chunk_y as usize][chunk_z as usize],
                    );
                    let _ = zone.add_chunk(pos, chunk);
                }
            }
        }
    }
}
