// src/canvas/mod.rs

pub mod actions;
pub mod state;
pub mod modes;

#[cfg(feature = "gui")]
pub mod gui;
#[cfg(feature = "gui")]
pub mod theme;

#[cfg(feature = "cursor-style")]
pub mod cursor;

// Keep these exports for current functionality
pub use modes::{AppMode, ModeManager, HighlightState};

#[cfg(feature = "cursor-style")]
pub use cursor::CursorManager;
