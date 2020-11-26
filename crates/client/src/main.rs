#![feature(type_name_of_val)]
#![allow(dead_code)]

use anyhow::{anyhow, Context};
use asset::{model::YamlModel, texture::PngLoader, Assets, YamlLoader};
use renderer::Renderer;
use sdl2::{event::Event, video::Window, EventPump};
use simple_logger::SimpleLogger;

mod asset;
mod renderer;

pub struct Client {
    assets: Assets,
    renderer: Renderer,

    // SDL2 state
    window: Window,
    event_pump: EventPump,

    open: bool,
}

impl Client {
    pub fn run(mut self) -> anyhow::Result<()> {
        while self.open {
            self.handle_events();
        }
        Ok(())
    }

    fn handle_events(&mut self) {
        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => self.open = false,
                _ => (),
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Debug)
        .init()?;
    let assets = load_assets()?;
    let (window, event_pump) =
        init_sdl2().map_err(|e| anyhow!("failed to initialize SDL2: {}", e))?;
    let renderer = Renderer::new(&window, &assets).context("failed to intiailize wgpu renderer")?;

    let client = Client {
        assets,
        renderer,

        window,
        event_pump,

        open: true,
    };
    client.run()
}

fn load_assets() -> anyhow::Result<Assets> {
    let mut assets = Assets::new();
    assets
        .add_loader("YamlModel", YamlLoader::<YamlModel>::new())
        .add_loader("Png", PngLoader::new());
    assets.load_dir("assets").context("failed to load assets")?;
    Ok(assets)
}

fn init_sdl2() -> Result<(Window, EventPump), String> {
    let sdl2 = sdl2::init()?;
    let video = sdl2.video()?;

    let title = "Voltz";
    let width = 1920 / 2;
    let height = 1080 / 2;

    let window = video
        .window(title, width, height)
        .allow_highdpi()
        .build()
        .map_err(|e| e.to_string())?;
    let event_pump = sdl2.event_pump()?;

    Ok((window, event_pump))
}
