// canvas/src/lib.rs

pub mod actions;
pub mod config;
pub mod dispatcher;
pub mod state;
pub mod suggestions; // Keep for backwards compatibility
pub mod autocomplete; // NEW: Core autocomplete functionality
pub mod modes;

#[cfg(feature = "gui")]
pub mod gui;

// Re-export commonly used types
pub use actions::{CanvasAction, ActionResult};
pub use dispatcher::ActionDispatcher;
pub use state::{CanvasState, ActionContext};
pub use autocomplete::{SuggestionItem, AutocompleteState}; // NEW
pub use modes::{AppMode, ModeManager, HighlightState};

#[cfg(feature = "gui")]
pub use gui::{render_canvas, CanvasTheme};

// Keep backwards compatibility exports
pub use suggestions::SuggestionState;
