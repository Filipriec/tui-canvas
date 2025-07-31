// src/canvas/actions/mod.rs

pub mod types;
pub mod handlers;
pub mod movement;

// Re-export the main API
pub use types::{CanvasAction, ActionResult, execute};
