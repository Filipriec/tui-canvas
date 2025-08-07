// src/suggestions/mod.rs

pub mod state;
#[cfg(feature = "gui")]
pub mod gui;

// Re-export the main suggestion types
pub use state::{SuggestionsProvider, SuggestionItem};

// Re-export GUI functions if available
#[cfg(feature = "gui")]
pub use gui::render_suggestions_dropdown;
