//! Generation of a 2D biome grid which defines a biome for each column of blocks.
//!
//! # Implementation
//! This biome generator is based on the "grow" technique pioneered by the Cuberite
//! project for generating Minecraft biomes. We operate on an array of integers, which
//! we can "zoom" to add detail, "smooth" to remove noise, and apply other operations
//! to map integers to biomes. The final result is an array of biomes.

use bytemuck::{Pod, Zeroable};
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;
use std::{mem::size_of, sync::Arc};

pub const BIOME_GRID_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R8Uint;
const INITIAL_GRID_SIZE: u32 = 16;

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct PushConstants {
    seed: u32,
    offset: [u32; 2],
}

pub struct BiomeBundle {
    bundle: SequenceBundle,
    push_constants: PushConstants,
}

impl BiomeBundle {
    pub fn output_size(&self) -> u32 {
        self.last_stage().output_dimensions
    }

    pub fn output_texture(&self) -> &wgpu::Texture {
        &self.last_stage().output_texture
    }

    fn last_stage(&self) -> &PreparedStage {
        self.bundle.stages.last().unwrap()
    }
}

pub struct BiomeGenerator {
    sequence: Sequence,
    pipelines: Pipelines,
}

impl BiomeGenerator {
    pub fn new(device: &wgpu::Device) -> Self {
        let pipelines = Pipelines::new(device);
        let sequence = Self::create_sequence(&pipelines);

        Self {
            sequence,
            pipelines,
        }
    }

    pub fn prepare<'a>(
        &'a self,
        device: &'a wgpu::Device,
        seed: u32,
        max_output_size: u32,
    ) -> BiomeBundle {
        let bundle =
            self.sequence
                .create_bundle(device, &self.pipelines.bg_layout, max_output_size);
        let push_constants = PushConstants {
            seed,
            offset: [0, 0],
        };
        BiomeBundle {
            bundle,
            push_constants,
        }
    }

    pub fn execute<'a>(
        &self,
        bundle: &'a BiomeBundle,
        pass: &mut wgpu::ComputePass<'a>,
        queue: &wgpu::Queue,
    ) {
        self.upload_initial_grid(
            bundle.push_constants.seed,
            queue,
            &bundle.bundle.input_texture,
        );
        for stage in &bundle.bundle.stages {
            pass.set_pipeline(&stage.pipeline);
            pass.set_push_constants(0, bytemuck::cast_slice(&[bundle.push_constants]));
            pass.set_bind_group(0, &stage.bind_group, &[]);
            let [x, y] = self.dispatch_size(stage.work_group_size, stage.output_dimensions);
            pass.dispatch(x, y, 1);
        }
    }

    fn dispatch_size(&self, work_group_size: [u32; 2], output_size: u32) -> [u32; 2] {
        [
            (output_size + work_group_size[0] - 1) / work_group_size[0],
            (output_size + work_group_size[1] - 1) / work_group_size[1],
        ]
    }

    fn create_sequence(pipelines: &Pipelines) -> Sequence {
        let mut encoder = SequenceEncoder::new(pipelines);

        encoder
            .push(Zoom)
            .push(Smooth)
            .push(Zoom)
            .push(Smooth)
            .push(Land)
            .push(Zoom)
            .push(Smooth)
            .push(Zoom)
            .push(Smooth)
            .push(Zoom)
            .push(Smooth)
            .push(Zoom)
            .push(Smooth)
            .push(Rivers)
            .push(Zoom)
            .push(Smooth)
            .push(Zoom)
            .push(Smooth)
            .push(Zoom)
            .push(Smooth)
            .push(Zoom)
            .push(Smooth);

        encoder.finish()
    }

    fn upload_initial_grid(&self, seed: u32, queue: &wgpu::Queue, texture: &wgpu::Texture) {
        let grid = self.generate_initial_grid(seed);
        queue.write_texture(
            wgpu::TextureCopyView {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &grid,
            wgpu::TextureDataLayout {
                offset: 0,
                bytes_per_row: INITIAL_GRID_SIZE,
                rows_per_image: INITIAL_GRID_SIZE,
            },
            wgpu::Extent3d {
                width: INITIAL_GRID_SIZE,
                height: INITIAL_GRID_SIZE,
                depth: 1,
            },
        );
    }

    fn generate_initial_grid<'a>(&self, seed: u32) -> Vec<u8> {
        let mut grid = vec![0u8; (INITIAL_GRID_SIZE * INITIAL_GRID_SIZE) as usize];
        let mut rng = Pcg64Mcg::seed_from_u64(seed as u64);
        for x in 0..INITIAL_GRID_SIZE {
            for y in 0..INITIAL_GRID_SIZE {
                let value = rng.gen::<bool>() as u8;
                grid[(y * INITIAL_GRID_SIZE + x) as usize] = value;
            }
        }

        grid
    }
}

