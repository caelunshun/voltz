use std::sync::Arc;

use crate::renderer::Resources;

pub type Index = u32;

/// Maintains a dynamic 2D texture array. Textures can
/// be added and removed on demand, and indexes into the array
/// are stable.
///
/// Each texture in the array must have the same size and format.
#[derive(Debug)]
pub struct TextureArray {
    texture: wgpu::Texture,
    desc: wgpu::TextureDescriptor<'static>,
    resources: Arc<Resources>,
    free: Vec<Index>,
}

const START_CAPACITY: u32 = 4;
const GROW_FACTOR: u32 = 2;

impl TextureArray {
    pub fn new(mut desc: wgpu::TextureDescriptor<'static>, resources: &Arc<Resources>) -> Self {
        let resources = Arc::clone(resources);

        assert_eq!(desc.dimension, wgpu::TextureDimension::D2);
        assert_eq!(desc.format, wgpu::TextureFormat::Bgra8UnormSrgb);
        assert_eq!(desc.size.depth, 1);

        desc.size.depth = START_CAPACITY;
        desc.usage |= wgpu::TextureUsage::COPY_SRC | wgpu::TextureUsage::COPY_DST;

        let texture = resources.device().create_texture(&desc);

        Self {
            resources,
            texture,
            desc,
            free: (0..START_CAPACITY).rev().collect(),
        }
    }

    /// Adds a texture to the array, returning its index.
    ///
    /// Resizes the internal array if necessary.
    pub fn add(
        &mut self,
        texture: &[u8],
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Index {
        let index = self.allocate_index(encoder);
        self.upload_texture(texture, queue, index);
        index
    }

    fn upload_texture(&self, texture: &[u8], queue: &wgpu::Queue, index: Index) {
        queue.write_texture(
            wgpu::TextureCopyView {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: index,
                },
            },
            texture,
            wgpu::TextureDataLayout {
                offset: 0,
                bytes_per_row: self.desc.size.width * 4,
                rows_per_image: self.desc.size.height,
            },
            wgpu::Extent3d {
                depth: 1,
                ..self.desc.size
            },
        );
    }

    /// Gets the internal texture array.
    pub fn get(&self) -> &wgpu::Texture {
        &self.texture
    }

    fn allocate_index(&mut self, encoder: &mut wgpu::CommandEncoder) -> Index {
        if let Some(index) = self.free.pop() {
            index
        } else {
            self.grow(encoder);
            self.free.pop().expect("grow() should create free indexes")
        }
    }

    pub fn capacity(&self) -> u32 {
        self.desc.size.depth
    }

    fn set_capacity(&mut self, cap: u32) {
        self.desc.size.depth = cap;
    }

    fn grow(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let old_cap = self.capacity();
        let old_size = self.desc.size;
        let new_cap = old_cap
            .checked_mul(GROW_FACTOR)
            .expect("texture array overflow");
        self.set_capacity(new_cap);

        let new_texture = self.resources.device().create_texture(&self.desc);
        encoder.copy_texture_to_texture(
            wgpu::TextureCopyView {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::TextureCopyView {
                texture: &new_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            old_size,
        );
        self.texture = new_texture;

        self.free.extend(old_cap..new_cap);
    }
}
