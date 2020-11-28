//! Definitions for each block.

use block_macros::Block;

#[derive(Block)]
#[block(slug = "air", display_name = "Air")]
pub struct Air;

#[derive(Block)]
#[block(slug = "dirt", display_name = "Dirt")]
pub struct Dirt;

#[derive(Block)]
#[block(slug = "stone", display_name = "Stone")]
pub struct Stone;

#[derive(Block)]
#[block(slug = "grass", display_name = "Grass")]
pub struct Grass;

#[derive(Block)]
#[block(slug = "melium", display_name = "Melium")]
pub struct Melium;
