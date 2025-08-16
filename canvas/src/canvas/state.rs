// src/canvas/state.rs
//! Library-owned UI state - user never directly modifies this

use crate::canvas::modes::AppMode;

/// Library-owned UI state - user never directly modifies this
#[derive(Debug, Clone)]
pub struct EditorState {
    // Navigation state
    pub(crate) current_field: usize,
    pub(crate) cursor_pos: usize,
    pub(crate) ideal_cursor_column: usize,

    // Mode state
    pub(crate) current_mode: AppMode,

    // Suggestions dropdown state (only available with suggestions feature)
    #[cfg(feature = "suggestions")]
    pub(crate) suggestions: SuggestionsUIState,

    // Selection state (for vim visual mode)
    pub(crate) selection: SelectionState,

    // Validation state (only available with validation feature)
    #[cfg(feature = "validation")]
    pub(crate) validation: crate::validation::ValidationState,

    /// Computed fields state (only when computed feature is enabled)
    #[cfg(feature = "computed")]
    pub(crate) computed: Option<crate::computed::ComputedState>,
}

#[cfg(feature = "suggestions")]
#[derive(Debug, Clone)]
pub struct SuggestionsUIState {
    pub(crate) is_active: bool,
    pub(crate) is_loading: bool,
    pub(crate) selected_index: Option<usize>,
    pub(crate) active_field: Option<usize>,
    pub(crate) active_query: Option<String>,
    pub(crate) completion_text: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SelectionState {
    None,
    Characterwise { anchor: (usize, usize) },
    Linewise { anchor_field: usize },
}

impl EditorState {
    pub fn new() -> Self {
        Self {
            current_field: 0,
            cursor_pos: 0,
            ideal_cursor_column: 0,
            current_mode: AppMode::Edit,
            #[cfg(feature = "suggestions")]
            suggestions: SuggestionsUIState {
                is_active: false,
                is_loading: false,
                selected_index: None,
                active_field: None,
                active_query: None,
                completion_text: None,
            },
            selection: SelectionState::None,
            #[cfg(feature = "validation")]
            validation: crate::validation::ValidationState::new(),
            #[cfg(feature = "computed")]
            computed: None,
        }
    }

    // ===================================================================
    // READ-ONLY ACCESS: User can fetch UI state for compatibility
    // ===================================================================

    /// Get current field index (for user's business logic)
    pub fn current_field(&self) -> usize {
        self.current_field
    }

    /// Check if field is computed
    #[cfg(feature = "computed")]
    pub fn is_computed_field(&self, field_index: usize) -> bool {
        self.computed
            .as_ref()
            .map(|state| state.is_computed_field(field_index))
            .unwrap_or(false)
    }

    /// Get current cursor position (for user's business logic)
    pub fn cursor_position(&self) -> usize {
        self.cursor_pos
    }

    /// Get ideal cursor column (for vim-like behavior)
    pub fn ideal_cursor_column(&self) -> usize {
        self.ideal_cursor_column
    }

    /// Get current mode (for user's business logic)
    pub fn mode(&self) -> AppMode {
        self.current_mode
    }

    /// Check if suggestions dropdown is active (for user's business logic)
    #[cfg(feature = "suggestions")]
    pub fn is_suggestions_active(&self) -> bool {
        self.suggestions.is_active
    }

    /// Check if suggestions dropdown is loading (for user's business logic)
    #[cfg(feature = "suggestions")]
    pub fn is_suggestions_loading(&self) -> bool {
        self.suggestions.is_loading
    }

    /// Get selection state (for user's business logic)
    pub fn selection_state(&self) -> &SelectionState {
        &self.selection
    }

    /// Get validation state (for user's business logic)
    /// Only available when the 'validation' feature is enabled
    #[cfg(feature = "validation")]
    pub fn validation_state(&self) -> &crate::validation::ValidationState {
        &self.validation
    }

    // ===================================================================
    // INTERNAL MUTATIONS: Only library modifies these
    // ===================================================================

    pub(crate) fn move_to_field(&mut self, field_index: usize, field_count: usize) {
        if field_index < field_count {
            self.current_field = field_index;
            // Reset cursor to safe position - will be clamped by movement logic
            self.cursor_pos = 0;
        }
    }

    pub(crate) fn set_cursor(
        &mut self,
        position: usize,
        max_position: usize,
        for_edit_mode: bool,
    ) {
        if for_edit_mode {
            // Edit mode: can go past end for insertion
            self.cursor_pos = position.min(max_position);
        } else {
            // ReadOnly/Highlight: stay within text bounds
            self.cursor_pos = position.min(max_position.saturating_sub(1));
        }
        self.ideal_cursor_column = self.cursor_pos;
    }

    /// Explicitly open suggestions — should only be called on Tab
    #[cfg(feature = "suggestions")]
    pub(crate) fn open_suggestions(&mut self, field_index: usize) {
        self.suggestions.is_active = true;
        self.suggestions.is_loading = true;
        self.suggestions.active_field = Some(field_index);
        self.suggestions.active_query = None;
        self.suggestions.selected_index = None;
        self.suggestions.completion_text = None;
    }

    /// Explicitly close suggestions — should be called on Esc or field change
    #[cfg(feature = "suggestions")]
    pub(crate) fn close_suggestions(&mut self) {
        self.suggestions.is_active = false;
        self.suggestions.is_loading = false;
        self.suggestions.active_field = None;
        self.suggestions.active_query = None;
        self.suggestions.selected_index = None;
        self.suggestions.completion_text = None;
    }
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new()
    }
}
