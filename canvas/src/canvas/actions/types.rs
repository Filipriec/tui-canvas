// src/canvas/actions/types.rs

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

impl CanvasAction {
    /// Get a human-readable description of this action
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
            Self::TriggerAutocomplete => "trigger autocomplete",
            Self::SuggestionUp => "suggestion up",
            Self::SuggestionDown => "suggestion down",
            Self::SelectSuggestion => "select suggestion",
            Self::ExitSuggestions => "exit suggestions",
            Self::Custom(_name) => "custom action",
        }
    }

    /// Get all movement-related actions
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

    /// Get all editing-related actions
    pub fn editing_actions() -> Vec<CanvasAction> {
        vec![
            Self::InsertChar(' '), // Example char
            Self::DeleteBackward,
            Self::DeleteForward,
        ]
    }

    /// Get all autocomplete-related actions
    pub fn autocomplete_actions() -> Vec<CanvasAction> {
        vec![
            Self::TriggerAutocomplete,
            Self::SuggestionUp,
            Self::SuggestionDown,
            Self::SelectSuggestion,
            Self::ExitSuggestions,
        ]
    }

    /// Check if this action modifies text content
    pub fn is_editing_action(&self) -> bool {
        matches!(self,
            Self::InsertChar(_) |
            Self::DeleteBackward |
            Self::DeleteForward
        )
    }

    /// Check if this action moves the cursor
    pub fn is_movement_action(&self) -> bool {
        matches!(self,
            Self::MoveLeft | Self::MoveRight | Self::MoveUp | Self::MoveDown |
            Self::MoveWordNext | Self::MoveWordPrev | Self::MoveWordEnd | Self::MoveWordEndPrev |
            Self::MoveLineStart | Self::MoveLineEnd | Self::NextField | Self::PrevField |
            Self::MoveFirstLine | Self::MoveLastLine
        )
    }
}
