use anyhow::anyhow;
use fontdue::{Font, FontSettings};

use super::AssetLoader;

pub struct FontLoader;

impl FontLoader {
    pub fn new() -> Self {
        Self
    }
}

impl AssetLoader for FontLoader {
    fn load(&self, data: &[u8]) -> anyhow::Result<Box<dyn std::any::Any + Send + Sync>> {
        let font = Font::from_bytes(data, FontSettings::default()).map_err(|e| anyhow!("{}", e))?;
        Ok(Box::new(font))
    }
}
