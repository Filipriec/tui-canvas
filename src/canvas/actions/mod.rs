// src/canvas/actions/mod.rs
//! Canvas action definitions and movement utilities

pub mod types;
pub mod movement;
pub mod dispatch;

// Re-export the main API
pub use types::{CanvasAction, ActionResult};
