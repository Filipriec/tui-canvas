// src/editor/suggestions.rs

use crate::editor::FormEditor;
use crate::{DataProvider, SuggestionItem};

impl<D: DataProvider> FormEditor<D> {
    /// Compute inline completion for current selection and text
    fn compute_current_completion(&self) -> Option<String> {
        let typed = self.current_text();
        let idx = self.ui_state.suggestions.selected_index?;
        let sugg = self.suggestions.get(idx)?;
        if let Some(rest) = sugg.value_to_store.strip_prefix(typed) {
            if !rest.is_empty() {
                return Some(rest.to_string());
            }
        }
        None
    }

    /// Update UI state's completion text from current selection
    pub fn update_inline_completion(&mut self) {
        self.ui_state.suggestions.completion_text =
            self.compute_current_completion();
    }

    /// Open the suggestions UI for `field_index`
    pub fn open_suggestions(&mut self, field_index: usize) {
        self.ui_state.open_suggestions(field_index);
    }

    /// Close suggestions UI and clear current suggestion results
    pub fn close_suggestions(&mut self) {
        self.ui_state.close_suggestions();
        self.suggestions.clear();
    }

    /// Handle Escape key in ReadOnly mode (closes suggestions if active)
    pub fn handle_escape_readonly(&mut self) {
        if self.ui_state.suggestions.is_active {
            self.close_suggestions();
        }
    }

    // ----------------- Non-blocking suggestions API --------------------

    #[cfg(feature = "suggestions")]
    pub fn start_suggestions(&mut self, field_index: usize) -> Option<String> {
        if !self.data_provider.supports_suggestions(field_index) {
            return None;
        }

        let query = self.current_text().to_string();
        self.ui_state.open_suggestions(field_index);
        self.ui_state.suggestions.is_loading = true;
        self.ui_state.suggestions.active_query = Some(query.clone());
        self.suggestions.clear();
        Some(query)
    }

    #[cfg(not(feature = "suggestions"))]
    pub fn start_suggestions(&mut self, _field_index: usize) -> Option<String> {
        None
    }

    #[cfg(feature = "suggestions")]
    pub fn apply_suggestions_result(
        &mut self,
        field_index: usize,
        query: &str,
        results: Vec<SuggestionItem>,
    ) -> bool {
        if self.ui_state.suggestions.active_field != Some(field_index) {
            return false;
        }
        if self.ui_state.suggestions.active_query.as_deref() != Some(query) {
            return false;
        }

        self.ui_state.suggestions.is_loading = false;
        self.suggestions = results;

        if !self.suggestions.is_empty() {
            self.ui_state.suggestions.selected_index = Some(0);
            self.update_inline_completion();
        } else {
            self.ui_state.suggestions.selected_index = None;
            self.ui_state.suggestions.completion_text = None;
        }
        true
    }

    #[cfg(not(feature = "suggestions"))]
    pub fn apply_suggestions_result(
        &mut self,
        _field_index: usize,
        _query: &str,
        _results: Vec<SuggestionItem>,
    ) -> bool {
        false
    }

    #[cfg(feature = "suggestions")]
    pub fn pending_suggestions_query(&self) -> Option<(usize, String)> {
        if self.ui_state.suggestions.is_loading {
            if let (Some(field), Some(query)) = (
                self.ui_state.suggestions.active_field,
                &self.ui_state.suggestions.active_query,
            ) {
                return Some((field, query.clone()));
            }
        }
        None
    }

    #[cfg(not(feature = "suggestions"))]
    pub fn pending_suggestions_query(&self) -> Option<(usize, String)> {
        None
    }

    pub fn cancel_suggestions(&mut self) {
        self.close_suggestions();
    }

    pub fn suggestions_next(&mut self) {
        if !self.ui_state.suggestions.is_active || self.suggestions.is_empty()
        {
            return;
        }

        let current = self.ui_state.suggestions.selected_index.unwrap_or(0);
        let next = (current + 1) % self.suggestions.len();
        self.ui_state.suggestions.selected_index = Some(next);
        self.update_inline_completion();
    }

    pub fn apply_suggestion(&mut self) -> Option<String> {
        if let Some(selected_index) = self.ui_state.suggestions.selected_index {
            if let Some(suggestion) = self.suggestions.get(selected_index).cloned()
            {
                let field_index = self.ui_state.current_field;

                self.data_provider.set_field_value(
                    field_index,
                    suggestion.value_to_store.clone(),
                );

                self.ui_state.cursor_pos = suggestion.value_to_store.len();
                self.ui_state.ideal_cursor_column = self.ui_state.cursor_pos;

                self.close_suggestions();
                self.suggestions.clear();

                #[cfg(feature = "validation")]
                {
                    let _ = self.ui_state.validation.validate_field_content(
                        field_index,
                        &suggestion.value_to_store,
                    );
                }

                return Some(suggestion.display_text);
            }
        }
        None
    }
}
