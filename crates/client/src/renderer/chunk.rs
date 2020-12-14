use std::{mem::size_of, sync::Arc};

use ahash::{AHashMap, AHashSet};
use anyhow::{bail, Context};
use common::{chunk::CHUNK_DIM, ChunkPos, Pos};
use glam::{vec4, Mat4, Vec4};
use mesher::{ChunkMesher, GpuMesh};

use crate::{
    asset::{shader::ShaderAsset, texture::TextureAsset, Assets},
    event::{ChunkLoaded, ChunkUnloaded},
    game::Game,
};

use self::{cull::Culler, mesher::RawVertex};

use super::{utils::TextureArray, Resources, DEPTH_FORMAT, SAMPLE_COUNT, SC_FORMAT};

mod cull;
mod mesher;

/// The chunk renderer. Responsible for
/// 1) Maintaining a mesh for each chunk to be rendered.
/// 2) Maintaining a texture array containing block textures.
/// 3) Rendering each visible chunk.
pub struct ChunkRenderer {
    block_textures: TextureArray,
    /// Maps block slug => texture index into `block_textures`.
    block_texture_indexes: AHashMap<String, u32>,

    block_sampler: wgpu::Sampler,

    mesher: ChunkMesher,
    culler: Culler,

    chunks: AHashMap<ChunkPos, GpuMesh>,
    pending_meshes: AHashSet<ChunkPos>,

    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
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

