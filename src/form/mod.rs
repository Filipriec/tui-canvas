//! Multi-field form rendering.

#[cfg(feature = "gui")]
pub mod render;

#[cfg(feature = "gui")]
pub use render::{
    CanvasDisplayOptions, OverflowMode, render_canvas, render_canvas_default,
    render_canvas_with_options, render_canvas_with_options_without_cursor,
    render_canvas_without_cursor,
};
