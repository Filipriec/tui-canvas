// src/lib.rs
pub mod canvas;
pub mod autocomplete;
pub mod config;
pub mod dispatcher;

// Re-export the main API for easy access
pub use dispatcher::{execute_canvas_action, ActionDispatcher};
pub use canvas::actions::{CanvasAction, ActionResult};
pub use canvas::state::{CanvasState, ActionContext};
pub use canvas::modes::{AppMode, HighlightState, ModeManager};
