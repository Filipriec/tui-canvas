// src/canvas/mod.rs
//! Top-level canvas module.
//!
//! Re-exports commonly used canvas types and modules so that downstream
//! consumers can import them from `crate::canvas`.

pub mod actions;
pub mod modes;
pub mod state;

#[cfg(feature = "gui")]
pub mod gui;
#[cfg(feature = "gui")]
pub mod theme;

#[cfg(feature = "cursor-style")]
pub mod cursor;

pub use modes::{AppMode, HighlightState};

#[cfg(feature = "cursor-style")]
pub use cursor::CursorManager;
