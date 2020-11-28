use std::{mem::size_of, sync::Arc};

use ahash::{AHashMap, AHashSet};
use anyhow::{bail, Context};
use common::{chunk::CHUNK_DIM, ChunkPos, Pos};
use glam::{Mat4, Vec3};
use mesher::{ChunkMesher, GpuMesh};
use wgpu::util::DeviceExt;

use crate::{
    asset::{shader::ShaderAsset, texture::TextureAsset, Assets},
    event::{ChunkLoaded, ChunkUnloaded},
    game::Game,
};

use self::mesher::RawVertex;

use super::{utils::TextureArray, Resources, SC_FORMAT};

mod mesher;

#[derive(Debug)]
struct ChunkRenderData {
    mesh: GpuMesh,
    bind_group: wgpu::BindGroup,
    transform: wgpu::Buffer,
}

/// The chunk renderer. Responsible for
/// 1) Maintaining a mesh for each chunk to be rendered.
/// 2) Maintaining a texture array containing block textures.
/// 3) Rendering each visible chunk.
#[derive(Debug)]
pub struct ChunkRenderer {
    block_textures: TextureArray,
    /// Maps block slug => texture index into `block_textures`.
    block_texture_indexes: AHashMap<String, u32>,

    block_sampler: wgpu::Sampler,

    mesher: ChunkMesher,

    chunks: AHashMap<ChunkPos, ChunkRenderData>,
    pending_meshes: AHashSet<ChunkPos>,

    pipeline: wgpu::RenderPipeline,
    bg_layout: wgpu::BindGroupLayout,
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
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            lod_min_clamp: 0.,
            lod_max_clamp: 100.,
            compare: None,
            anisotropy_clamp: None,
        });

        let bg_layout =
            resources
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("chunk_bg_layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStage::VERTEX,
                            ty: wgpu::BindingType::UniformBuffer {
                                dynamic: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStage::FRAGMENT,
                            ty: wgpu::BindingType::SampledTexture {
                                dimension: wgpu::TextureViewDimension::D2Array,
                                component_type: wgpu::TextureComponentType::Float,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
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
                        range: 0..size_of::<Mat4>() as u32,
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
                    cull_mode: wgpu::CullMode::Back,
                    ..Default::default()
                }),
                primitive_topology: wgpu::PrimitiveTopology::TriangleList,
                color_states: &[wgpu::ColorStateDescriptor {
                    format: SC_FORMAT,
                    color_blend: wgpu::BlendDescriptor::REPLACE,
                    alpha_blend: wgpu::BlendDescriptor::REPLACE,
                    write_mask: wgpu::ColorWrite::ALL,
                }],
                depth_stencil_state: None,
                vertex_state: wgpu::VertexStateDescriptor {
                    index_format: wgpu::IndexFormat::Uint16,
                    vertex_buffers: &[wgpu::VertexBufferDescriptor {
                        stride: size_of::<RawVertex>() as _,
                        step_mode: wgpu::InputStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float3, 1 => Float3],
                    }],
                },
                sample_count: 1,
                sample_mask: !0,
                alpha_to_coverage_enabled: false,
            });

        Ok(Self {
            block_textures,
            block_texture_indexes,
            block_sampler,
            mesher,
            chunks: AHashMap::new(),
            pending_meshes: AHashSet::new(),
            bg_layout,
            pipeline,
        })
    }

    pub fn prep_render(&mut self, resources: &Resources, game: &mut Game) {
        self.update_chunk_meshes(resources, game);
    }

    fn update_chunk_meshes(&mut self, resources: &Resources, game: &mut Game) {
        for event in game.events().iter::<ChunkLoaded>() {
            if let Some(chunk) = game.main_zone().chunk(event.pos) {
                self.mesher.spawn(event.pos, chunk.clone());
                log::trace!("Spawning mesher task for {:?}", event.pos);
                self.pending_meshes.insert(event.pos);
            }
        }

        for event in game.events().iter::<ChunkUnloaded>() {
            self.chunks.remove(&event.pos);
            self.pending_meshes.remove(&event.pos);

            log::trace!("Dropping chunk mesh for {:?}", event.pos);
        }

        for (pos, mesh) in self.mesher.iter_finished() {
            if self.pending_meshes.remove(&pos) {
                // Create chunk render data.
                let transform = Mat4::from_translation(glam::vec3(
                    (pos.x * CHUNK_DIM as i32) as f32,
                    (pos.y * CHUNK_DIM as i32) as f32,
                    (pos.z * CHUNK_DIM as i32) as f32,
                ));
                let transform =
                    resources
                        .device()
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some(&format!("chunk_transform_{},{},{}", pos.x, pos.y, pos.z)),
                            contents: bytemuck::cast_slice(&[transform]),
                            usage: wgpu::BufferUsage::UNIFORM,
                        });
                let bind_group = resources
                    .device()
                    .create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some(&format!("chunk_bg_{},{},{}", pos.x, pos.y, pos.z)),
                        layout: &self.bg_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::Buffer(transform.slice(..)),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::TextureView(
                                    &self.block_textures.get().create_view(
                                        &wgpu::TextureViewDescriptor {
                                            label: None,
                                            format: Some(wgpu::TextureFormat::Bgra8UnormSrgb),
                                            dimension: Some(wgpu::TextureViewDimension::D2Array),
                                            aspect: wgpu::TextureAspect::All,
                                            base_mip_level: 0,
                                            level_count: None,
                                            base_array_layer: 0,
                                            array_layer_count: None,
                                        },
                                    ),
                                ),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: wgpu::BindingResource::Sampler(&self.block_sampler),
                            },
                        ],
                    });
                let data = ChunkRenderData {
                    mesh,
                    bind_group,
                    transform,
                };
                self.chunks.insert(pos, data);

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

        const EYE_HEIGHT: f32 = 1.7;

        // Determine view and projection matrix.
        let player = game.player_ref();
        let pos = player.get::<Pos>().unwrap().0;
        let view = Mat4::look_at_lh(
            Vec3::from(pos) + glam::vec3(0., EYE_HEIGHT, 0.),
            glam::vec3(10., 101., 10.),
            Vec3::unit_y(),
        );
        let projection = Mat4::perspective_lh(70.0f32.to_radians(), 16. / 9., 0.1, 1000.);
        let view_projection = projection * view;

        // Render each chunk.
        for chunk_data in self.chunks.values() {
            pass.set_bind_group(0, &chunk_data.bind_group, &[]);
            pass.set_vertex_buffer(0, chunk_data.mesh.vertex_buffer.slice(..));
            pass.set_push_constants(
                wgpu::ShaderStage::VERTEX,
                0,
                bytemuck::cast_slice(&[view_projection]),
            );
            pass.draw(0..chunk_data.mesh.vertex_count, 0..1);
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
