// src/suggestions/mod.rs
//! Suggestions subsystem - provider and optional GUI.
//!
//! Contains the suggestion provider types used by the editor and, when the GUI
//! feature is enabled, the rendering helpers for the suggestions dropdown.

#[cfg(feature = "gui")]
pub mod gui;
pub mod state;

pub use state::SuggestionItem;

#[cfg(feature = "gui")]
pub use gui::render_suggestions_dropdown;
