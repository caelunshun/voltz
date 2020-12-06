use std::any::Any;

use super::AssetLoader;

/// A SPIR-V shader.
pub struct ShaderAsset(wgpu::ShaderModuleSource<'static>);

impl ShaderAsset {
    /// Gets the SPIRV data.
    pub fn to_source(&self) -> wgpu::ShaderModuleSource<'static> {
        match &self.0 {
            wgpu::ShaderModuleSource::SpirV(spv) => wgpu::ShaderModuleSource::SpirV(spv.clone()),
            wgpu::ShaderModuleSource::Wgsl(wgsl) => wgpu::ShaderModuleSource::Wgsl(wgsl.clone()),
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
            wgpu::ShaderModuleSource::SpirV(spv) => {
                wgpu::ShaderModuleSource::SpirV(spv.into_owned().into())
            }
            wgpu::ShaderModuleSource::Wgsl(wgsl) => {
                wgpu::ShaderModuleSource::Wgsl(wgsl.into_owned().into())
            }
        };
        Ok(Box::new(ShaderAsset(source)))
    }
}
