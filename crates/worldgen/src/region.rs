//! Generates regions of blocks on the GPU.
//! Regions are cubs of blocks with length [`REGION_DIM`].

use std::iter;

use common::{blocks, chunk::CHUNK_DIM, BlockId, Chunk};
use once_cell::sync::Lazy;

use crate::biomes::BIOME_GRID_FORMAT;

pub const REGION_CHUNKS: usize = 16;
pub const REGION_DIM: usize = CHUNK_DIM * REGION_CHUNKS; // 256

const BLOCK_BUFFER_SIZE: u64 = (REGION_DIM * REGION_DIM * REGION_DIM) as u64;

#[derive(Default)]
pub struct Region {
    // box is needed or we get a stack overflow
    pub chunks: Box<[[[Chunk; REGION_CHUNKS]; REGION_CHUNKS]; REGION_CHUNKS]>,
}

static BLOCK_LUT: Lazy<Vec<BlockId>> = Lazy::new(|| {
    // Needs to match block definitions in shader/include/blocks.glsl
    vec![
        BlockId::new(blocks::Air),
        BlockId::new(blocks::Stone),
        BlockId::new(blocks::Dirt),
        BlockId::new(blocks::Grass),
        BlockId::new(blocks::Sand),
        BlockId::new(blocks::Melium),
        BlockId::new(blocks::Water),
    ]
});

impl Region {
    pub fn from_gpu_data(data: &[u8]) -> Self {
        let mut region = Region::default();

        let lut = BLOCK_LUT.as_slice();

        for chunk_x in 0..REGION_CHUNKS {
            for chunk_z in 0..REGION_CHUNKS {
                for x in 0..CHUNK_DIM {
                    for z in 0..CHUNK_DIM {
                        for y in 0..REGION_DIM {
                            let x = chunk_x * CHUNK_DIM + x;
                            let z = chunk_z * CHUNK_DIM + z;
                            let chunk_y = y / CHUNK_DIM;

                            let block_index =
                                data[x * REGION_DIM * REGION_DIM + z * REGION_DIM + y];
                            let block = lut[block_index as usize];
                            region.chunks[chunk_x][chunk_y][chunk_z].set(
                                x % CHUNK_DIM,
                                y % CHUNK_DIM,
                                z % CHUNK_DIM,
                                block,
                            );
                        }
                    }
                }
            }
        }

        region
    }
}

pub struct ComputePayload {
    bind_group: wgpu::BindGroup,
    block_buffer: wgpu::Buffer,
}

pub struct RegionGenerator {
    pipeline: wgpu::ComputePipeline,
    bg_layout: wgpu::BindGroupLayout,
}

impl RegionGenerator {
    pub fn new(device: &wgpu::Device) -> Self {
        let bg_layout = Self::create_bg_layout(device);
        let pipeline = Self::create_pipeline(device, &bg_layout);

        Self {
            bg_layout,
            pipeline,
        }
    }

    pub fn prepare(&self, device: &wgpu::Device, biome_grid: &wgpu::Texture) -> ComputePayload {
        let block_buffer = self.create_block_buffer(device);
        let bind_group = self.create_bind_group(device, &block_buffer, biome_grid);
        ComputePayload {
            block_buffer,
            bind_group,
        }
    }

    pub fn execute<'a>(&'a self, payload: &'a ComputePayload, pass: &mut wgpu::ComputePass<'a>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &payload.bind_group, &[]);
        pass.dispatch(REGION_DIM as u32, 1, REGION_DIM as u32);
    }

    pub async fn load_region_from_gpu(
        &self,
        payload: &ComputePayload,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        mut encoder: wgpu::CommandEncoder,
    ) -> Region {
        // We need to copy the block_buffer to a temporary buffer with
        // MAP_READ usage.
        let temp_buffer = self.create_mappable_temp_buffer(device);
        encoder.copy_buffer_to_buffer(&payload.block_buffer, 0, &temp_buffer, 0, BLOCK_BUFFER_SIZE);
        queue.submit(iter::once(encoder.finish()));

        let block_buffer = temp_buffer.slice(..);
        block_buffer
            .map_async(wgpu::MapMode::Read)
            .await
            .expect("failed to map block buffer");

        let data = block_buffer.get_mapped_range();
        let region = Region::from_gpu_data(&data);
        region
    }

    fn create_bg_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                // uBlocks
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        min_binding_size: None,
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                    },
                    count: None,
                },
                // uBiomeGrid
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        view_dimension: wgpu::TextureViewDimension::D2,
                        format: BIOME_GRID_FORMAT,
                        access: wgpu::StorageTextureAccess::ReadOnly,
                    },
                    count: None,
                },
            ],
        })
    }

    fn create_pipeline(
        device: &wgpu::Device,
        bg_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::ComputePipeline {
        let layout = Self::create_pipeline_layout(device, bg_layout);
        let module = device.create_shader_module(&wgpu::include_spirv!(
            "../../../assets/shader/worldgen/region/region.spv"
        ));
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&layout),
            compute_stage: wgpu::ProgrammableStageDescriptor {
                module: &module,
                entry_point: "main",
            },
        })
    }

    fn create_pipeline_layout(
        device: &wgpu::Device,
        bg_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::PipelineLayout {
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[bg_layout],
            push_constant_ranges: &[],
        })
    }

    fn create_block_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: BLOCK_BUFFER_SIZE,
            usage: wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::COPY_SRC,
            mapped_at_creation: false,
        })
    }

    fn create_bind_group(
        &self,
        device: &wgpu::Device,
        block_buffer: &wgpu::Buffer,
        biome_grid: &wgpu::Texture,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.bg_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: block_buffer,
                        offset: 0,
                        size: None,
                    },
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &biome_grid.create_view(&Default::default()),
                    ),
                },
            ],
        })
    }

    fn create_mappable_temp_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: BLOCK_BUFFER_SIZE,
            usage: wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::MAP_READ,
            mapped_at_creation: false,
        })
    }
}
