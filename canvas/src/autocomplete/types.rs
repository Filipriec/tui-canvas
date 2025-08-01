// src/autocomplete/types.rs
//! Legacy autocomplete types - deprecated

// Re-export the new simplified types
pub use crate::data_provider::SuggestionItem;

/// Legacy type - use FormEditor instead
#[deprecated(note = "Use FormEditor instead")]
#[derive(Debug, Clone)]
pub struct AutocompleteState<T> {
    _phantom: std::marker::PhantomData<T>,
}

#[allow(dead_code)]
impl<T> AutocompleteState<T> {
    /// Legacy method - use FormEditor.is_autocomplete_active() instead
    #[deprecated(note = "Use FormEditor.is_autocomplete_active() instead")]
    pub fn is_active(&self) -> bool {
        false
    }
}
