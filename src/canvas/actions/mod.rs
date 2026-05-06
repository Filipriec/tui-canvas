// src/canvas/actions/mod.rs
//! Canvas action definitions and movement utilities

pub mod dispatch;
pub mod movement;
pub mod types;

// Re-export the main API
pub use types::{ActionResult, CanvasAction};
