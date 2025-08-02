// src/editor.rs
//! Main API for the canvas library - FormEditor with library-owned state

#[cfg(feature = "cursor-style")]
use crate::canvas::CursorManager;
#[cfg(feature = "cursor-style")]
use crossterm;

use anyhow::Result;
use crate::canvas::state::EditorState;
use crate::data_provider::{DataProvider, AutocompleteProvider, SuggestionItem};
use crate::canvas::modes::AppMode;
use crate::canvas::state::SelectionState;

/// Main editor that manages UI state internally and delegates data to user
pub struct FormEditor<D: DataProvider> {
    // Library owns all UI state
    ui_state: EditorState,

    // User owns business data
    data_provider: D,

    // Autocomplete suggestions (library manages UI, user provides data)
    pub(crate) suggestions: Vec<SuggestionItem>,
}

impl<D: DataProvider> FormEditor<D> {
    pub fn new(data_provider: D) -> Self {
        Self {
            ui_state: EditorState::new(),
            data_provider,
            suggestions: Vec::new(),
        }
    }

    // ===================================================================
    // READ-ONLY ACCESS: User can fetch UI state
    // ===================================================================

    /// Get current field index (for user's compatibility)
    pub fn current_field(&self) -> usize {
        self.ui_state.current_field()
    }

    /// Get current cursor position (for user's compatibility)
    pub fn cursor_position(&self) -> usize {
        self.ui_state.cursor_position()
    }

    /// Get current mode (for user's mode-dependent logic)
    pub fn mode(&self) -> AppMode {
        self.ui_state.mode()
    }

    /// Check if autocomplete is active (for user's logic)
    pub fn is_autocomplete_active(&self) -> bool {
        self.ui_state.is_autocomplete_active()
    }

    /// Get current field text (convenience method)
    pub fn current_text(&self) -> &str {
        let field_index = self.ui_state.current_field;
        if field_index < self.data_provider.field_count() {
            self.data_provider.field_value(field_index)
        } else {
            ""
        }
    }

    /// Get reference to UI state for rendering
    pub fn ui_state(&self) -> &EditorState {
        &self.ui_state
    }

    /// Get reference to data provider for rendering
    pub fn data_provider(&self) -> &D {
        &self.data_provider
    }

    /// Get autocomplete suggestions for rendering (read-only)
    pub fn suggestions(&self) -> &[SuggestionItem] {
        &self.suggestions
    }

    // ===================================================================
    // SYNC OPERATIONS: No async needed for basic editing
    // ===================================================================

    /// Handle character insertion
    pub fn insert_char(&mut self, ch: char) -> Result<()> {
        if self.ui_state.current_mode != AppMode::Edit {
            return Ok(()); // Ignore in non-edit modes
        }

        let field_index = self.ui_state.current_field;
        let cursor_pos = self.ui_state.cursor_pos;

        // Get current text from user
        let mut current_text = self.data_provider.field_value(field_index).to_string();

        // Insert character
        current_text.insert(cursor_pos, ch);

        // Update user's data
        self.data_provider.set_field_value(field_index, current_text);

        // Update library's UI state
        self.ui_state.cursor_pos += 1;
        self.ui_state.ideal_cursor_column = self.ui_state.cursor_pos;

        Ok(())
    }

    /// Handle cursor movement
    pub fn move_left(&mut self) {
        if self.ui_state.cursor_pos > 0 {
            self.ui_state.cursor_pos -= 1;
            self.ui_state.ideal_cursor_column = self.ui_state.cursor_pos;
        }
    }

    pub fn move_right(&mut self) {
        let current_text = self.current_text();
        let max_pos = if self.ui_state.current_mode == AppMode::Edit {
            current_text.len() // Edit mode: can go past end
        } else {
            current_text.len().saturating_sub(1) // ReadOnly: stay in bounds
        };

        if self.ui_state.cursor_pos < max_pos {
            self.ui_state.cursor_pos += 1;
            self.ui_state.ideal_cursor_column = self.ui_state.cursor_pos;
        }
    }

    /// Handle field navigation
    pub fn move_to_next_field(&mut self) {
        let field_count = self.data_provider.field_count();
        let next_field = (self.ui_state.current_field + 1) % field_count;
        self.ui_state.move_to_field(next_field, field_count);

        // Clamp cursor to new field
        let current_text = self.current_text();
        let max_pos = current_text.len();
        self.ui_state.set_cursor(
            self.ui_state.ideal_cursor_column,
            max_pos,
            self.ui_state.current_mode == AppMode::Edit
        );
    }

