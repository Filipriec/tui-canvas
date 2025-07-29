// canvas/src/gui/mod.rs

#[cfg(feature = "gui")]
pub mod canvas;

#[cfg(feature = "gui")]
pub mod autocomplete;

#[cfg(feature = "gui")]
pub mod theme;

// Export the separate rendering functions
#[cfg(feature = "gui")]
pub use canvas::render_canvas;

#[cfg(feature = "gui")]
pub use autocomplete::render_autocomplete_dropdown;

#[cfg(feature = "gui")]
pub use theme::CanvasTheme;
