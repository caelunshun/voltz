use std::{sync::Arc, thread};

use anyhow::Context;
use futures_executor::block_on;

pub fn init(
    instance: wgpu::Instance,
    compatible_surface: Option<&wgpu::Surface>,
) -> anyhow::Result<(wgpu::Device, wgpu::Queue, wgpu::Adapter)> {
    let backends = wgpu::BackendBit::PRIMARY;
    log::info!(
        "Available adapters: {:#?}",
        instance
            .enumerate_adapters(backends)
            .map(|adapter| adapter.get_info())
            .collect::<Vec<_>>()
    );

    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface,
    }))
    .context("could not find a suitable adapter")?;

    let (device, queue) = block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            features: wgpu::Features::PUSH_CONSTANTS,
            limits: wgpu::Limits {
                max_push_constant_size: 256,
                ..Default::default()
            },
            shader_validation: true,
        },
        None,
    ))?;

    Ok((device, queue, adapter))
}

pub fn launch_poll_thread(device: &Arc<wgpu::Device>) {
    let device = Arc::clone(device);
    thread::Builder::new()
        .name("device-poller".to_owned())
        .spawn(move || loop {
            device.poll(wgpu::Maintain::Wait);
        })
        .expect("failed to launch device polling thread");
}