struct Pipelines {
    zoom: Arc<wgpu::ComputePipeline>,
    smooth: Arc<wgpu::ComputePipeline>,
    land: Arc<wgpu::ComputePipeline>,
    rivers: Arc<wgpu::ComputePipeline>,
    bg_layout: wgpu::BindGroupLayout,
}

impl Pipelines {
    fn new(device: &wgpu::Device) -> Self {
        let bg_layout = Self::create_bg_layout(device);
        let zoom = Self::create_zoom_pipeline(device, &bg_layout);
        let smooth = Self::create_smooth_pipeline(device, &bg_layout);
        let land = Self::create_land_pipeline(device, &bg_layout);
        let rivers = Self::create_rivers_pipeline(device, &bg_layout);

        Self {
            zoom,
            smooth,
            land,
            rivers,
            bg_layout,
        }
    }

    fn create_bg_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("biome_bg_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        dimension: wgpu::TextureViewDimension::D2,
                        format: BIOME_GRID_FORMAT,
                        readonly: true,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        dimension: wgpu::TextureViewDimension::D2,
                        format: BIOME_GRID_FORMAT,
                        readonly: false,
                    },
                    count: None,
                },
            ],
        })
    }

    fn create_zoom_pipeline(
        device: &wgpu::Device,
        bg_layout: &wgpu::BindGroupLayout,
    ) -> Arc<wgpu::ComputePipeline> {
        Self::create_pipeline(
            device,
            bg_layout,
            wgpu::include_spirv!("../../../assets/shader/worldgen/biomegrid/zoom.spv"),
        )
    }

    fn create_smooth_pipeline(
        device: &wgpu::Device,
        bg_layout: &wgpu::BindGroupLayout,
    ) -> Arc<wgpu::ComputePipeline> {
        Self::create_pipeline(
            device,
            bg_layout,
            wgpu::include_spirv!("../../../assets/shader/worldgen/biomegrid/smooth.spv"),
        )
    }

    fn create_land_pipeline(
        device: &wgpu::Device,
        bg_layout: &wgpu::BindGroupLayout,
    ) -> Arc<wgpu::ComputePipeline> {
        Self::create_pipeline(
            device,
            bg_layout,
            wgpu::include_spirv!("../../../assets/shader/worldgen/biomegrid/land.spv"),
        )
    }

    fn create_rivers_pipeline(
        device: &wgpu::Device,
        bg_layout: &wgpu::BindGroupLayout,
    ) -> Arc<wgpu::ComputePipeline> {
        Self::create_pipeline(
            device,
            bg_layout,
            wgpu::include_spirv!("../../../assets/shader/worldgen/biomegrid/rivers.spv"),
        )
    }

    fn create_pipeline(
        device: &wgpu::Device,
        bg_layout: &wgpu::BindGroupLayout,
        shader_source: wgpu::ShaderModuleSource,
    ) -> Arc<wgpu::ComputePipeline> {
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[bg_layout],
            push_constant_ranges: &[wgpu::PushConstantRange {
                stages: wgpu::ShaderStage::COMPUTE,
                range: 0..size_of::<PushConstants>() as u32,
            }],
        });
        let module = device.create_shader_module(shader_source);
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&layout),
            compute_stage: wgpu::ProgrammableStageDescriptor {
                module: &module,
                entry_point: "main",
            },
        });
        Arc::new(pipeline)
    }
}

#[derive(Default)]
struct Sequence {
    stages: Vec<EncodedStage>,
}

impl Sequence {
    pub fn output_dimensions(&self) -> u32 {
        self.stages
            .last()
            .map(|stage| stage.output_dimensions)
            .unwrap_or(INITIAL_GRID_SIZE)
    }

    pub fn create_bundle<'a>(
        &'a self,
        device: &'a wgpu::Device,
        bg_layout: &'a wgpu::BindGroupLayout,
        max_size: u32,
    ) -> SequenceBundle {
        SequenceBundleEncoder::new(self, device, bg_layout, max_size).encode()
    }
}

struct EncodedStage {
    pipeline: Arc<wgpu::ComputePipeline>,
    output_dimensions: u32,
    work_group_size: [u32; 2],
}

trait Stage {
    fn output_dimensions(&self, input_dimensions: u32) -> u32;

    fn work_group_size(&self) -> [u32; 2];

    fn pipeline<'a>(&self, pipelines: &'a Pipelines) -> &'a Arc<wgpu::ComputePipeline>;
}

struct Zoom;

impl Stage for Zoom {
    fn output_dimensions(&self, input_dimensions: u32) -> u32 {
        input_dimensions * 2 - 1
    }

    fn work_group_size(&self) -> [u32; 2] {
        [31; 2]
    }

    fn pipeline<'a>(&self, pipelines: &'a Pipelines) -> &'a Arc<wgpu::ComputePipeline> {
        &pipelines.zoom
    }
}

struct Smooth;

impl Stage for Smooth {
    fn output_dimensions(&self, input_dimensions: u32) -> u32 {
        input_dimensions - 2
    }

    fn work_group_size(&self) -> [u32; 2] {
        [16; 2]
    }

