//! Generates a density grid for each chunk. The density
//! grid is stored as a bitset where set bits correspond
//! to solid blocks and unset bits correspond to air.

use bumpalo::Bump;
use utils::BitSet;

/// The generated density grid for a chunk.
#[derive(Clone)]
pub struct DensityChunk<'bump> {
    values: BitSet<&'bump Bump>,
}
