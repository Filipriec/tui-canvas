// src/editor/mod.rs
//! Editor submodule exports.
//!
//! This module exposes the internal editor pieces (core, editing, movement,
//! navigation, mode, and optional features like suggestions, validation, and
//! computed field helpers). Only module declarations and re-exports live here.

#[cfg(feature = "keybindings")]
pub(crate) mod behavior;
pub(crate) mod paradigm;
pub mod core;
pub mod display;
pub mod editing;
pub mod features;
pub mod mode;
pub mod movement;
pub mod navigation;

pub mod input;

// Re-export the editor core and public fixed-row form surface.
pub use core::{EditorCore, TextForm};
