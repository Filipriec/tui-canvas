// canvas/src/suggestions.rs

/// Generic suggestion system that can be implemented by any CanvasState
#[derive(Debug, Clone)]
pub struct SuggestionState {
    pub suggestions: Vec<String>,
    pub selected_index: Option<usize>,
    pub is_active: bool,
    pub trigger_chars: Vec<char>, // Characters that trigger suggestions
}

impl Default for SuggestionState {
    fn default() -> Self {
        Self {
            suggestions: Vec::new(),
            selected_index: None,
            is_active: false,
            trigger_chars: vec![], // No auto-trigger by default
        }
    }
}

impl SuggestionState {
    pub fn new(trigger_chars: Vec<char>) -> Self {
        Self {
            trigger_chars,
            ..Default::default()
        }
    }
    
    pub fn activate_with_suggestions(&mut self, suggestions: Vec<String>) {
        self.suggestions = suggestions;
        self.is_active = !self.suggestions.is_empty();
        self.selected_index = if self.is_active { Some(0) } else { None };
    }
    
    pub fn deactivate(&mut self) {
        self.suggestions.clear();
        self.selected_index = None;
        self.is_active = false;
    }
    
    pub fn select_next(&mut self) {
        if !self.suggestions.is_empty() {
            let current = self.selected_index.unwrap_or(0);
            self.selected_index = Some((current + 1) % self.suggestions.len());
        }
    }
    
    pub fn select_previous(&mut self) {
        if !self.suggestions.is_empty() {
            let current = self.selected_index.unwrap_or(0);
            self.selected_index = Some(
                if current == 0 { self.suggestions.len() - 1 } else { current - 1 }
            );
        }
    }
    
    pub fn get_selected(&self) -> Option<&String> {
        self.selected_index
            .and_then(|idx| self.suggestions.get(idx))
    }
    
    pub fn should_trigger(&self, c: char) -> bool {
        self.trigger_chars.contains(&c)
    }
}
