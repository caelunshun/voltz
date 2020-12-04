//! Node-graph-based UI library.
//!
//! Uses:
//! * `tiny-skia` for rendering
//! * `fontdue` for text rendering and layout
//! * `stretch` for node layout
//! * `std::panic::Location` for node stable identity

pub mod canvas;
pub mod ui;
pub mod widget;
pub mod widgets;

pub use canvas::{Canvas, Path};
pub use ui::Ui;
pub use widget::{WidgetData, WidgetState};

#[doc(inline)]
pub use stretch::style::*;
