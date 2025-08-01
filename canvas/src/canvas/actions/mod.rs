// src/canvas/actions/mod.rs

pub mod types;
pub mod movement;

// Re-export the main API
pub use types::{CanvasAction, ActionResult};
