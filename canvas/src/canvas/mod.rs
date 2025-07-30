// src/canvas/mod.rs
pub mod actions;
pub mod gui;
pub mod modes; 
pub mod state;
pub mod theme;

// Re-export commonly used canvas types
pub use actions::{CanvasAction, ActionResult};
pub use modes::{AppMode, ModeManager, HighlightState};
pub use state::{CanvasState, ActionContext};

// Re-export the main entry point
pub use crate::dispatcher::execute_canvas_action;

#[cfg(feature = "gui")]
pub use theme::CanvasTheme;

#[cfg(feature = "gui")]
pub use gui::render_canvas;
