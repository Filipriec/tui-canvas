// src/data_provider.rs
//! Simplified user interface - only business data, no UI state

use anyhow::Result;
use async_trait::async_trait;

/// User implements this - only business data, no UI state
pub trait DataProvider {
    /// How many fields in the form
    fn field_count(&self) -> usize;

    /// Get field label/name
    fn field_name(&self, index: usize) -> &str;

    /// Get field value
    fn field_value(&self, index: usize) -> &str;

    /// Set field value (library calls this when text changes)
    fn set_field_value(&mut self, index: usize, value: String);

    /// Check if field supports autocomplete (optional)
    fn supports_autocomplete(&self, _field_index: usize) -> bool {
        false
    }

    /// Get display value (for password masking, etc.) - optional
    fn display_value(&self, _index: usize) -> Option<&str> {
        None // Default: use actual value
    }
    
    /// Get validation configuration for a field (optional)
    /// Only available when the 'validation' feature is enabled
    #[cfg(feature = "validation")]
    fn validation_config(&self, _field_index: usize) -> Option<crate::validation::ValidationConfig> {
        None
    }
}

/// Optional: User implements this for autocomplete data
#[async_trait]
pub trait AutocompleteProvider {
    /// Fetch autocomplete suggestions (user's business logic)
    async fn fetch_suggestions(&mut self, field_index: usize, query: &str)
        -> Result<Vec<SuggestionItem>>;
}

#[derive(Debug, Clone)]
pub struct SuggestionItem {
    pub display_text: String,
    pub value_to_store: String,
}
