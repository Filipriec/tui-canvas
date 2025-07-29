// src/autocomplete/mod.rs
pub mod types;
pub mod gui;
pub mod state;
pub mod actions;

// Re-export autocomplete types
pub use types::{SuggestionItem, AutocompleteState};
pub use state::AutocompleteCanvasState;
pub use actions::execute_canvas_action_with_autocomplete;
