//! Generation of a 2D biome grid which defines a biome for each column of blocks.
//!
//! # Implementation
//! This biome generator is based on the "grow" technique pioneered by the Cuberite
//! project for generating Minecraft biomes. We operate on an array of integers, which
//! we can "zoom" to add detail, "smooth" to remove noise, and apply other operations
//! to map integers to biomes. The final result is an array of biomes.

use crate::Settings;

/// Generates the biome grid.
pub fn generate(width: usize, height: usize, settings: Settings) {}
