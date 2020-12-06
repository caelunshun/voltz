use std::mem::size_of;

use ahash::AHashMap;
use glam::{vec2, Mat4, Vec2};
use utils::Color;
use voltzui::Canvas;

use crate::{
    asset::{shader::ShaderAsset, Assets},
    game::Game,
};

use super::{Resources, SC_FORMAT};

#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
#[repr(C)]
struct PushConstants {
    ortho: Mat4,
    pos: Vec2,
    size: Vec2,
}

struct Bundle {
    push_constants: PushConstants,
    bind_group: wgpu::BindGroup,
}

/// Renderer which blits rendered `voltzui::Ui` canvases
/// to the present surface.
pub struct UiRenderer {
    pipeline: wgpu::RenderPipeline,
    bg_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    canvas_cache: AHashMap<(u32, u32), Canvas>,
    /// Cached for current frame.
    bundles: Vec<Bundle>,
}

impl UiRenderer {
    pub fn new(resources: &Resources, assets: &Assets) -> anyhow::Result<Self> {
        let bg_layout =
            resources
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("ui_sampler_and_texture"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            ty: wgpu::BindingType::SampledTexture {
                                dimension: wgpu::TextureViewDimension::D2,
                                component_type: wgpu::TextureComponentType::Float,
                                multisampled: false,
                            },
                            count: None,
                            visibility: wgpu::ShaderStage::FRAGMENT,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            ty: wgpu::BindingType::Sampler { comparison: false },
                            count: None,
                            visibility: wgpu::ShaderStage::FRAGMENT,
                        },
                    ],
                });

        let vertex_stage = assets
            .get::<ShaderAsset>("shader_compiled/blit/vertex.spv")?
            .to_source();
        let fragment_stage = assets
            .get::<ShaderAsset>("shader_compiled/blit/fragment.spv")?
            .to_source();

        let vertex_stage = resources.device().create_shader_module(vertex_stage);
        let fragment_stage = resources.device().create_shader_module(fragment_stage);

        let pipeline_layout =
            resources
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("ui_blit"),
                    bind_group_layouts: &[&bg_layout],
                    push_constant_ranges: &[wgpu::PushConstantRange {
                        stages: wgpu::ShaderStage::VERTEX,
                        range: 0..(size_of::<Vec2>() * 2 + size_of::<Mat4>()) as u32,
                    }],
                });
        let pipeline = resources
            .device()
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("ui_blit"),
                layout: Some(&pipeline_layout),
                vertex_stage: wgpu::ProgrammableStageDescriptor {
                    module: &vertex_stage,
                    entry_point: "main",
                },
                fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                    module: &fragment_stage,
                    entry_point: "main",
                }),
                rasterization_state: Some(wgpu::RasterizationStateDescriptor::default()),
                primitive_topology: wgpu::PrimitiveTopology::TriangleList,
                color_states: &[wgpu::ColorStateDescriptor {
                    format: SC_FORMAT,
                    alpha_blend: wgpu::BlendDescriptor::REPLACE,
                    color_blend: wgpu::BlendDescriptor {
                        operation: wgpu::BlendOperation::Add,
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    },
                    write_mask: wgpu::ColorWrite::ALL,
                }],
                depth_stencil_state: None,
                vertex_state: wgpu::VertexStateDescriptor {
                    index_format: wgpu::IndexFormat::Uint16,
                    vertex_buffers: &[],
                },
                sample_count: 1,
                sample_mask: !0,
                alpha_to_coverage_enabled: false,
            });

        let sampler = resources.device().create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ui_blit_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 0.,
            lod_max_clamp: 100.,
            compare: None,
            anisotropy_clamp: None,
        });

        Ok(Self {
            bg_layout,
            pipeline,
            sampler,
            canvas_cache: AHashMap::new(),
            bundles: Vec::new(),
        })
    }

    pub fn prep_render(&mut self, resources: &Resources, game: &mut Game) {
        let size = game.window().inner_size();
        let ortho = Mat4::orthographic_lh(0., size.width as f32, size.height as f32, 0., 0., 1.);

        let mut uis = Vec::new_in(game.bump());
        let mut store = game.ui_store();
        store.finish_frame(&mut uis);

        self.bundles.clear();
        for ui in uis {
            let width = ui.width.resolve(size.width as f32) as u32;
            let height = ui.height.resolve(size.height as f32) as u32;
            let canvas = self
                .canvas_cache
                .entry((width, height))
                .or_insert_with(|| Canvas::new(width, height, 1.));

            canvas.clear(Color::rgba(0., 0., 0., 0.));
            ui.ui.render(canvas);

            let size = wgpu::Extent3d {
                width: canvas.pixel_width(),
                height: canvas.pixel_height(),
                depth: 1,
            };
            let texture = resources.device().create_texture(&wgpu::TextureDescriptor {
                label: None,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
            });
            resources.queue().write_texture(
                wgpu::TextureCopyView {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                },
                canvas.data(),
                wgpu::TextureDataLayout {
                    offset: 0,
                    bytes_per_row: 4 * canvas.pixel_width(),
                    rows_per_image: canvas.pixel_height(),
                },
                size,
            );

            let bind_group = resources
                .device()
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &self.bg_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(
                                &texture.create_view(&Default::default()),
                            ),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                    ],
                });

            let bundle = Bundle {
                push_constants: PushConstants {
                    ortho,
                    pos: ui.pos,
                    size: vec2(canvas.width(), canvas.height()),
                },
                bind_group,
            };
            self.bundles.push(bundle);
        }
    }

    pub fn do_render<'a>(&'a mut self, pass: &mut wgpu::RenderPass<'a>) {
        pass.set_pipeline(&self.pipeline);

        for bundle in &self.bundles {
            pass.set_bind_group(0, &bundle.bind_group, &[]);
            pass.set_push_constants(
                wgpu::ShaderStage::VERTEX,
                0,
                bytemuck::cast_slice(&[bundle.push_constants]),
            );
            pass.draw(0..6, 0..1);
        }
    }
}
