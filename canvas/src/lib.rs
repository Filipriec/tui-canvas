// src/lib.rs
pub mod canvas;
pub mod autocomplete;
pub mod config;
pub mod dispatcher;
pub mod suggestions; // Keep for backwards compatibility

// Re-export from modules
pub use canvas::{CanvasAction, ActionResult, AppMode, ModeManager, HighlightState};

#[cfg(feature = "gui")]
pub use canvas::CanvasTheme;

pub use autocomplete::{SuggestionItem, AutocompleteState};
pub use dispatcher::ActionDispatcher;
pub use canvas::state::{CanvasState, ActionContext};  // Fixed path

// Backwards compatibility
pub use suggestions::SuggestionState;
