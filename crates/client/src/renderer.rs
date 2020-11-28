use std::sync::Arc;

use anyhow::{anyhow, Context};
use futures_executor::block_on;
use sdl2::video::Window;

use crate::{asset::Assets, game::Game};

use self::chunk::ChunkRenderer;

mod chunk;
mod utils;

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
                features: wgpu::Features::default(),
                limits: wgpu::Limits::default(),
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

        let mut init_encoder =
            resources
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("init_encoder"),
                });

        let chunk_renderer = ChunkRenderer::new(&resources, assets, &mut init_encoder)
            .context("failed to initialize chunk renderer")?;

        Ok(Self {
            resources,
            chunk_renderer,
        })
    }

    /// Renders a frame.
    pub fn render(&mut self, game: &mut Game) {
        self.prep_render(game);
        self.do_render(game);
    }

    fn prep_render(&mut self, game: &mut Game) {
        self.chunk_renderer.prep_render(game);
    }

    fn do_render(&mut self, game: &mut Game) {}
}
