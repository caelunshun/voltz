/// Renderer which blits rendered `voltzui::Ui` canvases
/// to the present surface.
pub struct UiRenderer {
    pipeline: wgpu::RenderPipeline,
    bg_layout: wgpu::BindGroupLayout,
}