    fn pipeline<'a>(&self, pipelines: &'a Pipelines) -> &'a Arc<wgpu::ComputePipeline> {
        &pipelines.smooth
    }
}

struct Land;

impl Stage for Land {
    fn output_dimensions(&self, input_dimensions: u32) -> u32 {
        input_dimensions
    }

    fn work_group_size(&self) -> [u32; 2] {
        [32; 2]
    }

    fn pipeline<'a>(&self, pipelines: &'a Pipelines) -> &'a Arc<wgpu::ComputePipeline> {
        &pipelines.land
    }
}

struct Rivers;

impl Stage for Rivers {
    fn output_dimensions(&self, input_dimensions: u32) -> u32 {
        input_dimensions - 2
    }

    fn work_group_size(&self) -> [u32; 2] {
        [16; 2]
    }

    fn pipeline<'a>(&self, pipelines: &'a Pipelines) -> &'a Arc<wgpu::ComputePipeline> {
        &pipelines.rivers
    }
}

struct SequenceEncoder<'a> {
    pipelines: &'a Pipelines,
    sequence: Sequence,
}

impl<'a> SequenceEncoder<'a> {
    pub fn new(pipelines: &'a Pipelines) -> Self {
        Self {
            pipelines,
            sequence: Default::default(),
        }
    }

    pub fn push(&mut self, stage: impl Stage) -> &mut Self {
        let input_dimensions = self.sequence.output_dimensions();
        let output_dimensions = stage.output_dimensions(input_dimensions);
        let pipeline = Arc::clone(stage.pipeline(&self.pipelines));
        let work_group_size = stage.work_group_size();

        self.sequence.stages.push(EncodedStage {
            output_dimensions,
            pipeline,
            work_group_size,
        });

        self
    }

    pub fn finish(self) -> Sequence {
        self.sequence
    }
}

struct PreparedStage {
    pipeline: Arc<wgpu::ComputePipeline>,
    output_dimensions: u32,
    work_group_size: [u32; 2],
    output_texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
}

struct SequenceBundle {
    input_texture: wgpu::Texture,
    stages: Vec<PreparedStage>,
}

struct SequenceBundleEncoder<'a> {
    sequence: &'a Sequence,
    bundle: SequenceBundle,
    device: &'a wgpu::Device,
    bg_layout: &'a wgpu::BindGroupLayout,
    max_dimensions: u32,
}

impl<'a> SequenceBundleEncoder<'a> {
    pub fn new(
        sequence: &'a Sequence,
        device: &'a wgpu::Device,
        bg_layout: &'a wgpu::BindGroupLayout,
        max_dimensions: u32,
    ) -> Self {
        let input_texture = Self::create_input_texture(device);
        Self {
            sequence,
            bundle: SequenceBundle {
                input_texture,
                stages: Vec::new(),
            },
            device,
            bg_layout,
            max_dimensions,
        }
    }

    fn create_input_texture(device: &wgpu::Device) -> wgpu::Texture {
        let mut desc = texture_descriptor(INITIAL_GRID_SIZE);
        desc.usage |= wgpu::TextureUsage::COPY_DST;
        device.create_texture(&desc)
    }

    pub fn encode(mut self) -> SequenceBundle {
        self.prepare_stages();

        self.bundle
    }

    fn prepare_stages(&mut self) {
        for stage in &self.sequence.stages {
            self.prepare_stage(stage);
        }
    }

    fn prepare_stage(&mut self, stage: &'a EncodedStage) {
        let output_dimensions = self.max_dimensions.min(stage.output_dimensions);
        let output_texture = self.create_output_texture(output_dimensions);
        let bind_group = self.create_stage_bind_group(&output_texture);
        self.bundle.stages.push(PreparedStage {
            output_texture,
            bind_group,
            pipeline: Arc::clone(&stage.pipeline),
            output_dimensions,
            work_group_size: stage.work_group_size,
        });
    }

    fn create_stage_bind_group(&self, output_texture: &wgpu::Texture) -> wgpu::BindGroup {
        let input_texture = self.input_texture();

        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.bg_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&default_view(input_texture)),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&default_view(output_texture)),
                },
            ],
        })
    }

    fn create_output_texture(&self, size: u32) -> wgpu::Texture {
        let desc = texture_descriptor(size);
        self.device.create_texture(&desc)
    }

    fn input_texture(&self) -> &wgpu::Texture {
        self.bundle
            .stages
            .last()
            .map(|stage| &stage.output_texture)
            .unwrap_or(&self.bundle.input_texture)
    }
}

fn texture_descriptor(size: u32) -> wgpu::TextureDescriptor<'static> {
    wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: size,
            height: size,
            depth: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: BIOME_GRID_FORMAT,
        usage: wgpu::TextureUsage::COPY_SRC | wgpu::TextureUsage::STORAGE,
    }
}

fn default_view(texture: &wgpu::Texture) -> wgpu::TextureView {
    texture.create_view(&Default::default())
}
