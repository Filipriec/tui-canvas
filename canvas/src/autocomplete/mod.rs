// src/autocomplete/mod.rs

pub mod state;
#[cfg(feature = "gui")]
pub mod gui;

// Re-export the main autocomplete types
pub use state::{AutocompleteProvider, SuggestionItem};

// Re-export GUI functions if available
#[cfg(feature = "gui")]
pub use gui::render_autocomplete_dropdown;
