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
    
    // Autocomplete state
    pub(crate) autocomplete: AutocompleteUIState,
    
    // Selection state (for vim visual mode)
    pub(crate) selection: SelectionState,
}

#[derive(Debug, Clone)]
pub struct AutocompleteUIState {
    pub(crate) is_active: bool,
    pub(crate) is_loading: bool,
    pub(crate) selected_index: Option<usize>,
    pub(crate) active_field: Option<usize>,
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
            autocomplete: AutocompleteUIState {
                is_active: false,
                is_loading: false,
                selected_index: None,
                active_field: None,
            },
            selection: SelectionState::None,
        }
    }
    
    // ===================================================================
    // READ-ONLY ACCESS: User can fetch UI state for compatibility
    // ===================================================================
    
    /// Get current field index (for user's business logic)
    pub fn current_field(&self) -> usize {
        self.current_field
    }
    
    /// Get current cursor position (for user's business logic)  
    pub fn cursor_position(&self) -> usize {
        self.cursor_pos
    }

    /// Get ideal cursor column (for vim-like behavior)
    pub fn ideal_cursor_column(&self) -> usize {  // ADD THIS
        self.ideal_cursor_column
    }
    
    /// Get current mode (for user's business logic)
    pub fn mode(&self) -> AppMode {
        self.current_mode
    }
    
    /// Check if autocomplete is active (for user's business logic)
    pub fn is_autocomplete_active(&self) -> bool {
        self.autocomplete.is_active
    }
    
    /// Check if autocomplete is loading (for user's business logic)
    pub fn is_autocomplete_loading(&self) -> bool {
        self.autocomplete.is_loading
    }
    
    /// Get selection state (for user's business logic)
    pub fn selection_state(&self) -> &SelectionState {
        &self.selection
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
    
    pub(crate) fn set_cursor(&mut self, position: usize, max_position: usize, for_edit_mode: bool) {
        if for_edit_mode {
            // Edit mode: can go past end for insertion
            self.cursor_pos = position.min(max_position);
        } else {
            // ReadOnly/Highlight: stay within text bounds
            self.cursor_pos = position.min(max_position.saturating_sub(1));
        }
        self.ideal_cursor_column = self.cursor_pos;
    }
    
    pub(crate) fn activate_autocomplete(&mut self, field_index: usize) {
        self.autocomplete.is_active = true;
        self.autocomplete.is_loading = true;
        self.autocomplete.active_field = Some(field_index);
        self.autocomplete.selected_index = None;
    }
    
    pub(crate) fn deactivate_autocomplete(&mut self) {
        self.autocomplete.is_active = false;
        self.autocomplete.is_loading = false;
        self.autocomplete.active_field = None;
        self.autocomplete.selected_index = None;
    }
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new()
    }
}
