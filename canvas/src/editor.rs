// src/editor.rs
//! Main API for the canvas library - FormEditor with library-owned state

use anyhow::Result;
use crate::canvas::state::EditorState;
use crate::data_provider::{DataProvider, AutocompleteProvider, SuggestionItem};
use crate::canvas::modes::AppMode;

/// Main editor that manages UI state internally and delegates data to user
pub struct FormEditor<D: DataProvider> {
    // Library owns all UI state
    ui_state: EditorState,
    
    // User owns business data
    data_provider: D,
    
    // Autocomplete suggestions (library manages UI, user provides data)
    pub(crate) suggestions: Vec<SuggestionItem<String>>,
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
    pub fn suggestions(&self) -> &[SuggestionItem<String>] {
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
        self.ui_state.current_mode = mode;
        
        // Clear autocomplete when changing modes
        if mode != AppMode::Edit {
            self.ui_state.deactivate_autocomplete();
        }
    }
    
    // ===================================================================
    // ASYNC OPERATIONS: Only autocomplete needs async
    // ===================================================================
    
    /// Trigger autocomplete (async because it fetches data)
    pub async fn trigger_autocomplete<A>(&mut self, provider: &mut A) -> Result<()> 
    where 
        A: AutocompleteProvider,
        A::SuggestionData: std::fmt::Debug,  // Change from Display to Debug
    {
        let field_index = self.ui_state.current_field;
        
        if !self.data_provider.supports_autocomplete(field_index) {
            return Ok(());
        }
        
        // Activate autocomplete UI
        self.ui_state.activate_autocomplete(field_index);
        
        // Fetch suggestions from user
        let query = self.current_text();
        let suggestions = provider.fetch_suggestions(field_index, query).await?;
        
        // Convert to library's format (could be avoided with better generics)
        self.suggestions = suggestions.into_iter()
            .map(|item| SuggestionItem {
                data: format!("{:?}", item.data), // Use Debug formatting instead
                display_text: item.display_text,
                value_to_store: item.value_to_store,
            })
            .collect();
        
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
}
