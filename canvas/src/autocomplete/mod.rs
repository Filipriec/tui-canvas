// src/autocomplete/mod.rs
pub mod types;
pub mod gui;
pub mod state;  // Add this line

// Re-export autocomplete types
pub use types::{SuggestionItem, AutocompleteState};
pub use state::AutocompleteCanvasState;  // Add this line
