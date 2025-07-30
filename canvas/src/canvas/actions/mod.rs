// src/canvas/actions/mod.rs

pub mod types;
pub mod movement;
pub mod handlers;

// Re-export the main types
pub use types::{CanvasAction, ActionResult};
