use std::sync::Arc;

use chunk::ChunkRenderer;

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
    chunk_renderer: ChunkRenderer,
}
