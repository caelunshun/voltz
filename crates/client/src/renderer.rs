use std::sync::Arc;

use anyhow::{anyhow, Context};
use futures_executor::block_on;
use sdl2::video::Window;

use crate::{asset::Assets, game::Game};

use self::chunk::ChunkRenderer;

mod chunk;
mod utils;

const SC_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;

#[derive(Debug)]
pub struct Resources {
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface,
}

impl Resources {
    pub fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn surface(&self) -> &wgpu::Surface {
        &self.surface
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
}

#[derive(Debug)]
pub struct Renderer {
    resources: Arc<Resources>,
    chunk_renderer: ChunkRenderer,
    swap_chain: wgpu::SwapChain,
}

impl Renderer {
    pub fn new(window: &Window, assets: &Assets) -> anyhow::Result<Self> {
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        log::info!(
            "Available adapters: {:#?}",
            instance
                .enumerate_adapters(wgpu::BackendBit::PRIMARY)
                .map(|adapter| adapter.get_info())
                .collect::<Vec<_>>()
        );
        let surface = block_on(async {
            // SAFETY: a wgpu surface can be created with an SDL2 window.
            unsafe { instance.create_surface(window) }
        });
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
        }))
        .ok_or_else(|| anyhow!("failed to select a suitable adapter"))?;
        log::info!("Selected adapter: {:#?}", adapter.get_info());

        let (device, queue) = block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::PUSH_CONSTANTS,
                limits: wgpu::Limits {
                    max_push_constant_size: 128,
                    ..Default::default()
                },
                shader_validation: true,
            },
            None,
        ))
        .context("failed to create device")?;

        log::info!("Device limits: {:#?}", device.limits());

        let resources = Arc::new(Resources {
            adapter,
            device,
            queue,
            surface,
        });

        let (width, height) = window.size();
        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: SC_FORMAT,
            width,
            height,
            present_mode: wgpu::PresentMode::Mailbox,
        };
        let swap_chain = resources
            .device()
            .create_swap_chain(resources.surface(), &sc_desc);

        let mut init_encoder =
            resources
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("init_encoder"),
                });

        let chunk_renderer = ChunkRenderer::new(&resources, assets, &mut init_encoder)
            .context("failed to initialize chunk renderer")?;

        resources.queue().submit(vec![init_encoder.finish()]);

        Ok(Self {
            resources,
            chunk_renderer,
            swap_chain,
        })
    }

    /// Renders a frame.
    pub fn render(&mut self, game: &mut Game) {
        self.prep_render(game);
        self.do_render(game);
    }

    fn prep_render(&mut self, game: &mut Game) {
        self.chunk_renderer.prep_render(&self.resources, game);
    }

    fn do_render(&mut self, game: &mut Game) {
        let mut encoder =
            self.resources
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("render_frame"),
                });
        let frame = self
            .swap_chain
            .get_current_frame()
            .expect("failed to get next output frame");

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.output.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.4,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });

            self.chunk_renderer.do_render(&mut pass, game);
        }

        self.resources.queue().submit(vec![encoder.finish()]);
    }
}
