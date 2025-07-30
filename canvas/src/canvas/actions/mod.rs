// canvas/src/canvas/actions/mod.rs
pub mod types;
pub mod edit;

// Re-export the main types for convenience
pub use types::{CanvasAction, ActionResult};
pub use edit::execute_canvas_action;