    /// Change mode (for vim compatibility)
    pub fn set_mode(&mut self, mode: AppMode) {
        match (self.ui_state.current_mode, mode) {
            // Entering highlight mode from read-only
            (AppMode::ReadOnly, AppMode::Highlight) => {
                self.enter_highlight_mode();
            }
            // Exiting highlight mode
            (AppMode::Highlight, AppMode::ReadOnly) => {
                self.exit_highlight_mode();
            }
            // Other transitions
            (_, new_mode) => {
                self.ui_state.current_mode = new_mode;
                if new_mode != AppMode::Highlight {
                    self.ui_state.selection = SelectionState::None;
                }
                
                #[cfg(feature = "cursor-style")]
                {
                    let _ = CursorManager::update_for_mode(new_mode);
                }
            }
        }
    }

    /// Enter edit mode with cursor positioned for append (vim 'a' command)
    pub fn enter_append_mode(&mut self) {
        let current_text = self.current_text();
        
        // Calculate append position: always move right, even at line end
        let append_pos = if current_text.is_empty() {
            0
        } else {
            (self.ui_state.cursor_pos + 1).min(current_text.len())
        };
        
        // Set cursor position for append
        self.ui_state.cursor_pos = append_pos;
        self.ui_state.ideal_cursor_column = append_pos;
        
        // Enter edit mode (which will update cursor style)
        self.set_mode(AppMode::Edit);
    }

    // ===================================================================
    // ASYNC OPERATIONS: Only autocomplete needs async
    // ===================================================================

    /// Trigger autocomplete (async because it fetches data)
    pub async fn trigger_autocomplete<A>(&mut self, provider: &mut A) -> Result<()>
    where
        A: AutocompleteProvider,
    {
        let field_index = self.ui_state.current_field;

        if !self.data_provider.supports_autocomplete(field_index) {
            return Ok(());
        }

        // Activate autocomplete UI
        self.ui_state.activate_autocomplete(field_index);

        // Fetch suggestions from user (no conversion needed!)
        let query = self.current_text();
        self.suggestions = provider.fetch_suggestions(field_index, query).await?;

        // Update UI state
        self.ui_state.autocomplete.is_loading = false;
        if !self.suggestions.is_empty() {
            self.ui_state.autocomplete.selected_index = Some(0);
        }

        Ok(())
    }

    /// Navigate autocomplete suggestions
    pub fn autocomplete_next(&mut self) {
        if !self.ui_state.autocomplete.is_active || self.suggestions.is_empty() {
            return;
        }

        let current = self.ui_state.autocomplete.selected_index.unwrap_or(0);
        let next = (current + 1) % self.suggestions.len();
        self.ui_state.autocomplete.selected_index = Some(next);
    }

    /// Apply selected autocomplete suggestion
    pub fn apply_autocomplete(&mut self) -> Option<String> {
        if let Some(selected_index) = self.ui_state.autocomplete.selected_index {
            if let Some(suggestion) = self.suggestions.get(selected_index).cloned() {
                let field_index = self.ui_state.current_field;

                // Apply to user's data
                self.data_provider.set_field_value(
                    field_index,
                    suggestion.value_to_store.clone()
                );

                // Update cursor position
                self.ui_state.cursor_pos = suggestion.value_to_store.len();
                self.ui_state.ideal_cursor_column = self.ui_state.cursor_pos;

                // Close autocomplete
                self.ui_state.deactivate_autocomplete();
                self.suggestions.clear();

                return Some(suggestion.display_text);
            }
        }
        None
    }

    // ===================================================================
    // ADD THESE MISSING MOVEMENT METHODS
    // ===================================================================

    /// Move to previous field (vim k / up arrow)
    pub fn move_up(&mut self) {
        let field_count = self.data_provider.field_count();
        if field_count == 0 {
            return;
        }
        
        let current_field = self.ui_state.current_field;
        let new_field = current_field.saturating_sub(1);
        
        self.ui_state.move_to_field(new_field, field_count);
        self.clamp_cursor_to_current_field();
    }

    /// Move to next field (vim j / down arrow) 
    pub fn move_down(&mut self) {
        let field_count = self.data_provider.field_count();
        if field_count == 0 {
            return;
        }
        
        let current_field = self.ui_state.current_field;
        let new_field = (current_field + 1).min(field_count - 1);
        
        self.ui_state.move_to_field(new_field, field_count);
        self.clamp_cursor_to_current_field();
    }

