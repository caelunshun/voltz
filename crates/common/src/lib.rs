#![feature(const_generics)]
#![allow(incomplete_features)]

pub mod biome;
pub mod block;
pub mod chunk;
pub mod entity;
pub mod event;
pub mod system;
pub mod world;

pub use block::{blocks, BlockId};
pub use chunk::{Chunk, ChunkPos};
pub use entity::{Orient, Pos};
pub use system::{System, SystemExecutor};
pub use world::{BlockPos, World, Zone};
