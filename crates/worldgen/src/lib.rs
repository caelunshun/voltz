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

pub mod biomes;