    /// Move to first field (vim gg)
    pub fn move_first_line(&mut self) {
        let field_count = self.data_provider.field_count();
        if field_count == 0 {
            return;
        }
        
        self.ui_state.move_to_field(0, field_count);
        self.clamp_cursor_to_current_field();
    }

    /// Move to last field (vim G)
    pub fn move_last_line(&mut self) {
        let field_count = self.data_provider.field_count();
        if field_count == 0 {
            return;
        }
        
        let last_field = field_count - 1;
        self.ui_state.move_to_field(last_field, field_count);
        self.clamp_cursor_to_current_field();
    }

    /// Move to previous field (alternative to move_up)
    pub fn prev_field(&mut self) {
        self.move_up();
    }

    /// Move to next field (alternative to move_down) 
    pub fn next_field(&mut self) {
        self.move_down();
    }

    /// Move to start of current field (vim 0)
    pub fn move_line_start(&mut self) {
        use crate::canvas::actions::movement::line::line_start_position;
        let new_pos = line_start_position();
        self.ui_state.cursor_pos = new_pos;
        self.ui_state.ideal_cursor_column = new_pos;
    }

    /// Move to end of current field (vim $)
    pub fn move_line_end(&mut self) {
        use crate::canvas::actions::movement::line::line_end_position;
        let current_text = self.current_text();
        let is_edit_mode = self.ui_state.current_mode == AppMode::Edit;
        
        let new_pos = line_end_position(current_text, is_edit_mode);
        self.ui_state.cursor_pos = new_pos;
        self.ui_state.ideal_cursor_column = new_pos;
    }

    /// Move to start of next word (vim w)
    pub fn move_word_next(&mut self) {
        use crate::canvas::actions::movement::word::find_next_word_start;
        let current_text = self.current_text();
        
        if current_text.is_empty() {
            return;
        }
        
        let new_pos = find_next_word_start(current_text, self.ui_state.cursor_pos);
        let is_edit_mode = self.ui_state.current_mode == AppMode::Edit;
        
        // Clamp to valid bounds for current mode
        let final_pos = if is_edit_mode {
            new_pos.min(current_text.len())
        } else {
            new_pos.min(current_text.len().saturating_sub(1))
        };
        
        self.ui_state.cursor_pos = final_pos;
        self.ui_state.ideal_cursor_column = final_pos;
    }

    /// Move to start of previous word (vim b)
    pub fn move_word_prev(&mut self) {
        use crate::canvas::actions::movement::word::find_prev_word_start;
        let current_text = self.current_text();
        
        if current_text.is_empty() {
            return;
        }
        
        let new_pos = find_prev_word_start(current_text, self.ui_state.cursor_pos);
        self.ui_state.cursor_pos = new_pos;
        self.ui_state.ideal_cursor_column = new_pos;
    }

    /// Move to end of current/next word (vim e)
    pub fn move_word_end(&mut self) {
        use crate::canvas::actions::movement::word::find_word_end;
        let current_text = self.current_text();
        
        if current_text.is_empty() {
            return;
        }
        
        let current_pos = self.ui_state.cursor_pos;
        let new_pos = find_word_end(current_text, current_pos);
        
        // If we didn't move, try next word
        let final_pos = if new_pos == current_pos && current_pos + 1 < current_text.len() {
            find_word_end(current_text, current_pos + 1)
        } else {
            new_pos
        };
        
        // Clamp for read-only mode
        let is_edit_mode = self.ui_state.current_mode == AppMode::Edit;
        let clamped_pos = if is_edit_mode {
            final_pos.min(current_text.len())
        } else {
            final_pos.min(current_text.len().saturating_sub(1))
        };
        
        self.ui_state.cursor_pos = clamped_pos;
        self.ui_state.ideal_cursor_column = clamped_pos;
    }

    /// Move to end of previous word (vim ge)
    pub fn move_word_end_prev(&mut self) {
        use crate::canvas::actions::movement::word::find_prev_word_end;
        let current_text = self.current_text();
        
        if current_text.is_empty() {
            return;
        }
        
        let new_pos = find_prev_word_end(current_text, self.ui_state.cursor_pos);
        self.ui_state.cursor_pos = new_pos;
        self.ui_state.ideal_cursor_column = new_pos;
    }

