// src/lib.rs

pub mod canvas;
pub mod editor;
pub mod data_provider;

// Only include autocomplete module if feature is enabled
#[cfg(feature = "autocomplete")]
pub mod autocomplete;

// ===================================================================
// NEW API: Library-owned state pattern
// ===================================================================

// Main API exports
pub use editor::FormEditor;
pub use data_provider::{DataProvider, AutocompleteProvider, SuggestionItem};

// UI state (read-only access for users)
pub use canvas::state::EditorState;
pub use canvas::modes::AppMode;

// Actions and results (for users who want to handle actions manually)
pub use canvas::actions::{CanvasAction, ActionResult};

// Theming and GUI
#[cfg(feature = "gui")]
pub use canvas::theme::CanvasTheme;

#[cfg(feature = "gui")]
pub use canvas::gui::render_canvas;

#[cfg(all(feature = "gui", feature = "autocomplete"))]
pub use autocomplete::gui::render_autocomplete_dropdown;
