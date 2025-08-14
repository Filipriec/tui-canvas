// src/editor/mod.rs
// Only module declarations and re-exports.

pub mod core;
pub mod display;
pub mod editing;
pub mod movement;
pub mod navigation;
pub mod mode;

#[cfg(feature = "suggestions")]
pub mod suggestions;

#[cfg(feature = "validation")]
pub mod validation_helpers;

#[cfg(feature = "computed")]
pub mod computed_helpers;

// Re-export the main type
pub use core::FormEditor;