    /// Delete character before cursor (vim x in insert mode / backspace)
    pub fn delete_backward(&mut self) -> Result<()> {
        if self.ui_state.current_mode != AppMode::Edit {
            return Ok(()); // Silently ignore in non-edit modes
        }
        
        if self.ui_state.cursor_pos == 0 {
            return Ok(()); // Nothing to delete
        }
        
        let field_index = self.ui_state.current_field;
        let mut current_text = self.data_provider.field_value(field_index).to_string();
        
        if self.ui_state.cursor_pos <= current_text.len() {
            current_text.remove(self.ui_state.cursor_pos - 1);
            self.data_provider.set_field_value(field_index, current_text);
            self.ui_state.cursor_pos -= 1;
            self.ui_state.ideal_cursor_column = self.ui_state.cursor_pos;
        }
        
        Ok(())
    }

    /// Delete character under cursor (vim x / delete key)
    pub fn delete_forward(&mut self) -> Result<()> {
        if self.ui_state.current_mode != AppMode::Edit {
            return Ok(()); // Silently ignore in non-edit modes
        }
        
        let field_index = self.ui_state.current_field;
        let mut current_text = self.data_provider.field_value(field_index).to_string();
        
        if self.ui_state.cursor_pos < current_text.len() {
            current_text.remove(self.ui_state.cursor_pos);
            self.data_provider.set_field_value(field_index, current_text);
        }
        
        Ok(())
    }

    /// Exit edit mode to read-only mode (vim Escape)
    // TODO this is still flickering, I have no clue how to fix it
    pub fn exit_edit_mode(&mut self) {
        // Adjust cursor position when transitioning from edit to normal mode
        let current_text = self.current_text();
        if !current_text.is_empty() {
            // In normal mode, cursor must be ON a character, not after the last one
            let max_normal_pos = current_text.len().saturating_sub(1);
            if self.ui_state.cursor_pos > max_normal_pos {
                self.ui_state.cursor_pos = max_normal_pos;
                self.ui_state.ideal_cursor_column = self.ui_state.cursor_pos;
            }
        }

        self.set_mode(AppMode::ReadOnly);
        // Deactivate autocomplete when exiting edit mode
        self.ui_state.deactivate_autocomplete();
    }

    /// Enter edit mode from read-only mode (vim i/a/o)
    pub fn enter_edit_mode(&mut self) {
        self.set_mode(AppMode::Edit);
    }

    // ===================================================================
    // HELPER METHODS
    // ===================================================================

    /// Clamp cursor position to valid bounds for current field and mode
    fn clamp_cursor_to_current_field(&mut self) {
        let current_text = self.current_text();
        let is_edit_mode = self.ui_state.current_mode == AppMode::Edit;
        
        use crate::canvas::actions::movement::line::safe_cursor_position;
        let safe_pos = safe_cursor_position(
            current_text, 
            self.ui_state.ideal_cursor_column, 
            is_edit_mode
        );
        
        self.ui_state.cursor_pos = safe_pos;
    }

  
    /// Set the value of the current field
    pub fn set_current_field_value(&mut self, value: String) {
        let field_index = self.ui_state.current_field;
        self.data_provider.set_field_value(field_index, value);
        // Reset cursor to start of field
        self.ui_state.cursor_pos = 0;
        self.ui_state.ideal_cursor_column = 0;
    }
    
    /// Set the value of a specific field by index
    pub fn set_field_value(&mut self, field_index: usize, value: String) {
        if field_index < self.data_provider.field_count() {
            self.data_provider.set_field_value(field_index, value);
            // If we're modifying the current field, reset cursor
            if field_index == self.ui_state.current_field {
                self.ui_state.cursor_pos = 0;
                self.ui_state.ideal_cursor_column = 0;
            }
        }
    }
    
    /// Clear the current field (set to empty string)
    pub fn clear_current_field(&mut self) {
        self.set_current_field_value(String::new());
    }
    
    /// Get mutable access to data provider (for advanced operations)
    pub fn data_provider_mut(&mut self) -> &mut D {
        &mut self.data_provider
    }

