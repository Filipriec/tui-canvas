// src/editor/features/suggestions.rs

use crate::editor::FormEditor;
use crate::{DataProvider, SuggestionItem, SuggestionQuery};

impl<D: DataProvider> FormEditor<D> {
    #[cfg(feature = "suggestions")]
    fn current_suggestion_query(&self) -> Option<SuggestionQuery> {
        let idx = self.current_field();
        if !self.data_provider.supports_suggestions(idx) {
            return None;
        }

        self.data_provider
            .suggestion_query(idx, self.cursor_position())
    }

    #[cfg(feature = "suggestions")]
    fn set_active_suggestion_query(&mut self, field_index: usize, query: &SuggestionQuery) {
        self.ui_state.open_suggestions(field_index);
        self.ui_state.suggestions.active_query = Some(query.query.clone());
        self.ui_state.suggestions.replace_range = query.replace_range;
        self.suggestions.clear();
        self.ui_state.suggestions.selected_index = None;
        self.ui_state.suggestions.completion_text = None;
    }

    #[cfg(feature = "suggestions")]
    fn fetch_and_open_suggestions(&mut self) -> bool {
        let Some((idx, query)) = self.trigger_suggestions() else {
            return false;
        };

        let items = self.data_provider.fetch_suggestions_sync(idx, &query);
        if items.is_empty() {
            self.dismiss_suggestions();
            return false;
        }

        self.apply_suggestions(items);
        true
    }

    /// Compute inline completion for current selection and text
    fn compute_current_completion(&self) -> Option<String> {
        let typed = self
            .ui_state
            .suggestions
            .active_query
            .as_deref()
            .unwrap_or_else(|| self.current_text());
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
        self.ui_state.suggestions.completion_text = self.compute_current_completion();
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
        let query = self.current_suggestion_query()?;
        let query_text = query.query.clone();
        self.set_active_suggestion_query(idx, &query);
        Some((idx, query_text))
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
        let Some(query) = self.current_suggestion_query() else {
            if self.ui_state.suggestions.is_active {
                self.dismiss_suggestions();
            }
            return;
        };
        let query_text = query.query.as_str();
        let should_show = match trigger {
            crate::SuggestionTrigger::None => false,
            // WhenFieldStarts: show when in edit mode (empty shows all, typed filters)
            crate::SuggestionTrigger::WhenFieldStarts => true,
            crate::SuggestionTrigger::SpecialChar(ch) => query_text.starts_with(ch),
        };

        if should_show {
            let items = self.data_provider.fetch_suggestions_sync(idx, query_text);
            if items.is_empty() {
                if self.ui_state.suggestions.is_active {
                    self.dismiss_suggestions();
                }
            } else {
                if !self.ui_state.suggestions.is_active {
                    self.set_active_suggestion_query(idx, &query);
                } else {
                    self.ui_state.suggestions.active_query = Some(query.query.clone());
                    self.ui_state.suggestions.replace_range = query.replace_range;
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
        if !self.ui_state.suggestions.is_active || self.suggestions.is_empty() {
            let _ = self.fetch_and_open_suggestions();
            return;
        }

        let current = self.ui_state.suggestions.selected_index.unwrap_or(0);
        let next = (current + 1) % self.suggestions.len();
        self.ui_state.suggestions.selected_index = Some(next);
        self.update_inline_completion();
    }

    pub fn suggestions_prev(&mut self) {
        if !self.ui_state.suggestions.is_active || self.suggestions.is_empty() {
            if self.fetch_and_open_suggestions() {
                self.ui_state.suggestions.selected_index =
                    Some(self.suggestions.len().saturating_sub(1));
                self.update_inline_completion();
            }
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
            if let Some(suggestion) = self.suggestions.get(selected_index).cloned() {
                let field_index = self.ui_state.current_field;
                let query = SuggestionQuery {
                    query: self
                        .ui_state
                        .suggestions
                        .active_query
                        .clone()
                        .unwrap_or_else(|| self.current_text().to_string()),
                    replace_range: self.ui_state.suggestions.replace_range,
                };
                self.record_checkpoint(crate::editor::features::history::EditKind::Other);

                let cursor = self.data_provider.accept_suggestion(
                    field_index,
                    self.cursor_position(),
                    &suggestion,
                    &query,
                );

                self.set_cursor_raw(cursor);

                self.dismiss_suggestions();
                self.suggestions.clear();

                #[cfg(feature = "validation")]
                {
                    let text = self.data_provider.field_value(field_index).to_string();
                    let _ = self
                        .ui_state
                        .validation
                        .validate_field_content(field_index, &text);
                }

                return Some(suggestion.display_text);
            }
        }
        None
    }
}
