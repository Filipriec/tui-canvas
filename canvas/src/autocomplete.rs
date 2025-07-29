// canvas/src/autocomplete.rs

/// Generic suggestion item that clients push to canvas
#[derive(Debug, Clone)]
pub struct SuggestionItem<T> {
    /// The underlying data (client-specific, e.g., Hit, String, etc.)
    pub data: T,
    /// Text to display in the dropdown
    pub display_text: String,
    /// Value to store in the form field when selected
    pub value_to_store: String,
}

impl<T> SuggestionItem<T> {
    pub fn new(data: T, display_text: String, value_to_store: String) -> Self {
        Self {
            data,
            display_text,
            value_to_store,
        }
    }
    
    /// Convenience constructor for simple string suggestions
    pub fn simple(data: T, text: String) -> Self {
        Self {
            data,
            display_text: text.clone(),
            value_to_store: text,
        }
    }
}

/// Autocomplete state managed by canvas
#[derive(Debug, Clone)]
pub struct AutocompleteState<T> {
    /// Whether autocomplete is currently active/visible
    pub is_active: bool,
    /// Whether suggestions are being loaded (for spinner/loading indicator)
    pub is_loading: bool,
    /// Current suggestions to display
    pub suggestions: Vec<SuggestionItem<T>>,
    /// Currently selected suggestion index
    pub selected_index: Option<usize>,
    /// Field index that triggered autocomplete (for context)
    pub active_field: Option<usize>,
}

impl<T> Default for AutocompleteState<T> {
    fn default() -> Self {
        Self {
            is_active: false,
            is_loading: false,
            suggestions: Vec::new(),
            selected_index: None,
            active_field: None,
        }
    }
}

impl<T> AutocompleteState<T> {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Activate autocomplete for a specific field
    pub fn activate(&mut self, field_index: usize) {
        self.is_active = true;
        self.active_field = Some(field_index);
        self.selected_index = None;
        self.suggestions.clear();
        self.is_loading = true;
    }
    
    /// Deactivate autocomplete and clear state
    pub fn deactivate(&mut self) {
        self.is_active = false;
        self.is_loading = false;
        self.suggestions.clear();
        self.selected_index = None;
        self.active_field = None;
    }
    
    /// Set suggestions and stop loading
    pub fn set_suggestions(&mut self, suggestions: Vec<SuggestionItem<T>>) {
        self.suggestions = suggestions;
        self.is_loading = false;
        self.selected_index = if self.suggestions.is_empty() { 
            None 
        } else { 
            Some(0) 
        };
    }
    
    /// Move selection down
    pub fn select_next(&mut self) {
        if !self.suggestions.is_empty() {
            let current = self.selected_index.unwrap_or(0);
            self.selected_index = Some((current + 1) % self.suggestions.len());
        }
    }
    
    /// Move selection up
    pub fn select_previous(&mut self) {
        if !self.suggestions.is_empty() {
            let current = self.selected_index.unwrap_or(0);
            self.selected_index = Some(
                if current == 0 { 
                    self.suggestions.len() - 1 
                } else { 
                    current - 1 
                }
            );
        }
    }
    
    /// Get currently selected suggestion
    pub fn get_selected(&self) -> Option<&SuggestionItem<T>> {
        self.selected_index
            .and_then(|idx| self.suggestions.get(idx))
    }
    
    /// Check if autocomplete is ready for interaction (active and has suggestions)
    pub fn is_ready(&self) -> bool {
        self.is_active && !self.suggestions.is_empty() && !self.is_loading
    }
}
