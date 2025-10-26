// src/canvas/mod.rs
//! Top-level canvas module.
//!
//! Re-exports commonly used canvas types and modules so that downstream
//! consumers can import them from `crate::canvas`.

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
