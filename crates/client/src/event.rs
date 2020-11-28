use common::ChunkPos;

/// A chunk has been loaded.
pub struct ChunkLoaded {
    pub pos: ChunkPos,
}

/// A chunk has been unloaded.
pub struct ChunkUnloaded {
    pub pos: ChunkPos,
}
