// src/canvas/actions/types.rs
//! Core action types and result handling for canvas operations.

/// All available canvas actions.
///
/// This enum lists high-level actions that can be performed on the canvas.
/// Consumers can match on variants to implement custom handling or map input
/// events to these canonical actions.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanvasAction {
    // Movement actions
    /// Move the cursor left by one character (or logical unit).
    MoveLeft,
    /// Move the cursor right by one character (or logical unit).
    MoveRight,
    /// Move the cursor up a visual line/field.
    MoveUp,
    /// Move the cursor down a visual line/field.
    MoveDown,

    // Word movement
    /// Move to the start of the next word.
    MoveWordNext,
    /// Move to the start of the previous word.
    MoveWordPrev,
    /// Move to the end of the current/next word.
    MoveWordEnd,
    /// Move to the previous word end (vim `ge`).
    MoveWordEndPrev,

    // Line movement
    /// Move to the start of the current line.
    MoveLineStart,
    /// Move to the end of the current line.
    MoveLineEnd,

    // Field movement
    /// Move to the next field.
    NextField,
    /// Move to the previous field.
    PrevField,
    /// Move to the first field.
    MoveFirstLine,
    /// Move to the last field.
    MoveLastLine,

    // Editing actions
    /// Insert a character at the cursor.
    InsertChar(char),
    /// Delete character before the cursor.
    DeleteBackward,
    /// Delete character under/after the cursor.
    DeleteForward,

    // Suggestions actions
    /// Trigger suggestions dropdown (e.g. Tab).
    TriggerSuggestions,
    /// Move selection up in suggestions dropdown.
    SuggestionUp,
    /// Move selection down in suggestions dropdown.
    SuggestionDown,
    /// Accept the selected suggestion.
    SelectSuggestion,
    /// Exit suggestions UI.
    ExitSuggestions,

    // Custom actions
    /// Custom named action for application-specific behavior.
    Custom(String),

    // Mode transitions
    EnterEditMode,
    EnterEditModeAfter,
    ExitEditMode,
    EnterHighlightMode,
    EnterHighlightModeLinewise,
    ExitHighlightMode,

    // Advanced editing
    OpenLineBelow,
    OpenLineAbove,
}

/// Result type for canvas actions.
///
/// Action handlers return an ActionResult to indicate success, user-facing
/// messages, or errors. The enum is non-exhaustive to allow extension.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum ActionResult {
    /// Action completed successfully.
    Success,
    /// Action completed with a user-facing message.
    Message(String),
    /// An error occurred while handling the action.
    Error(String),
}

impl ActionResult {
    /// Convenience constructor for Success.
    pub fn success() -> Self {
        Self::Success
    }

    /// Convenience constructor for Message.
    pub fn success_with_message(msg: &str) -> Self {
        Self::Message(msg.to_string())
    }

    /// Convenience constructor for message-style handled outcomes.
    pub fn handled_by_app(msg: &str) -> Self {
        Self::Message(msg.to_string())
    }

    /// Convenience constructor for Error.
    pub fn error(msg: &str) -> Self {
        Self::Error(msg.to_string())
    }

    /// Returns true for any variant representing a success-like outcome.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success | Self::Message(_))
    }

    /// Extract a message from the result when present.
    pub fn message(&self) -> Option<&str> {
        match self {
            Self::Message(msg) | Self::Error(msg) => Some(msg),
            Self::Success => None,
        }
    }
}

impl CanvasAction {
    /// Get a human-readable description of this action.
    pub fn description(&self) -> &'static str {
        match self {
            Self::MoveLeft => "move left",
            Self::MoveRight => "move right",
            Self::MoveUp => "move up",
            Self::MoveDown => "move down",
            Self::MoveWordNext => "next word",
            Self::MoveWordPrev => "previous word",
            Self::MoveWordEnd => "word end",
            Self::MoveWordEndPrev => "previous word end",
            Self::MoveLineStart => "line start",
            Self::MoveLineEnd => "line end",
            Self::NextField => "next field",
            Self::PrevField => "previous field",
            Self::MoveFirstLine => "first field",
            Self::MoveLastLine => "last field",
            Self::InsertChar(_c) => "insert character",
            Self::DeleteBackward => "delete backward",
            Self::DeleteForward => "delete forward",
            Self::TriggerSuggestions => "trigger suggestions",
            Self::SuggestionUp => "suggestion up",
            Self::SuggestionDown => "suggestion down",
            Self::SelectSuggestion => "select suggestion",
            Self::ExitSuggestions => "exit suggestions",
            Self::Custom(_name) => "custom action",

            // Mode transitions
            Self::EnterEditMode => "enter edit mode",
            Self::EnterEditModeAfter => "enter append mode (a)",
            Self::ExitEditMode => "exit edit mode",
            Self::EnterHighlightMode => "enter visual mode",
            Self::EnterHighlightModeLinewise => "enter linewise visual mode",
            Self::ExitHighlightMode => "exit visual mode",

            // Advanced editing
            Self::OpenLineBelow => "open line below (o)",
            Self::OpenLineAbove => "open line above (O)",
        }
    }

    /// Get all movement-related actions.
    pub fn movement_actions() -> Vec<CanvasAction> {
        vec![
            Self::MoveLeft,
            Self::MoveRight,
            Self::MoveUp,
            Self::MoveDown,
            Self::MoveWordNext,
            Self::MoveWordPrev,
            Self::MoveWordEnd,
            Self::MoveWordEndPrev,
            Self::MoveLineStart,
            Self::MoveLineEnd,
            Self::NextField,
            Self::PrevField,
            Self::MoveFirstLine,
            Self::MoveLastLine,
        ]
    }

    /// Get all editing-related actions.
    pub fn editing_actions() -> Vec<CanvasAction> {
        vec![
            Self::InsertChar(' '), // Example char
            Self::DeleteBackward,
            Self::DeleteForward,
        ]
    }

    /// Get all suggestions-related actions.
    pub fn suggestions_actions() -> Vec<CanvasAction> {
        vec![
            Self::TriggerSuggestions,
            Self::SuggestionUp,
            Self::SuggestionDown,
            Self::SelectSuggestion,
            Self::ExitSuggestions,
        ]
    }

    /// Check if this action modifies text content.
    pub fn is_editing_action(&self) -> bool {
        matches!(self,
            Self::InsertChar(_) |
            Self::DeleteBackward |
            Self::DeleteForward
        )
    }

    /// Check if this action moves the cursor.
    pub fn is_movement_action(&self) -> bool {
        matches!(self,
            Self::MoveLeft | Self::MoveRight | Self::MoveUp | Self::MoveDown |
            Self::MoveWordNext | Self::MoveWordPrev | Self::MoveWordEnd | Self::MoveWordEndPrev |
            Self::MoveLineStart | Self::MoveLineEnd | Self::NextField | Self::PrevField |
            Self::MoveFirstLine | Self::MoveLastLine
        )
    }
}
