use super::{DEPTH_FORMAT, SAMPLE_COUNT, SC_FORMAT};

#[derive(Debug)]
pub struct Presenter {
    sc_desc: wgpu::SwapChainDescriptor,
    sc: wgpu::SwapChain,
    sample_buffer: wgpu::Texture,
    sample_buffer_view: wgpu::TextureView,
    depth_buffer: wgpu::Texture,
    depth_buffer_view: wgpu::TextureView,
}

impl Presenter {
    pub fn new(device: &wgpu::Device, surface: &wgpu::Surface, width: u32, height: u32) -> Self {
        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: SC_FORMAT,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        let sc = device.create_swap_chain(&surface, &sc_desc);

        let sample_buffer = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sample_texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth: 1,
            },
            mip_level_count: 1,
            sample_count: SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format: SC_FORMAT,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::SAMPLED,
        });
        let sample_buffer_view = sample_buffer.create_view(&Default::default());

        let depth_buffer = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth_texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth: 1,
            },
            mip_level_count: 1,
            sample_count: SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::SAMPLED,
        });
        let depth_buffer_view = depth_buffer.create_view(&Default::default());

        Self {
            sc_desc,
            sc,
            sample_buffer,
            sample_buffer_view,
            depth_buffer,
            depth_buffer_view,
        }
    }

    pub fn width(&self) -> u32 {
        self.sc_desc.width
    }

    pub fn height(&self) -> u32 {
        self.sc_desc.height
    }

    pub fn swapchain(&mut self) -> &mut wgpu::SwapChain {
        &mut self.sc
    }

    pub fn sample_buffer(&self) -> &wgpu::TextureView {
        &self.sample_buffer_view
    }

    pub fn depth_buffer(&self) -> &wgpu::TextureView {
        &self.depth_buffer_view
    }
}
