use std::sync::Arc;

use anyhow::{anyhow, Context};
use common::{System, SystemExecutor};
use futures_executor::block_on;
use present::Presenter;
use winit::window::Window;

use crate::{asset::Assets, game::Game};

use self::{chunk::ChunkRenderer, ui::UiRenderer};

mod chunk;
mod present;
mod ui;
mod utils;

const SC_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;
const SAMPLE_COUNT: u32 = 2;

#[derive(Debug)]
pub struct Resources {
    adapter: wgpu::Adapter,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
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
    ui_renderer: UiRenderer,
    presenter: Presenter,
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
            // SAFETY: a wgpu surface can be created with a winit window.
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
                    max_push_constant_size: 256,
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
            device: Arc::new(device),
            queue: Arc::new(queue),
            surface,
        });

        let size = window.inner_size();
        let presenter = Presenter::new(
            resources.device(),
            resources.surface(),
            size.width,
            size.height,
        );

        let mut init_encoder =
            resources
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("init_encoder"),
                });

        let chunk_renderer = ChunkRenderer::new(&resources, assets, &mut init_encoder)
            .context("failed to initialize chunk renderer")?;
        let ui_renderer =
            UiRenderer::new(&resources, assets).context("failed to initialize UI renderer")?;

        resources.queue().submit(vec![init_encoder.finish()]);

        common::gpu::launch_poll_thread(&resources.device);

        Ok(Self {
            resources,
            chunk_renderer,
            ui_renderer,
            presenter,
        })
    }

    pub fn setup(self, systems: &mut SystemExecutor<Game>, game: &mut Game) {
        game.debug_data.adapter = Some(self.resources.adapter().get_info());
        systems.add(self);
    }

    pub fn device_arc(&self) -> &Arc<wgpu::Device> {
        &self.resources.device
    }

    pub fn queue_arc(&self) -> &Arc<wgpu::Queue> {
        &self.resources.queue
    }

    fn on_resize(&mut self, new_width: u32, new_height: u32) {
        self.presenter = Presenter::new(
            self.resources.device(),
            self.resources.surface(),
            new_width,
            new_height,
        );
    }

    /// Renders a frame.
    fn render(&mut self, game: &mut Game) {
        self.prep_render(game);
        self.do_render(game);
    }

    fn prep_render(&mut self, game: &mut Game) {
        self.chunk_renderer.prep_render(&self.resources, game);
        self.ui_renderer.prep_render(&self.resources, game);
    }

    fn do_render(&mut self, game: &mut Game) {
        let mut encoder =
            self.resources
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("render_frame"),
                });

        let frame = self
            .presenter
            .swapchain()
            .get_current_frame()
            .expect("failed to get next output frame");

        {
            let mut pass_3d = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: self.presenter.sample_buffer(),
                    resolve_target: Some(&frame.output.view),
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
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    attachment: self.presenter.depth_buffer(),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            self.chunk_renderer.do_render(&mut pass_3d, game);
        }
        {
            let mut pass_2d = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.output.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            self.ui_renderer.do_render(&mut pass_2d);
        }

        self.resources.queue().submit(vec![encoder.finish()]);
    }
}

impl System<Game> for Renderer {
    fn run(&mut self, game: &mut Game) {
        let size = game.window().inner_size();
        if size.width != self.presenter.width() || size.height != self.presenter.height() {
            self.on_resize(size.width, size.height);
        }

        self.render(game);
    }
}
