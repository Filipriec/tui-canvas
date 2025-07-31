// src/canvas/actions/types.rs

use crate::canvas::state::CanvasState;
use anyhow::Result;

/// All available canvas actions
#[derive(Debug, Clone, PartialEq)]
pub enum CanvasAction {
    // Movement actions
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    
    // Word movement
    MoveWordNext,
    MoveWordPrev,
    MoveWordEnd,
    MoveWordEndPrev,
    
    // Line movement
    MoveLineStart,
    MoveLineEnd,
    
    // Field movement
    NextField,
    PrevField,
    MoveFirstLine,
    MoveLastLine,
    
    // Editing actions
    InsertChar(char),
    DeleteBackward,
    DeleteForward,
    
    // Autocomplete actions
    TriggerAutocomplete,
    SuggestionUp,
    SuggestionDown,
    SelectSuggestion,
    ExitSuggestions,
    
    // Custom actions
    Custom(String),
}

/// Result type for canvas actions
#[derive(Debug, Clone)]
pub enum ActionResult {
    Success,
    Message(String),
    HandledByApp(String),
    HandledByFeature(String), // Keep for compatibility
    Error(String),
}

impl ActionResult {
    pub fn success() -> Self {
        Self::Success
    }
    
    pub fn success_with_message(msg: &str) -> Self {
        Self::Message(msg.to_string())
    }
    
    pub fn handled_by_app(msg: &str) -> Self {
        Self::HandledByApp(msg.to_string())
    }
    
    pub fn error(msg: &str) -> Self {
        Self::Error(msg.to_string())
    }
    
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success | Self::Message(_) | Self::HandledByApp(_) | Self::HandledByFeature(_))
    }
    
    pub fn message(&self) -> Option<&str> {
        match self {
            Self::Message(msg) | Self::HandledByApp(msg) | Self::HandledByFeature(msg) | Self::Error(msg) => Some(msg),
            Self::Success => None,
        }
    }
}

/// Execute a canvas action on the given state
pub async fn execute<S: CanvasState>(
    action: CanvasAction,
    state: &mut S,
) -> Result<ActionResult> {
    let mut ideal_cursor_column = 0;
    
    super::handlers::dispatch_action(action, state, &mut ideal_cursor_column).await
}
