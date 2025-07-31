// src/lib.rs

pub mod canvas;
// pub mod autocomplete;
pub mod dispatcher; // Keep for compatibility

// Re-export the main API for easy access
pub use canvas::actions::{CanvasAction, ActionResult, execute};
pub use canvas::state::{CanvasState, ActionContext};
pub use canvas::modes::{AppMode, HighlightState, ModeManager};

// Keep legacy exports for compatibility
pub use dispatcher::{execute_canvas_action, ActionDispatcher};

// Re-export result type for convenience
pub type Result<T> = anyhow::Result<T>;
