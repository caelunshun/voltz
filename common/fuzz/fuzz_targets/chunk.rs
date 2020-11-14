#![no_main]
use libfuzzer_sys::fuzz_target;

use common::{blocks, BlockId, Chunk};
use std::collections::HashMap;

fuzz_target!(|input: Vec<(bool, usize, usize, usize, u32, u32)>| {
    let mut chunk = Chunk::new();
    let mut oracle = HashMap::new();

    for (should_set, x, y, z, kind, state) in input {
        if x >= 16 || y >= 16 || z >= 16 {
            continue;
        }

        let block = BlockId::from_raw_parts(kind, state);

        if should_set {
            chunk.set(x, y, z, block);
            oracle.insert((x, y, z), block);
        } else {
            let block = chunk.get(x, y, z);
            assert_eq!(
                block,
                oracle
                    .get(&(x, y, z))
                    .copied()
                    .unwrap_or_else(|| BlockId::new(blocks::Air))
            );
        }
    }
});
