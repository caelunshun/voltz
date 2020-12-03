use std::borrow::Cow;

use anyhow::{anyhow, bail};
pub use tiny_skia::FilterQuality;
use tiny_skia::{Canvas, Pixmap, PixmapPaint};

/// Performs scaling (upsampling or downsampling)
/// on textures. Also able to generate mipmaps with
/// high quality cubic filtering.
pub struct TextureScaler;

impl TextureScaler {
    pub fn new() -> Self {
        Self
    }

    /// Scales a texture to new dimensions using
    /// the provided filter quality.
    pub fn scale(
        &mut self,
        texture: &[u8],
        input_width: u32,
        input_height: u32,
        output_width: u32,
        output_height: u32,
        quality: FilterQuality,
    ) -> anyhow::Result<Vec<u8>> {
        let mut input = Pixmap::new(input_width, input_height)
            .ok_or_else(|| anyhow!("input dimensions zero"))?;
        if texture.len() != input.data().len() {
            bail!("texture data length must match dimensions");
        }
        input.data_mut().copy_from_slice(texture);

        let mut output = Canvas::new(output_width, output_height)
            .ok_or_else(|| anyhow!("output dimensions zero"))?;

        let scale_x = output_width as f32 / input_width as f32;
        let scale_y = output_height as f32 / input_height as f32;
        output.scale(scale_x, scale_y);

        output.draw_pixmap(
            0,
            0,
            &input,
            &PixmapPaint {
                quality,
                ..Default::default()
            },
        );

        Ok(output.pixmap.take())
    }

    /// Generates mipmaps and writes them to the given GPU texture.
    ///
    /// Mip level 0 is taken from `texture`. This function will write
    /// mipmap levels `0..num_levels` to the `target`. Uses bicubic
    /// filtering for maximum quality mipmaps.
    pub fn generate_mipmaps(
        &mut self,
        texture: &[u8],
        width: u32,
        height: u32,
        num_levels: u32,
        target: &wgpu::Texture,
        array_layer: u32,
        queue: &wgpu::Queue,
    ) -> anyhow::Result<()> {
        for level in 0..num_levels {
            let mip_width = width / 2u32.pow(level);
            let mip_height = height / 2u32.pow(level);
            let data = if level == 0 {
                Cow::Borrowed(texture)
            } else {
                Cow::Owned(self.scale(
                    texture,
                    width,
                    height,
                    mip_width,
                    mip_height,
                    FilterQuality::Bicubic,
                )?)
            };
            queue.write_texture(
                wgpu::TextureCopyView {
                    texture: target,
                    mip_level: level,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: array_layer,
                    },
                },
                &data,
                wgpu::TextureDataLayout {
                    offset: 0,
                    bytes_per_row: 4 * mip_width,
                    rows_per_image: mip_height,
                },
                wgpu::Extent3d {
                    width: mip_width,
                    height: mip_height,
                    depth: 1,
                },
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_white() {
        let width = 100;
        let height = 100;
        let texture = vec![u8::MAX; width * height * 4];
        let result = TextureScaler::new()
            .scale(
                &texture,
                width as u32,
                height as u32,
                width as u32 * 2,
                height as u32 * 2,
                FilterQuality::Bilinear,
            )
            .unwrap();

        assert_eq!(result, vec![u8::MAX; width * height * 4 * 4]);
    }
}
