//! Node-graph-based UI library.
//!
//! Uses:
//! * `tiny-skia` for rendering
//! * `fontdue` for text rendering and layout
//! * `stretch` for node layout
//!
//! Similar in design to `bevy_ui` but without
//! the ECS being externally exposed. Also doesn't
//! use the GPU for rendering.

pub mod canvas;
pub use canvas::{Canvas, Path};

pub mod ui;
