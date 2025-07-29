// canvas/src/lib.rs
//! Canvas - A reusable text editing and form canvas system
//!
//! This crate provides a generic canvas abstraction for building text-based interfaces
//! with multiple input fields, cursor management, and mode-based editing.

pub mod state;
pub mod actions;
pub mod modes;
pub mod config;
pub mod suggestions;
pub mod dispatcher;

// Re-export the main types for easy use
pub use state::{CanvasState, ActionContext};
pub use actions::{CanvasAction, ActionResult, execute_edit_action, execute_canvas_action};
pub use modes::{AppMode, ModeManager, HighlightState};
pub use suggestions::SuggestionState;
pub use dispatcher::ActionDispatcher;

// High-level convenience API
pub mod prelude {
    pub use crate::{
        CanvasState,
        ActionContext,
        CanvasAction,
        ActionResult,
        execute_edit_action,
        execute_canvas_action,
        ActionDispatcher,
        AppMode,
        ModeManager,
        HighlightState,
        SuggestionState,
    };
}
