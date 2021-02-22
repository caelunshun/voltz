use std::any::Any;

use super::AssetLoader;

/// A SPIR-V shader.
pub struct ShaderAsset(wgpu::ShaderSource<'static>);

impl ShaderAsset {
    /// Gets the SPIRV data.
    pub fn to_source(&self) -> wgpu::ShaderSource<'static> {
        match &self.0 {
            wgpu::ShaderSource::SpirV(spv) => wgpu::ShaderSource::SpirV(spv.clone()),
            wgpu::ShaderSource::Wgsl(wgsl) => wgpu::ShaderSource::Wgsl(wgsl.clone()),
        }
    }
}

/// Loader for `ShaderAsset`s.
pub struct SpirvLoader;

impl SpirvLoader {
    pub fn new() -> Self {
        Self
    }
}

impl AssetLoader for SpirvLoader {
    fn load(&self, data: &[u8]) -> anyhow::Result<Box<dyn Any + Send + Sync>> {
        let source = wgpu::util::make_spirv(data);
        // Make source 'static
        let source = match source {
            wgpu::ShaderSource::SpirV(spv) => wgpu::ShaderSource::SpirV(spv.into_owned().into()),
            wgpu::ShaderSource::Wgsl(wgsl) => wgpu::ShaderSource::Wgsl(wgsl.into_owned().into()),
        };
        Ok(Box::new(ShaderAsset(source)))
    }
}
