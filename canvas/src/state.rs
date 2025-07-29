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

    // --- LEGACY AUTOCOMPLETE SUPPORT (for backwards compatibility) ---

    /// Legacy suggestion support (deprecated - use AutocompleteCanvasState for rich features)
    fn get_suggestions(&self) -> Option<&[String]> {
        None
    }

    /// Legacy selected suggestion index (deprecated)
    fn get_selected_suggestion_index(&self) -> Option<usize> {
        None
    }

    /// Legacy suggestion index setter (deprecated)
    fn set_selected_suggestion_index(&mut self, _index: Option<usize>) {
        // Default: no-op
    }

    /// Legacy activate suggestions (deprecated)
    fn activate_suggestions(&mut self, _suggestions: Vec<String>) {
        // Default: no-op
    }

    /// Legacy deactivate suggestions (deprecated)
    fn deactivate_suggestions(&mut self) {
        // Default: no-op
    }

    // --- Feature-specific action handling ---

    /// Feature-specific action handling (NEW: Type-safe)
    fn handle_feature_action(&mut self, _action: &CanvasAction, _context: &ActionContext) -> Option<String> {
        None // Default: no feature-specific handling
    }

    /// Legacy string-based action handling (for backwards compatibility)
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

/// OPTIONAL extension trait for states that want rich autocomplete functionality.
/// Only implement this if you need the new autocomplete features.
pub trait AutocompleteCanvasState: CanvasState {
    /// Associated type for suggestion data (e.g., Hit, String, CustomType)
    type SuggestionData: Clone + Send + 'static;

    /// Check if a field supports autocomplete
    fn supports_autocomplete(&self, _field_index: usize) -> bool {
        false // Default: no autocomplete support
    }

    /// Get autocomplete state (read-only)
    fn autocomplete_state(&self) -> Option<&crate::autocomplete::AutocompleteState<Self::SuggestionData>> {
        None // Default: no autocomplete state
    }

    /// Get autocomplete state (mutable)
    fn autocomplete_state_mut(&mut self) -> Option<&mut crate::autocomplete::AutocompleteState<Self::SuggestionData>> {
        None // Default: no autocomplete state
    }

    /// CLIENT API: Activate autocomplete for current field
    fn activate_autocomplete(&mut self) {
        let current_field = self.current_field(); // Get field first
        if let Some(state) = self.autocomplete_state_mut() {
            state.activate(current_field); // Then use it
        }
    }

    /// CLIENT API: Deactivate autocomplete
    fn deactivate_autocomplete(&mut self) {
        if let Some(state) = self.autocomplete_state_mut() {
            state.deactivate();
        }
    }

    /// CLIENT API: Set suggestions (called after async fetch completes)
    fn set_autocomplete_suggestions(&mut self, suggestions: Vec<crate::autocomplete::SuggestionItem<Self::SuggestionData>>) {
        if let Some(state) = self.autocomplete_state_mut() {
            state.set_suggestions(suggestions);
        }
    }

    /// CLIENT API: Set loading state
    fn set_autocomplete_loading(&mut self, loading: bool) {
        if let Some(state) = self.autocomplete_state_mut() {
            state.is_loading = loading;
        }
    }

    /// Check if autocomplete is currently active
    fn is_autocomplete_active(&self) -> bool {
        self.autocomplete_state()
            .map(|state| state.is_active)
            .unwrap_or(false)
    }

    /// Check if autocomplete is ready for interaction
    fn is_autocomplete_ready(&self) -> bool {
        self.autocomplete_state()
            .map(|state| state.is_ready())
            .unwrap_or(false)
    }

    /// INTERNAL: Apply selected autocomplete value to current field
    fn apply_autocomplete_selection(&mut self) -> Option<String> {
        // First, get the selected value and display text (if any)
        let selection_info = if let Some(state) = self.autocomplete_state() {
            state.get_selected().map(|selected| {
                (selected.value_to_store.clone(), selected.display_text.clone())
            })
        } else {
            None
        };

        // Apply the selection if we have one
        if let Some((value, display)) = selection_info {
            // Apply the value to current field
            *self.get_current_input_mut() = value;
            self.set_has_unsaved_changes(true);

            // Deactivate autocomplete
            if let Some(state_mut) = self.autocomplete_state_mut() {
                state_mut.deactivate();
            }

            Some(format!("Selected: {}", display))
        } else {
            None
        }
    }
}
