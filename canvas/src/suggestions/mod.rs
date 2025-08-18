// src/suggestions/mod.rs
//! Suggestions subsystem - provider and optional GUI.
//!
//! Contains the suggestion provider types used by the editor and, when the GUI
//! feature is enabled, the rendering helpers for the suggestions dropdown.

pub mod state;
#[cfg(feature = "gui")]
pub mod gui;

// Re-export the main suggestion types
pub use state::{SuggestionsProvider, SuggestionItem};

// Re-export GUI functions if available
#[cfg(feature = "gui")]
pub use gui::render_suggestions_dropdown;