        let block_sampler = resources.device().create_sampler(&wgpu::SamplerDescriptor {
            label: Some("block_sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bg_layout =
            resources
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("chunk_bg_layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStage::FRAGMENT,
                            ty: wgpu::BindingType::SampledTexture {
                                dimension: wgpu::TextureViewDimension::D2Array,
                                component_type: wgpu::TextureComponentType::Float,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStage::FRAGMENT,
                            ty: wgpu::BindingType::Sampler { comparison: false },
                            count: None,
                        },
                    ],
                });

        let pipeline_layout =
            resources
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("chunk_pipeline_layout"),
                    bind_group_layouts: &[&bg_layout],
                    push_constant_ranges: &[wgpu::PushConstantRange {
                        stages: wgpu::ShaderStage::VERTEX,
                        range: 0..(size_of::<Mat4>() as u32 * 2 + size_of::<Vec4>() as u32),
                    }],
                });
        let vertex = resources.device().create_shader_module(
            assets
                .get::<ShaderAsset>("shader_compiled/chunk/vertex.spv")?
                .to_source(),
        );
        let fragment = resources.device().create_shader_module(
            assets
                .get::<ShaderAsset>("shader_compiled/chunk/fragment.spv")?
                .to_source(),
        );
        let pipeline = resources
            .device()
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("chunk_pipeline"),
                layout: Some(&pipeline_layout),
                vertex_stage: wgpu::ProgrammableStageDescriptor {
                    module: &vertex,
                    entry_point: "main",
                },
                fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                    module: &fragment,
                    entry_point: "main",
                }),
                rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: wgpu::CullMode::None,
                    ..Default::default()
                }),
                primitive_topology: wgpu::PrimitiveTopology::TriangleList,
                color_states: &[wgpu::ColorStateDescriptor {
                    format: SC_FORMAT,
                    color_blend: wgpu::BlendDescriptor::REPLACE,
                    alpha_blend: wgpu::BlendDescriptor::REPLACE,
                    write_mask: wgpu::ColorWrite::ALL,
                }],
                depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilStateDescriptor::default(),
                }),
                vertex_state: wgpu::VertexStateDescriptor {
                    index_format: wgpu::IndexFormat::Uint16,
                    vertex_buffers: &[wgpu::VertexBufferDescriptor {
                        stride: size_of::<RawVertex>() as _,
                        step_mode: wgpu::InputStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float3, 1 => Float3, 2 => Float3],
                    }],
                },
                sample_count: SAMPLE_COUNT,
                sample_mask: !0,
                alpha_to_coverage_enabled: false,
            });
        let bind_group = resources
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("chunk_bg"),
                layout: &bg_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(
                            &block_textures.get().create_view(&Default::default()),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&block_sampler),
                    },
                ],
            });

        Ok(Self {
            block_textures,
            block_texture_indexes,
            block_sampler,
            mesher,
            culler: Culler::new(),
            chunks: AHashMap::new(),
            pending_meshes: AHashSet::new(),
            pipeline,
            bind_group,
        })
    }

    pub fn prep_render(&mut self, resources: &Resources, game: &mut Game) {
        self.update_chunk_meshes(resources, game);
    }

    fn update_chunk_meshes(&mut self, _resources: &Resources, game: &mut Game) {
        for event in game.events().iter::<ChunkLoaded>() {
            if let Some(chunk) = game.main_zone().chunk(event.pos) {
                log::trace!("Spawning cull task for {:?}", event.pos);
                self.culler.on_chunk_loaded(event.pos, chunk);
                self.mesher.spawn(event.pos, chunk.clone());
                log::trace!("Spawning mesher task for {:?}", event.pos);
                self.pending_meshes.insert(event.pos);
            }
        }

        for event in game.events().iter::<ChunkUnloaded>() {
            self.chunks.remove(&event.pos);
            self.pending_meshes.remove(&event.pos);
            self.culler.on_chunk_unloaded(event.pos);

            log::trace!("Dropping chunk mesh for {:?}", event.pos);
        }

        for (pos, mesh) in self.mesher.iter_finished() {
            let was_pending = self.pending_meshes.remove(&pos);
            let mesh = match mesh {
                Some(mesh) => mesh,
                None => continue,
            };
            if was_pending {
                self.chunks.insert(pos, mesh);

                log::trace!(
                    "Loaded mesh for {:?}. Total chunks in renderer: {}",
                    pos,
                    self.chunks.len()
                );
            }
        }
    }

    pub fn do_render<'a>(&'a mut self, pass: &mut wgpu::RenderPass<'a>, game: &mut Game) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);

        let matrices = game.matrices();

        let pos = *game.player_ref().get::<Pos>().unwrap();
        let player_chunk = ChunkPos::from_pos(pos);

        #[cfg(debug_assertions)]
        let visible = {
            // Culling disabled in debug mode - it's too slow.
            self.chunks.keys().copied()
        };
        #[cfg(not(debug_assertions))]
        let visible = {
            self.culler.update(player_chunk, game.bump());
            self.culler.visible_chunks()
        };

        let mut count = 0;
        for pos in visible {
            let mesh = match self.chunks.get(&pos) {
                Some(m) => m,
                None => continue,
            };
            pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));

            #[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
            #[repr(C)]
            struct PushConstants {
                transform: Vec4,
                view: Mat4,
                projection: Mat4,
            }
            let transform = vec4(
                (pos.x * CHUNK_DIM as i32) as f32,
                (pos.y * CHUNK_DIM as i32) as f32,
                (pos.z * CHUNK_DIM as i32) as f32,
                0.,
            );
            let push_constants = PushConstants {
                transform,
                view: matrices.view,
                projection: matrices.projection,
            };
            pass.set_push_constants(
                wgpu::ShaderStage::VERTEX,
                0,
                bytemuck::cast_slice(&[push_constants]),
            );

            pass.draw(0..mesh.vertex_count, 0..1);
            count += 1;
        }
        game.debug_data.render_chunks = count;
    }
}

/// A fixed dimension used for block textures. Block textures
/// must match this dimension exactly.
const BLOCK_TEXTURE_DIM: u32 = 64;
const MIP_LEVELS: u32 = 6;

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
            mip_level_count: MIP_LEVELS,
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
        let index = textures.add_mipmapped(data, resources.queue(), encoder)?;
        indexes.insert(name.to_owned(), index);

        log::info!("Uploaded block texture '{}'", name);
    }

    Ok((textures, indexes))
}
