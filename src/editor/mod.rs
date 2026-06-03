// src/editor/mod.rs
//! Editor submodule exports.
//!
//! This module exposes the internal editor pieces (core, editing, movement,
//! navigation, mode, and optional features like suggestions, validation, and
//! computed field helpers). Only module declarations and re-exports live here.

pub mod core;
#[cfg(feature = "keybindings")]
pub(crate) mod behavior;
pub mod display;
pub mod editing;
#[cfg(feature = "crossterm")]
pub mod event_input;
pub mod mode;
pub mod movement;
pub mod navigation;

#[cfg(feature = "suggestions")]
pub mod suggestions;

#[cfg(feature = "validation")]
pub mod validation_helpers;

#[cfg(feature = "computed")]
pub mod computed_helpers;

#[cfg(feature = "keybindings")]
pub mod key_input;

pub mod history;

// Re-export the main type
pub use core::FormEditor;
