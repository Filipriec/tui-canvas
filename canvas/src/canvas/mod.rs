// src/canvas/mod.rs
pub mod actions;
pub mod modes; 
pub mod gui;
pub mod theme;
pub mod state;  // Add this line

// Re-export commonly used canvas types
pub use actions::{CanvasAction, ActionResult};
pub use modes::{AppMode, ModeManager, HighlightState};
pub use state::{CanvasState, ActionContext};  // Add this line

#[cfg(feature = "gui")]
pub use theme::CanvasTheme;
