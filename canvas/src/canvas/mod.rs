// src/canvas/mod.rs
pub mod actions;
pub mod modes; 
pub mod gui;
pub mod theme;
pub mod state;

// Re-export commonly used canvas types
pub use actions::{CanvasAction, ActionResult};
pub use modes::{AppMode, ModeManager, HighlightState};
pub use state::{CanvasState, ActionContext};

#[cfg(feature = "gui")]
pub use theme::CanvasTheme;

#[cfg(feature = "gui")]
pub use gui::render_canvas;