    /// Set cursor to exact position (for vim-style movements like f, F, t, T)
    pub fn set_cursor_position(&mut self, position: usize) {
        let current_text = self.current_text();
        let is_edit_mode = self.ui_state.current_mode == AppMode::Edit;
        
        // Clamp to valid bounds for current mode
        let max_pos = if is_edit_mode {
            current_text.len() // Edit mode: can go past end
        } else {
            current_text.len().saturating_sub(1).max(0) // Read-only: stay within text
        };
        
        let clamped_pos = position.min(max_pos);
        
        // Update cursor position directly
        self.ui_state.cursor_pos = clamped_pos;
        self.ui_state.ideal_cursor_column = clamped_pos;
    }

    /// Get cursor position for display (respects mode-specific positioning rules)
    pub fn display_cursor_position(&self) -> usize {
        let current_text = self.current_text();
        
        match self.ui_state.current_mode {
            AppMode::Edit => {
                // Edit mode: cursor can be past end of text
                self.ui_state.cursor_pos.min(current_text.len())
            }
            _ => {
                // Normal/other modes: cursor must be on a character
                if current_text.is_empty() {
                    0
                } else {
                    self.ui_state.cursor_pos.min(current_text.len().saturating_sub(1))
                }
            }
        }
    }

    /// Cleanup cursor style (call this when shutting down)
    pub fn cleanup_cursor(&self) -> std::io::Result<()> {
        #[cfg(feature = "cursor-style")]
        {
            crate::canvas::CursorManager::reset()
        }
        #[cfg(not(feature = "cursor-style"))]
        {
            Ok(())
        }
    }


    // ===================================================================
    // HIGHLIGHT MODE
    // ===================================================================

    /// Enter highlight mode (visual mode)
    pub fn enter_highlight_mode(&mut self) {
        if self.ui_state.current_mode == AppMode::ReadOnly {
            self.ui_state.current_mode = AppMode::Highlight;
            self.ui_state.selection = SelectionState::Characterwise {
                anchor: (self.ui_state.current_field, self.ui_state.cursor_pos),
            };
            
            #[cfg(feature = "cursor-style")]
            {
                let _ = CursorManager::update_for_mode(AppMode::Highlight);
            }
        }
    }

    /// Enter highlight line mode (visual line mode)
    pub fn enter_highlight_line_mode(&mut self) {
        if self.ui_state.current_mode == AppMode::ReadOnly {
            self.ui_state.current_mode = AppMode::Highlight;
            self.ui_state.selection = SelectionState::Linewise {
                anchor_field: self.ui_state.current_field,
            };
            
            #[cfg(feature = "cursor-style")]
            {
                let _ = CursorManager::update_for_mode(AppMode::Highlight);
            }
        }
    }

    /// Exit highlight mode back to read-only
    pub fn exit_highlight_mode(&mut self) {
        if self.ui_state.current_mode == AppMode::Highlight {
            self.ui_state.current_mode = AppMode::ReadOnly;
            self.ui_state.selection = SelectionState::None;
            
            #[cfg(feature = "cursor-style")]
            {
                let _ = CursorManager::update_for_mode(AppMode::ReadOnly);
            }
        }
    }

    /// Check if currently in highlight mode
    pub fn is_highlight_mode(&self) -> bool {
        self.ui_state.current_mode == AppMode::Highlight
    }

    /// Get current selection state
    pub fn selection_state(&self) -> &SelectionState {
        &self.ui_state.selection
    }

    /// Enhanced movement methods that update selection in highlight mode
    pub fn move_left_with_selection(&mut self) {
        self.move_left();
        // Selection anchor stays in place, cursor position updates automatically
    }

    pub fn move_right_with_selection(&mut self) {
        self.move_right();
        // Selection anchor stays in place, cursor position updates automatically
    }

    pub fn move_up_with_selection(&mut self) {
        self.move_up();
        // Selection anchor stays in place, cursor position updates automatically
    }

    pub fn move_down_with_selection(&mut self) {
        self.move_down();
        // Selection anchor stays in place, cursor position updates automatically
    }

    // Add similar methods for word movement, line movement, etc.
    pub fn move_word_next_with_selection(&mut self) {
        self.move_word_next();
    }

    pub fn move_word_prev_with_selection(&mut self) {
        self.move_word_prev();
    }

    pub fn move_line_start_with_selection(&mut self) {
        self.move_line_start();
    }

    pub fn move_line_end_with_selection(&mut self) {
        self.move_line_end();
    }
}

// Add Drop implementation for automatic cleanup
impl<D: DataProvider> Drop for FormEditor<D> {
    fn drop(&mut self) {
        // Reset cursor to default when FormEditor is dropped
        let _ = self.cleanup_cursor();
    }
}
