// src/canvas/actions/mod.rs

pub mod types;
pub mod movement;
pub mod handlers;
pub mod edit;  // Compatibility layer

// Re-export the main types for convenience
pub use types::{CanvasAction, ActionResult};

// Re-export from edit.rs for backward compatibility
pub use edit::{execute_canvas_action, handle_generic_canvas_action};
