use std::{env, ptr, sync::Arc, time::Instant};

use futures_executor::block_on;
use image::{ImageBuffer, Rgba};
use renderdoc::RenderDoc;
use wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
use worldgen::biomes::{BiomeGenerator, BIOME_GRID_FORMAT};

fn main() -> anyhow::Result<()> {
    let renderdoc = if env::var("WORLDGEN_RENDERDOC").is_ok() {
        let mut renderdoc = RenderDoc::<renderdoc::V100>::new()?;
        renderdoc.trigger_capture();
        renderdoc.start_frame_capture(ptr::null(), ptr::null());
        Some(renderdoc)
    } else {
        None
    };

    let (device, queue, _) =
        common::gpu::init(wgpu::Instance::new(wgpu::BackendBit::PRIMARY), None)?;
    let device = Arc::new(device);
    common::gpu::launch_poll_thread(&device);

    let generator = BiomeGenerator::new(&device);

    let start = Instant::now();
    let bundle = generator.prepare(&device, 10, 4096);

    let dim = bundle.output_size();
    let output_texture = bundle.output_texture();

    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut pass = encoder.begin_compute_pass();
        generator.execute(&bundle, &mut pass, &queue);
    }

    // Read biomes into an image on the CPU.
    let mut image: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(dim, dim);

    dbg!(dim);
    let dim_aligned = (dim + COPY_BYTES_PER_ROW_ALIGNMENT - 1) / COPY_BYTES_PER_ROW_ALIGNMENT
        * COPY_BYTES_PER_ROW_ALIGNMENT;
    dbg!(dim_aligned);

    let temp_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: dim_aligned,
            height: dim,
            depth: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: BIOME_GRID_FORMAT,
        usage: wgpu::TextureUsage::COPY_SRC | wgpu::TextureUsage::COPY_DST,
    });
    encoder.copy_texture_to_texture(
        wgpu::TextureCopyView {
            texture: output_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
        },
        wgpu::TextureCopyView {
            texture: &temp_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
        },
        wgpu::Extent3d {
            width: dim,
            height: dim,
            depth: 1,
        },
    );

    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (dim_aligned * dim) as u64,
        usage: wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::MAP_READ,
        mapped_at_creation: false,
    });

    encoder.copy_texture_to_buffer(
        wgpu::TextureCopyView {
            texture: &temp_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
        },
        wgpu::BufferCopyView {
            buffer: &buffer,
            layout: wgpu::TextureDataLayout {
                offset: 0,
                bytes_per_row: dim_aligned,
                rows_per_image: dim,
            },
        },
        wgpu::Extent3d {
            width: dim_aligned,
            height: dim,
            depth: 1,
        },
    );

    queue.submit(vec![encoder.finish()]);

    block_on(buffer.slice(..).map_async(wgpu::MapMode::Read)).unwrap();
    println!("{:?}", start.elapsed());

    let view = buffer.slice(..).get_mapped_range();

    for x in 0..dim {
        for y in 0..dim {
            let index = y * dim_aligned + x;
            let src = view[index as usize];

            let color = if src == 0 {
                Rgba([40, 80, 200, u8::MAX])
            } else if src == 1 {
                Rgba([40, 200, 80, u8::MAX])
            } else if src == 2 {
                Rgba([140, 80, 80, u8::MAX])
            } else if src == 3 {
                Rgba([200, 180, 20, u8::MAX])
            } else if src == 4 {
                Rgba([40, 140, 20, u8::MAX])
            } else if src == 5 {
                Rgba([40, 40, 160, u8::MAX])
            } else {
                panic!("unexpected biome value {}", src)
            };

            image.put_pixel(x, y, color);
        }
    }

    image.save("biomes.png")?;

    if let Some(mut renderdoc) = renderdoc {
        renderdoc.end_frame_capture(ptr::null(), ptr::null());
    }
    Ok(())
}
