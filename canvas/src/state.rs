// canvas/src/state.rs

use crate::actions::CanvasAction;

/// Context passed to feature-specific action handlers
#[derive(Debug)]
pub struct ActionContext {
    pub key_code: Option<crossterm::event::KeyCode>, // Kept for backwards compatibility
    pub ideal_cursor_column: usize,
    pub current_input: String,
    pub current_field: usize,
}

/// Core trait that any form-like state must implement to work with the canvas system.
/// This enables the same mode behaviors (edit, read-only, highlight) to work across
/// any implementation - login forms, data entry forms, configuration screens, etc.
pub trait CanvasState {
    // --- Core Navigation ---
    fn current_field(&self) -> usize;
    fn current_cursor_pos(&self) -> usize;
    fn set_current_field(&mut self, index: usize);
    fn set_current_cursor_pos(&mut self, pos: usize);

    // --- Data Access ---
    fn get_current_input(&self) -> &str;
    fn get_current_input_mut(&mut self) -> &mut String;
    fn inputs(&self) -> Vec<&String>;
    fn fields(&self) -> Vec<&str>;

    // --- State Management ---
    fn has_unsaved_changes(&self) -> bool;
    fn set_has_unsaved_changes(&mut self, changed: bool);

    // --- Autocomplete/Suggestions (Optional) ---
    fn get_suggestions(&self) -> Option<&[String]> {
        None
    }
    fn get_selected_suggestion_index(&self) -> Option<usize> {
        None
    }
    fn set_selected_suggestion_index(&mut self, _index: Option<usize>) {
        // Default: no-op (override if you support suggestions)
    }
    fn activate_suggestions(&mut self, _suggestions: Vec<String>) {
        // Default: no-op (override if you support suggestions)
    }
    fn deactivate_suggestions(&mut self) {
        // Default: no-op (override if you support suggestions)
    }

    // --- Feature-specific action handling (NEW: Type-safe) ---
    fn handle_feature_action(&mut self, _action: &CanvasAction, _context: &ActionContext) -> Option<String> {
        None // Default: no feature-specific handling
    }

    // --- Legacy string-based action handling (for backwards compatibility) ---
    fn handle_feature_action_legacy(&mut self, action: &str, context: &ActionContext) -> Option<String> {
        // Convert string to typed action and delegate
        let typed_action = match action {
            "insert_char" => {
                // This is tricky - we need the char from the KeyCode in context
                if let Some(crossterm::event::KeyCode::Char(c)) = context.key_code {
                    CanvasAction::InsertChar(c)
                } else {
                    CanvasAction::Custom(action.to_string())
                }
            }
            _ => CanvasAction::from_string(action),
        };
        self.handle_feature_action(&typed_action, context)
    }

    // --- Display Overrides (for links, computed values, etc.) ---
    fn get_display_value_for_field(&self, index: usize) -> &str {
        self.inputs()
            .get(index)
            .map(|s| s.as_str())
            .unwrap_or("")
    }
    fn has_display_override(&self, _index: usize) -> bool {
        false
    }
}
