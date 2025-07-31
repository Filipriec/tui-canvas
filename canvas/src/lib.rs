// src/lib.rs

pub mod canvas;

// Only include autocomplete module if feature is enabled
#[cfg(feature = "autocomplete")]
pub mod autocomplete;

// Re-export the main API for easy access
pub use canvas::actions::{CanvasAction, ActionResult, execute};
pub use canvas::state::{CanvasState, ActionContext};
pub use canvas::modes::{AppMode, ModeManager, HighlightState};

#[cfg(feature = "gui")]
pub use canvas::theme::CanvasTheme;

#[cfg(feature = "gui")]
pub use canvas::gui::render_canvas;

// Re-export autocomplete API if feature is enabled
#[cfg(feature = "autocomplete")]
pub use autocomplete::{
    AutocompleteCanvasState, 
    AutocompleteState, 
    SuggestionItem,
    execute_with_autocomplete,
    handle_autocomplete_feature_action,
};

#[cfg(all(feature = "gui", feature = "autocomplete"))]
pub use autocomplete::render_autocomplete_dropdown;
