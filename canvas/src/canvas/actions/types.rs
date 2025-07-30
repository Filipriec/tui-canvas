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
    pub fn from_key(key: crossterm::event::KeyCode) -> Option<Self> {
        match key {
            crossterm::event::KeyCode::Char(c) => Some(Self::InsertChar(c)),
            crossterm::event::KeyCode::Backspace => Some(Self::DeleteBackward),
            crossterm::event::KeyCode::Delete => Some(Self::DeleteForward),
            crossterm::event::KeyCode::Left => Some(Self::MoveLeft),
            crossterm::event::KeyCode::Right => Some(Self::MoveRight),
            crossterm::event::KeyCode::Up => Some(Self::MoveUp),
            crossterm::event::KeyCode::Down => Some(Self::MoveDown),
            crossterm::event::KeyCode::Home => Some(Self::MoveLineStart),
            crossterm::event::KeyCode::End => Some(Self::MoveLineEnd),
            crossterm::event::KeyCode::Tab => Some(Self::NextField),
            crossterm::event::KeyCode::BackTab => Some(Self::PrevField),
            _ => None,
        }
    }

    // Backward compatibility method
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
