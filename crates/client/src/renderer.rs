use std::sync::Arc;

use anyhow::{anyhow, Context};
use futures_executor::block_on;
use sdl2::video::Window;

use crate::asset::Assets;

mod chunk;
mod utils;

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

pub struct Renderer {
    resources: Arc<Resources>,
    // chunk_renderer: ChunkRenderer,
}

impl Renderer {
    pub fn new(window: &Window, _assets: &Assets) -> anyhow::Result<Self> {
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        log::info!(
            "Available adapters: {:?}",
            instance
                .enumerate_adapters(wgpu::BackendBit::PRIMARY)
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
        log::info!("Selected adapter: {:?}", adapter);

        let (device, queue) = block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::default(),
                limits: wgpu::Limits::default(),
                shader_validation: true,
            },
            None,
        ))
        .context("failed to create device")?;

        let resources = Resources {
            adapter,
            device,
            queue,
            surface,
        };

        Ok(Self {
            resources: Arc::new(resources),
        })
    }
}
