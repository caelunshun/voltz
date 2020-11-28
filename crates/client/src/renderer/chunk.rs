use std::sync::Arc;

use ahash::{AHashMap, AHashSet};
use anyhow::{bail, Context};
use common::ChunkPos;
use mesher::{ChunkMesher, GpuMesh};

use crate::{
    asset::{texture::TextureAsset, Assets},
    event::{ChunkLoaded, ChunkUnloaded},
    game::Game,
};

use super::{utils::TextureArray, Resources};

mod mesher;

/// The chunk renderer. Responsible for
/// 1) Maintaining a mesh for each chunk to be rendered.
/// 2) Maintaining a texture array containing block textures.
/// 3) Rendering each visible chunk.
#[derive(Debug)]
pub struct ChunkRenderer {
    block_textures: TextureArray,
    /// Maps block slug => texture index into `block_textures`.
    block_texture_indexes: AHashMap<String, u32>,
    mesher: ChunkMesher,

    meshes: AHashMap<ChunkPos, GpuMesh>,
    pending_meshes: AHashSet<ChunkPos>,
}

impl ChunkRenderer {
    pub fn new(
        resources: &Arc<Resources>,
        assets: &Assets,
        encoder: &mut wgpu::CommandEncoder,
    ) -> anyhow::Result<Self> {
        let (block_textures, block_texture_indexes) =
            create_block_textures(resources, assets, encoder)
                .context("failed to create block texture array")?;
        let mesher = ChunkMesher::new(assets, resources, |texture_name| {
            block_texture_indexes.get(texture_name).copied()
        })
        .context("failed to initialize chunk mesher")?;

        Ok(Self {
            block_textures,
            block_texture_indexes,
            mesher,
            meshes: AHashMap::new(),
            pending_meshes: AHashSet::new(),
        })
    }

    pub fn prep_render(&mut self, game: &mut Game) {
        self.update_chunk_meshes(game);
    }

    fn update_chunk_meshes(&mut self, game: &mut Game) {
        for event in game.events().iter::<ChunkLoaded>() {
            if let Some(chunk) = game.main_zone().chunk(event.pos) {
                self.mesher.spawn(event.pos, chunk.clone());
                log::trace!("Spawning mesher task for {:?}", event.pos);
                self.pending_meshes.insert(event.pos);
            }
        }

        for event in game.events().iter::<ChunkUnloaded>() {
            self.meshes.remove(&event.pos);
            self.pending_meshes.remove(&event.pos);

            log::trace!("Dropping chunk mesh for {:?}", event.pos);
        }

        for (pos, finished_mesh) in self.mesher.iter_finished() {
            if self.pending_meshes.remove(&pos) {
                self.meshes.insert(pos, finished_mesh);
                log::trace!(
                    "Loaded mesh for {:?}. Total meshes: {}",
                    pos,
                    self.meshes.len()
                );
            }
        }
    }
}

/// A fixed dimension used for block textures. Block textures
/// must match this dimension exactly.
const BLOCK_TEXTURE_DIM: u32 = 64;

fn create_block_textures(
    resources: &Arc<Resources>,
    assets: &Assets,
    encoder: &mut wgpu::CommandEncoder,
) -> anyhow::Result<(TextureArray, AHashMap<String, u32>)> {
    let mut textures = TextureArray::new(
        wgpu::TextureDescriptor {
            label: Some("block_textures"),
            size: wgpu::Extent3d {
                width: BLOCK_TEXTURE_DIM,
                height: BLOCK_TEXTURE_DIM,
                depth: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsage::SAMPLED,
        },
        resources,
    );
    let mut indexes = AHashMap::new();

    let prefix = "texture/block/";
    for (name, texture) in assets.iter_prefixed::<TextureAsset>(prefix) {
        let name = name.strip_prefix(prefix).expect("prefix");

        if texture.width() != BLOCK_TEXTURE_DIM || texture.height() != BLOCK_TEXTURE_DIM {
            bail!(
                "texture '{}' has invalid width/height. required: {}x{}. found: {}x{}",
                name,
                BLOCK_TEXTURE_DIM,
                BLOCK_TEXTURE_DIM,
                texture.width(),
                texture.height()
            );
        }

        let data = texture.data();
        let index = textures.add(data, resources.queue(), encoder);
        indexes.insert(name.to_owned(), index);

        log::info!("Uploaded block texture '{}'", name);
    }

    Ok((textures, indexes))
}
