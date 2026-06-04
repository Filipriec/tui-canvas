//! Multi-field form rendering.

#[cfg(feature = "gui")]
pub mod render;

#[cfg(feature = "gui")]
pub use render::{
    render_canvas, render_canvas_default, render_canvas_with_options, CanvasDisplayOptions,
    OverflowMode,
};
