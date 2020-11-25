use image::ImageFormat;

use super::AssetLoader;

/// A texture stored in BGRA8.
pub struct TextureAsset {
    data: Vec<u8>,
    width: u32,
    height: u32,
}

impl TextureAsset {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

#[derive(Default)]
pub struct PngLoader;

impl PngLoader {
    pub fn new() -> Self {
        Self::default()
    }
}

impl AssetLoader for PngLoader {
    fn load(&self, data: &[u8]) -> anyhow::Result<Box<dyn std::any::Any + Send + Sync>> {
        let image = image::load_from_memory_with_format(data, ImageFormat::Png)?.to_bgra8();

        let texture = TextureAsset {
            width: image.width(),
            height: image.height(),
            data: image.into_raw(),
        };
        Ok(Box::new(texture))
    }
}
