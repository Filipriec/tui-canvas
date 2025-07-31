// src/autocomplete/mod.rs

pub mod types;
pub mod state;
pub mod actions;

#[cfg(feature = "gui")]
pub mod gui;

// Re-export the main autocomplete API
pub use types::{SuggestionItem, AutocompleteState};
pub use state::AutocompleteCanvasState;

// Re-export the new action functions
pub use actions::{
    execute_with_autocomplete,
    handle_autocomplete_feature_action,
};

// Re-export GUI functions if available
#[cfg(feature = "gui")]
pub use gui::render_autocomplete_dropdown;
