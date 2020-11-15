#![no_main]
use common::{blocks, BlockId, BlockPos, Chunk, ChunkPos, Zone};
use libfuzzer_sys::{arbitrary, arbitrary::Arbitrary, fuzz_target};
use std::collections::HashMap;

const CHUNK_DIM: i32 = common::chunk::CHUNK_DIM as i32;

#[derive(Arbitrary, Debug)]
struct Input {
    minx: i16,
    miny: i16,
    minz: i16,
    maxx: i16,
    maxy: i16,
    maxz: i16,

    operations: Vec<Operation>,
}

#[derive(Arbitrary, Debug)]
enum Operation {
    Set(i32, i32, i32, u32, u32),
    Get(i32, i32, i32),
}

fuzz_target!(|data: Input| {
    let mut builder = Zone::builder(
        ChunkPos {
            x: data.minx as i32,
            y: data.miny as i32,
            z: data.minz as i32,
        },
        ChunkPos {
            x: data.maxx as i32,
            y: data.maxy as i32,
            z: data.maxz as i32,
        },
    );
    if builder.needed_chunks() > 1024 {
        return;
    }

    let min = builder.min();
    let max = builder.max();

    for x in min.x..=max.x {
        for y in min.y..=max.y {
            for z in min.z..=max.z {
                builder
                    .add_chunk(ChunkPos { x, y, z }, Chunk::new())
                    .unwrap();
            }
        }
    }

    let mut zone = builder.build().ok().unwrap();
    let mut oracle = HashMap::new();

    for op in data.operations {
        match op {
            Operation::Set(x, y, z, kind, state) => {
                let block = BlockId::from_raw_parts(kind, state);
                let pos = BlockPos {
                    x: x.max(zone.min().x * CHUNK_DIM)
                        .min(zone.max().x * CHUNK_DIM),
                    y: y.max(zone.min().y * CHUNK_DIM)
                        .min(zone.max().y * CHUNK_DIM),
                    z: z.max(zone.min().z * CHUNK_DIM)
                        .min(zone.max().z * CHUNK_DIM),
                };
                zone.set_block(pos, block).unwrap();
                oracle.insert(pos, block);
            }
            Operation::Get(x, y, z) => {
                let pos = BlockPos {
                    x: x.max(zone.min().x * CHUNK_DIM)
                        .min(zone.max().x * CHUNK_DIM),
                    y: y.max(zone.min().y * CHUNK_DIM)
                        .min(zone.max().y * CHUNK_DIM),
                    z: z.max(zone.min().z * CHUNK_DIM)
                        .min(zone.max().z * CHUNK_DIM),
                };
                let block = zone.block(pos).unwrap();
                assert_eq!(
                    block,
                    oracle
                        .get(&pos)
                        .copied()
                        .unwrap_or(BlockId::new(blocks::Air))
                );
            }
        }
    }
});
