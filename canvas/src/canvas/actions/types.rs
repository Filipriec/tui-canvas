// src/canvas/actions/types.rs

#[derive(Debug, Clone, PartialEq)]
pub enum CanvasAction {
    // Character input
    InsertChar(char),

    // Deletion
    DeleteBackward,
    DeleteForward,

    // Basic cursor movement
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,

    // Line movement
    MoveLineStart,
    MoveLineEnd,
    MoveFirstLine,
    MoveLastLine,

    // Word movement
    MoveWordNext,
    MoveWordEnd,
    MoveWordPrev,
    MoveWordEndPrev,

    // Field navigation
    NextField,
    PrevField,

    // Autocomplete actions
    TriggerAutocomplete,
    SuggestionUp,
    SuggestionDown,
    SelectSuggestion,
    ExitSuggestions,

    // Custom actions
    Custom(String),
}

impl CanvasAction {
    /// Convert string action name to CanvasAction enum (config-driven)
    pub fn from_string(action: &str) -> Self {
        match action {
            "delete_char_backward" => Self::DeleteBackward,
            "delete_char_forward" => Self::DeleteForward,
            "move_left" => Self::MoveLeft,
            "move_right" => Self::MoveRight,
            "move_up" => Self::MoveUp,
            "move_down" => Self::MoveDown,
            "move_line_start" => Self::MoveLineStart,
            "move_line_end" => Self::MoveLineEnd,
            "move_first_line" => Self::MoveFirstLine,
            "move_last_line" => Self::MoveLastLine,
            "move_word_next" => Self::MoveWordNext,
            "move_word_end" => Self::MoveWordEnd,
            "move_word_prev" => Self::MoveWordPrev,
            "move_word_end_prev" => Self::MoveWordEndPrev,
            "next_field" => Self::NextField,
            "prev_field" => Self::PrevField,
            "trigger_autocomplete" => Self::TriggerAutocomplete,
            "suggestion_up" => Self::SuggestionUp,
            "suggestion_down" => Self::SuggestionDown,
            "select_suggestion" => Self::SelectSuggestion,
            "exit_suggestions" => Self::ExitSuggestions,
            _ => Self::Custom(action.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActionResult {
    Success(Option<String>),
    HandledByFeature(String),
    RequiresContext(String),
    Error(String),
}

impl ActionResult {
    pub fn success() -> Self {
        Self::Success(None)
    }

    pub fn success_with_message(msg: &str) -> Self {
        Self::Success(Some(msg.to_string()))
    }

    pub fn error(msg: &str) -> Self {
        Self::Error(msg.into())
    }

    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success(_) | Self::HandledByFeature(_))
    }

    pub fn message(&self) -> Option<&str> {
        match self {
            Self::Success(msg) => msg.as_deref(),
            Self::HandledByFeature(msg) => Some(msg),
            Self::RequiresContext(msg) => Some(msg),
            Self::Error(msg) => Some(msg),
        }
    }
}
