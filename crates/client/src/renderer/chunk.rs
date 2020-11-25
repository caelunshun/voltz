use super::utils::TextureArray;

mod mesher;

/// The chunk renderer. Responsible for
/// 1) Maintaining a mesh for each chunk to be rendered.
/// 2) Maintaining a texture array containing block textures.
/// 3) Rendering each visible chunk.
pub struct ChunkRenderer {
    block_textures: TextureArray,
}
