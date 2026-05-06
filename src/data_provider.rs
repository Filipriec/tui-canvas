// src/data_provider.rs
//! Simplified user interface - only business data, no UI state

/// Defines when suggestions should be shown for a field
#[cfg(feature = "suggestions")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestionTrigger {
    /// No suggestions for this field
    None,
    /// Show suggestions when field starts (becomes non-empty)
    WhenFieldStarts,
    /// Show suggestions when field starts with this special character
    SpecialChar(char),
}

#[cfg(feature = "suggestions")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuggestionQuery {
    pub query: String,
    pub replace_range: Option<(usize, usize)>,
}

#[cfg(feature = "suggestions")]
impl SuggestionQuery {
    pub fn whole_field(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            replace_range: None,
        }
    }

    pub fn with_replace_range(query: impl Into<String>, replace_range: (usize, usize)) -> Self {
        Self {
            query: query.into(),
            replace_range: Some(replace_range),
        }
    }
}

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

    /// Check if field supports suggestions (optional)
    fn supports_suggestions(&self, _field_index: usize) -> bool {
        false
    }

    /// When should suggestions be triggered for a field? (optional)
    /// Only used when suggestions feature is enabled
    #[cfg(feature = "suggestions")]
    fn suggestion_trigger(&self, _field_index: usize) -> SuggestionTrigger {
        SuggestionTrigger::None
    }

    /// Build the active suggestion query for the current field/cursor.
    ///
    /// Default behavior uses the whole current field value and replaces the
    /// whole field on accept. More advanced editors can return a token-local
    /// query and replace range.
    #[cfg(feature = "suggestions")]
    fn suggestion_query(&self, field_index: usize, _cursor_char: usize) -> Option<SuggestionQuery> {
        Some(SuggestionQuery::whole_field(self.field_value(field_index)))
    }

    /// Fetch suggestions synchronously (for auto-trigger feature)
    /// Returns empty vec by default. Override to enable auto-trigger.
    #[cfg(feature = "suggestions")]
    fn fetch_suggestions_sync(&self, _field_index: usize, _query: &str) -> Vec<SuggestionItem> {
        Vec::new()
    }

    /// Apply the selected suggestion to the underlying field content and
    /// return the desired cursor character position after insertion.
    #[cfg(feature = "suggestions")]
    fn accept_suggestion(
        &mut self,
        field_index: usize,
        _cursor_char: usize,
        suggestion: &SuggestionItem,
        _query: &SuggestionQuery,
    ) -> usize {
        let value = suggestion.value_to_store.clone();
        let cursor = value.chars().count();
        self.set_field_value(field_index, value);
        cursor
    }

    /// Get display value (for password masking, etc.) - optional
    fn display_value(&self, _index: usize) -> Option<&str> {
        None // Default: use actual value
    }

    /// Get validation configuration for a field (optional)
    /// Only available when the 'validation' feature is enabled
    #[cfg(feature = "validation")]
    fn validation_config(
        &self,
        _field_index: usize,
    ) -> Option<crate::validation::ValidationConfig> {
        None
    }

    /// Check if field is computed (display-only, skip in navigation)
    /// Default: not computed
    #[cfg(feature = "computed")]
    fn is_computed_field(&self, _field_index: usize) -> bool {
        false
    }

    /// Get computed field value if this is a computed field.
    /// Returns None for regular fields. Default: not computed.
    #[cfg(feature = "computed")]
    fn computed_field_value(&self, _field_index: usize) -> Option<String> {
        None
    }
}

#[cfg(feature = "suggestions")]
#[derive(Debug, Clone)]
pub struct SuggestionItem {
    pub display_text: String,
    pub value_to_store: String,
}

#[cfg(feature = "suggestions")]
impl SuggestionItem {
    pub fn new(display: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            display_text: display.into(),
            value_to_store: value.into(),
        }
    }
}
