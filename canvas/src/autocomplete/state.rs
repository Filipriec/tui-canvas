// src/autocomplete/state.rs

use crate::canvas::state::CanvasState;
use async_trait::async_trait;

/// OPTIONAL extension trait for states that want rich autocomplete functionality.
/// Only implement this if you need the new autocomplete features.
///
/// # User Workflow:
/// 1. User presses trigger key (Tab, Ctrl+K, etc.)
/// 2. User's key mapping calls CanvasAction::TriggerAutocomplete
/// 3. Library calls your trigger_autocomplete_suggestions() method
/// 4. You implement async fetching logic in that method
/// 5. You call set_autocomplete_suggestions() with results
/// 6. Library manages UI state and navigation
#[async_trait]
pub trait AutocompleteCanvasState: CanvasState {
    /// Associated type for suggestion data (e.g., Hit, String, CustomType)
    type SuggestionData: Clone + Send + 'static;

    /// Check if a field supports autocomplete (user decides which fields)
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

    // === PUBLIC API METHODS (called by library) ===

    /// Activate autocomplete for current field (shows loading spinner)
    fn activate_autocomplete(&mut self) {
        let current_field = self.current_field();
        if let Some(state) = self.autocomplete_state_mut() {
            state.activate(current_field);
        }
    }

    /// Deactivate autocomplete (hides dropdown)
    fn deactivate_autocomplete(&mut self) {
        if let Some(state) = self.autocomplete_state_mut() {
            state.deactivate();
        }
    }

    /// Set suggestions (called after your async fetch completes)
    fn set_autocomplete_suggestions(&mut self, suggestions: Vec<crate::autocomplete::SuggestionItem<Self::SuggestionData>>) {
        if let Some(state) = self.autocomplete_state_mut() {
            state.set_suggestions(suggestions);
        }
    }

    /// Set loading state (show/hide spinner)
    fn set_autocomplete_loading(&mut self, loading: bool) {
        if let Some(state) = self.autocomplete_state_mut() {
            state.is_loading = loading;
        }
    }

    // === QUERY METHODS ===

    /// Check if autocomplete is currently active/visible
    fn is_autocomplete_active(&self) -> bool {
        self.autocomplete_state()
            .map(|state| state.is_active)
            .unwrap_or(false)
    }

    /// Check if autocomplete has suggestions ready for navigation
    fn is_autocomplete_ready(&self) -> bool {
        self.autocomplete_state()
            .map(|state| state.is_ready())
            .unwrap_or(false)
    }

    /// Check if there are available suggestions
    fn has_autocomplete_suggestions(&self) -> bool {
        self.autocomplete_state()
            .map(|state| !state.suggestions.is_empty())
            .unwrap_or(false)
    }

    // === USER-IMPLEMENTABLE METHODS ===

    /// Check if autocomplete should be triggered automatically (e.g., after typing 2+ chars)
    /// Override this to implement your own trigger logic
    fn should_trigger_autocomplete(&self) -> bool {
        let current_input = self.get_current_input();
        let current_field = self.current_field();

        self.supports_autocomplete(current_field) &&
        current_input.len() >= 2 && // Default: trigger after 2 chars
        !self.is_autocomplete_active()
    }

    /// **USER MUST IMPLEMENT**: Trigger autocomplete suggestions (async)
    /// This is where you implement your API calls, caching, etc.
    ///
    /// # Example Implementation:
    /// ```rust
    /// #[async_trait]
    /// impl AutocompleteCanvasState for MyState {
    ///     type SuggestionData = MyData;
    ///     
    ///     async fn trigger_autocomplete_suggestions(&mut self) {
    ///         self.activate_autocomplete(); // Show loading state
    ///
    ///         let query = self.get_current_input().to_string();
    ///         let suggestions = my_api.search(&query).await.unwrap_or_default();
    ///
    ///         self.set_autocomplete_suggestions(suggestions);
    ///     }
    /// }
    /// ```
    async fn trigger_autocomplete_suggestions(&mut self) {
        // Activate autocomplete UI
        self.activate_autocomplete();

        // Default: just show loading state
        // User should override this to do actual async fetching
        self.set_autocomplete_loading(true);

        // In a real implementation, you'd:
        // 1. Get current input: let query = self.get_current_input();
        // 2. Make API call: let results = api.search(query).await;
        // 3. Convert to suggestions: let suggestions = results.into_suggestions();
        // 4. Set suggestions: self.set_autocomplete_suggestions(suggestions);
    }

    // === INTERNAL NAVIGATION METHODS (called by library actions) ===

    /// Clear autocomplete suggestions and hide dropdown
    fn clear_autocomplete_suggestions(&mut self) {
        self.deactivate_autocomplete();
    }

    /// Move selection up/down in suggestions list
    fn move_suggestion_selection(&mut self, direction: i32) {
        if let Some(state) = self.autocomplete_state_mut() {
            if direction > 0 {
                state.select_next();
            } else {
                state.select_previous();
            }
        }
    }

    /// Get currently selected suggestion for display/application
    fn get_selected_suggestion(&self) -> Option<crate::autocomplete::SuggestionItem<Self::SuggestionData>> {
        self.autocomplete_state()?
            .get_selected()
            .cloned()
    }

    /// Apply the selected suggestion to the current field
    fn apply_suggestion(&mut self, suggestion: &crate::autocomplete::SuggestionItem<Self::SuggestionData>) {
        // Apply the value to current field
        *self.get_current_input_mut() = suggestion.value_to_store.clone();
        self.set_has_unsaved_changes(true);

        // Clear autocomplete
        self.clear_autocomplete_suggestions();
    }

    /// Apply the currently selected suggestion (convenience method)
    fn apply_selected_suggestion(&mut self) -> Option<String> {
        if let Some(suggestion) = self.get_selected_suggestion() {
            let display_text = suggestion.display_text.clone();
            self.apply_suggestion(&suggestion);
            Some(format!("Applied: {}", display_text))
        } else {
            None
        }
    }

    // === LEGACY COMPATIBILITY ===

    /// INTERNAL: Apply selected autocomplete value to current field (legacy method)
    fn apply_autocomplete_selection(&mut self) -> Option<String> {
        self.apply_selected_suggestion()
    }
}
