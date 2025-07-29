// canvas/src/actions/types.rs

use crossterm::event::KeyCode;

/// All possible canvas actions, type-safe and exhaustive
#[derive(Debug, Clone, PartialEq, Eq)]
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
    
    // Suggestions
    SuggestionUp,
    SuggestionDown,
    SelectSuggestion,
    ExitSuggestions,
    
    // Custom actions (escape hatch for feature-specific behavior)
    Custom(String),
}

impl CanvasAction {
    /// Convert a string action to typed action (for backwards compatibility during migration)
    pub fn from_string(action: &str) -> Self {
        match action {
            "insert_char" => {
                // This is a bit tricky - we need the char from context
                // For now, we'll use Custom until we refactor the call sites
                Self::Custom(action.to_string())
            }
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
            "suggestion_up" => Self::SuggestionUp,
            "suggestion_down" => Self::SuggestionDown,
            "select_suggestion" => Self::SelectSuggestion,
            "exit_suggestions" => Self::ExitSuggestions,
            _ => Self::Custom(action.to_string()),
        }
    }
    
    /// Get string representation (for logging, debugging)
    pub fn as_str(&self) -> &str {
        match self {
            Self::InsertChar(_) => "insert_char",
            Self::DeleteBackward => "delete_char_backward",
            Self::DeleteForward => "delete_char_forward",
            Self::MoveLeft => "move_left",
            Self::MoveRight => "move_right",
            Self::MoveUp => "move_up",
            Self::MoveDown => "move_down",
            Self::MoveLineStart => "move_line_start",
            Self::MoveLineEnd => "move_line_end",
            Self::MoveFirstLine => "move_first_line",
            Self::MoveLastLine => "move_last_line",
            Self::MoveWordNext => "move_word_next",
            Self::MoveWordEnd => "move_word_end",
            Self::MoveWordPrev => "move_word_prev",
            Self::MoveWordEndPrev => "move_word_end_prev",
            Self::NextField => "next_field",
            Self::PrevField => "prev_field",
            Self::SuggestionUp => "suggestion_up",
            Self::SuggestionDown => "suggestion_down",
            Self::SelectSuggestion => "select_suggestion",
            Self::ExitSuggestions => "exit_suggestions",
            Self::Custom(s) => s,
        }
    }
    
    /// Create action from KeyCode for common cases
    pub fn from_key(key: KeyCode) -> Option<Self> {
        match key {
            KeyCode::Char(c) => Some(Self::InsertChar(c)),
            KeyCode::Backspace => Some(Self::DeleteBackward),
            KeyCode::Delete => Some(Self::DeleteForward),
            KeyCode::Left => Some(Self::MoveLeft),
            KeyCode::Right => Some(Self::MoveRight),
            KeyCode::Up => Some(Self::MoveUp),
            KeyCode::Down => Some(Self::MoveDown),
            KeyCode::Home => Some(Self::MoveLineStart),
            KeyCode::End => Some(Self::MoveLineEnd),
            KeyCode::Tab => Some(Self::NextField),
            KeyCode::BackTab => Some(Self::PrevField),
            _ => None,
        }
    }
    
    /// Check if this action modifies content
    pub fn is_modifying(&self) -> bool {
        matches!(self, 
            Self::InsertChar(_) | 
            Self::DeleteBackward | 
            Self::DeleteForward |
            Self::SelectSuggestion
        )
    }
    
    /// Check if this action moves the cursor
    pub fn is_movement(&self) -> bool {
        matches!(self,
            Self::MoveLeft | Self::MoveRight | Self::MoveUp | Self::MoveDown |
            Self::MoveLineStart | Self::MoveLineEnd | Self::MoveFirstLine | Self::MoveLastLine |
            Self::MoveWordNext | Self::MoveWordEnd | Self::MoveWordPrev | Self::MoveWordEndPrev |
            Self::NextField | Self::PrevField
        )
    }
    
    /// Check if this is a suggestion-related action
    pub fn is_suggestion(&self) -> bool {
        matches!(self,
            Self::SuggestionUp | Self::SuggestionDown | 
            Self::SelectSuggestion | Self::ExitSuggestions
        )
    }
}

/// Result of executing a canvas action
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionResult {
    /// Action completed successfully, optional message for user feedback
    Success(Option<String>),
    /// Action was handled by custom feature logic
    HandledByFeature(String),
    /// Action requires additional context or cannot be performed
    RequiresContext(String),
    /// Action failed with error message
    Error(String),
}

impl ActionResult {
    pub fn success() -> Self {
        Self::Success(None)
    }
    
    pub fn success_with_message(msg: impl Into<String>) -> Self {
        Self::Success(Some(msg.into()))
    }
    
    pub fn error(msg: impl Into<String>) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_action_from_string() {
        assert_eq!(CanvasAction::from_string("move_left"), CanvasAction::MoveLeft);
        assert_eq!(CanvasAction::from_string("delete_char_backward"), CanvasAction::DeleteBackward);
        assert_eq!(CanvasAction::from_string("unknown"), CanvasAction::Custom("unknown".to_string()));
    }
    
    #[test]
    fn test_action_from_key() {
        assert_eq!(CanvasAction::from_key(KeyCode::Char('a')), Some(CanvasAction::InsertChar('a')));
        assert_eq!(CanvasAction::from_key(KeyCode::Left), Some(CanvasAction::MoveLeft));
        assert_eq!(CanvasAction::from_key(KeyCode::Backspace), Some(CanvasAction::DeleteBackward));
        assert_eq!(CanvasAction::from_key(KeyCode::F(1)), None);
    }
    
    #[test]
    fn test_action_properties() {
        assert!(CanvasAction::InsertChar('a').is_modifying());
        assert!(!CanvasAction::MoveLeft.is_modifying());
        
        assert!(CanvasAction::MoveLeft.is_movement());
        assert!(!CanvasAction::InsertChar('a').is_movement());
        
        assert!(CanvasAction::SuggestionUp.is_suggestion());
        assert!(!CanvasAction::MoveLeft.is_suggestion());
    }
}
