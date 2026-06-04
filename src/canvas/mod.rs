// src/canvas/mod.rs
//! Top-level canvas module.
//!
//! Re-exports commonly used canvas types and modules so that downstream
//! consumers can import them from `crate::canvas`.

pub mod actions;
pub mod modes;
pub mod state;

#[cfg(feature = "gui")]
pub mod theme;

pub use modes::{AppMode, HighlightState};
