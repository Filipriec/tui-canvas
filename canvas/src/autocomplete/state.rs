// canvas/src/state.rs

use crate::canvas::state::CanvasState;

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
