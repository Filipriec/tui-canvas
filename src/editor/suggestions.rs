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

    /// Trigger suggestions - opens UI and returns request info for client to fetch data.
    /// Client should fetch data and call apply_suggestions().
    /// Returns Some((field_index, query)) if suggestions can be triggered, None otherwise.
    #[cfg(feature = "suggestions")]
    pub fn trigger_suggestions(&mut self) -> Option<(usize, String)> {
        let idx = self.current_field();
        if !self.data_provider.supports_suggestions(idx) {
            return None;
        }

        let query = self.current_text().to_string();
        self.ui_state.open_suggestions(idx);
        self.ui_state.suggestions.active_query = Some(query.clone());
        self.suggestions.clear();
        self.ui_state.suggestions.selected_index = None;

        Some((idx, query))
    }

    /// Apply fetched suggestions from client - opens UI with the provided items.
    #[cfg(feature = "suggestions")]
    pub fn apply_suggestions(&mut self, items: Vec<SuggestionItem>) {
        self.ui_state.suggestions.is_loading = false;
        self.suggestions = items;

        if !self.suggestions.is_empty() {
            self.ui_state.suggestions.selected_index = Some(0);
            self.update_inline_completion();
        } else {
            self.ui_state.suggestions.selected_index = None;
            self.ui_state.suggestions.completion_text = None;
        }
    }

    /// Update suggestions with new query results - adjusts selection if needed.
    #[cfg(feature = "suggestions")]
    pub fn update_suggestions(&mut self, items: Vec<SuggestionItem>) {
        self.ui_state.suggestions.is_loading = false;
        self.suggestions = items;

        if !self.suggestions.is_empty() {
            // Keep selected_index if valid, else reset
            let current_idx = self.ui_state.suggestions.selected_index.unwrap_or(0);
            if current_idx >= self.suggestions.len() {
                self.ui_state.suggestions.selected_index = Some(0);
            }
            self.update_inline_completion();
        } else {
            self.ui_state.suggestions.selected_index = None;
            self.ui_state.suggestions.completion_text = None;
        }
    }

    /// Dismiss suggestions - closes UI and clears data.
    #[cfg(feature = "suggestions")]
    pub fn dismiss_suggestions(&mut self) {
        self.ui_state.close_suggestions();
        self.suggestions.clear();
        self.ui_state.suggestions.selected_index = None;
        self.ui_state.suggestions.completion_text = None;
    }

    /// Check suggestion trigger condition and update suggestions accordingly.
    /// This is called automatically when entering edit mode or changing text.
    #[cfg(feature = "suggestions")]
    pub fn check_suggestion_trigger(&mut self) {
        let idx = self.current_field();
        if !self.data_provider.supports_suggestions(idx) {
            if self.ui_state.suggestions.is_active {
                self.dismiss_suggestions();
            }
            return;
        }

        let trigger = self.data_provider.suggestion_trigger(idx);
        let current_text = self.current_text();
        let should_show = match trigger {
            crate::SuggestionTrigger::None => false,
            // WhenFieldStarts: show when in edit mode (empty shows all, typed filters)
            crate::SuggestionTrigger::WhenFieldStarts => true,
            crate::SuggestionTrigger::SpecialChar(ch) => current_text.starts_with(ch),
        };

        if should_show {
            let items = self.data_provider.fetch_suggestions_sync(idx, &current_text);
            if items.is_empty() {
                if self.ui_state.suggestions.is_active {
                    self.dismiss_suggestions();
                }
            } else {
                if !self.ui_state.suggestions.is_active {
                    let _ = self.trigger_suggestions();
                }
                self.apply_suggestions(items);
            }
        } else {
            if self.ui_state.suggestions.is_active {
                self.dismiss_suggestions();
            }
        }
    }

    /// Handle Escape key in ReadOnly mode (closes suggestions if active)
    pub fn handle_escape_readonly(&mut self) {
        if self.ui_state.suggestions.is_active {
            self.dismiss_suggestions();
        }
    }

    pub fn cancel_suggestions(&mut self) {
        self.dismiss_suggestions();
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

    pub fn suggestions_prev(&mut self) {
        if !self.ui_state.suggestions.is_active || self.suggestions.is_empty() {
            return;
        }

        let current = self.ui_state.suggestions.selected_index.unwrap_or(0);
        let prev = if current == 0 {
            self.suggestions.len() - 1
        } else {
            current - 1
        };
        self.ui_state.suggestions.selected_index = Some(prev);
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

                self.set_cursor_raw(suggestion.value_to_store.chars().count());

                self.dismiss_suggestions();
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
