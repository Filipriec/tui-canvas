// examples/basic_usage.rs
//! Basic usage patterns and quick start guide
//!
//! This example demonstrates the core patterns for using the canvas crate:
//! 1. Implementing CanvasState
//! 2. Using the ActionDispatcher
//! 3. Handling different types of actions
//! 4. Working with suggestions
//!
//! Run with: cargo run --example basic_usage

use canvas::prelude::*;

#[tokio::main]
async fn main() {
    println!("🎨 Canvas Crate - Basic Usage Patterns");
    println!("=====================================\n");

    // Example 1: Minimal form implementation
    example_1_minimal_form();

    // Example 2: Form with suggestions
    example_2_with_suggestions();

    // Example 3: Custom actions
    example_3_custom_actions().await;

    // Example 4: Batch operations
    example_4_batch_operations().await;
}

// Example 1: Minimal form - just the required methods
fn example_1_minimal_form() {
    println!("📝 Example 1: Minimal Form Implementation");

    #[derive(Debug)]
    struct SimpleForm {
        current_field: usize,
        cursor_pos: usize,
        name: String,
        email: String,
        has_changes: bool,
    }

    impl SimpleForm {
        fn new() -> Self {
            Self {
                current_field: 0,
                cursor_pos: 0,
                name: String::new(),
                email: String::new(),
                has_changes: false,
            }
        }
    }

    impl CanvasState for SimpleForm {
        fn current_field(&self) -> usize { self.current_field }
        fn current_cursor_pos(&self) -> usize { self.cursor_pos }
        fn set_current_field(&mut self, index: usize) { self.current_field = index.min(1); }
        fn set_current_cursor_pos(&mut self, pos: usize) { self.cursor_pos = pos; }

        fn get_current_input(&self) -> &str {
            match self.current_field {
                0 => &self.name,
                1 => &self.email,
                _ => "",
            }
        }

        fn get_current_input_mut(&mut self) -> &mut String {
            match self.current_field {
                0 => &mut self.name,
                1 => &mut self.email,
                _ => unreachable!(),
            }
        }

        fn inputs(&self) -> Vec<&String> { vec![&self.name, &self.email] }
        fn fields(&self) -> Vec<&str> { vec!["Name", "Email"] }
        fn has_unsaved_changes(&self) -> bool { self.has_changes }
        fn set_has_unsaved_changes(&mut self, changed: bool) { self.has_changes = changed; }
    }

    let form = SimpleForm::new();
    println!("  Created form with {} fields", form.fields().len());
    println!("  Current field: {}", form.fields()[form.current_field()]);
    println!("  ✅ Minimal implementation works!\n");
}

// Example 2: Form with suggestion support
fn example_2_with_suggestions() {
    println!("💡 Example 2: Form with Suggestions");

    #[derive(Debug)]
    struct FormWithSuggestions {
        current_field: usize,
        cursor_pos: usize,
        country: String,
        has_changes: bool,
        suggestions: SuggestionState,
    }

    impl FormWithSuggestions {
        fn new() -> Self {
            Self {
                current_field: 0,
                cursor_pos: 0,
                country: String::new(),
                has_changes: false,
                suggestions: SuggestionState::default(),
            }
        }
    }

    impl CanvasState for FormWithSuggestions {
        fn current_field(&self) -> usize { self.current_field }
        fn current_cursor_pos(&self) -> usize { self.cursor_pos }
        fn set_current_field(&mut self, index: usize) { self.current_field = index; }
        fn set_current_cursor_pos(&mut self, pos: usize) { self.cursor_pos = pos; }

        fn get_current_input(&self) -> &str { &self.country }
        fn get_current_input_mut(&mut self) -> &mut String { &mut self.country }
        fn inputs(&self) -> Vec<&String> { vec![&self.country] }
        fn fields(&self) -> Vec<&str> { vec!["Country"] }
        fn has_unsaved_changes(&self) -> bool { self.has_changes }
        fn set_has_unsaved_changes(&mut self, changed: bool) { self.has_changes = changed; }

        // Suggestion support
        fn get_suggestions(&self) -> Option<&[String]> {
            if self.suggestions.is_active {
                Some(&self.suggestions.suggestions)
            } else {
                None
            }
        }

        fn get_selected_suggestion_index(&self) -> Option<usize> {
            self.suggestions.selected_index
        }

        fn set_selected_suggestion_index(&mut self, index: Option<usize>) {
            self.suggestions.selected_index = index;
        }

        fn activate_suggestions(&mut self, suggestions: Vec<String>) {
            self.suggestions.activate_with_suggestions(suggestions);
        }

        fn deactivate_suggestions(&mut self) {
            self.suggestions.deactivate();
        }

        fn handle_feature_action(&mut self, action: &CanvasAction, _context: &ActionContext) -> Option<String> {
            match action {
                CanvasAction::SelectSuggestion => {
                    // Fix: Clone the suggestion first to avoid borrow checker issues
                    if let Some(suggestion) = self.suggestions.get_selected().cloned() {
                        self.country = suggestion.clone();
                        self.cursor_pos = suggestion.len();
                        self.deactivate_suggestions();
                        self.has_changes = true;
                        return Some(format!("Selected: {}", suggestion));
                    }
                    None
                }
                _ => None,
            }
        }
    }

    let mut form = FormWithSuggestions::new();

    // Simulate user typing and triggering suggestions
    form.activate_suggestions(vec![
        "United States".to_string(),
        "United Kingdom".to_string(),
        "Ukraine".to_string(),
    ]);

    println!("  Activated suggestions: {:?}", form.get_suggestions().unwrap());
    println!("  Current selection: {:?}", form.get_selected_suggestion_index());

    // Navigate suggestions
    form.set_selected_suggestion_index(Some(1));
    println!("  Navigated to: {}", form.suggestions.get_selected().unwrap());
    println!("  ✅ Suggestions work!\n");
}

// Example 3: Custom actions
async fn example_3_custom_actions() {
    println!("⚡ Example 3: Custom Actions");

    #[derive(Debug)]
    struct FormWithCustomActions {
        current_field: usize,
        cursor_pos: usize,
        text: String,
        has_changes: bool,
    }

    impl FormWithCustomActions {
        fn new() -> Self {
            Self {
                current_field: 0,
                cursor_pos: 0,
                text: "hello world".to_string(),
                has_changes: false,
            }
        }
    }

    impl CanvasState for FormWithCustomActions {
        fn current_field(&self) -> usize { self.current_field }
        fn current_cursor_pos(&self) -> usize { self.cursor_pos }
        fn set_current_field(&mut self, index: usize) { self.current_field = index; }
        fn set_current_cursor_pos(&mut self, pos: usize) { self.cursor_pos = pos; }

        fn get_current_input(&self) -> &str { &self.text }
        fn get_current_input_mut(&mut self) -> &mut String { &mut self.text }
        fn inputs(&self) -> Vec<&String> { vec![&self.text] }
        fn fields(&self) -> Vec<&str> { vec!["Text"] }
        fn has_unsaved_changes(&self) -> bool { self.has_changes }
        fn set_has_unsaved_changes(&mut self, changed: bool) { self.has_changes = changed; }

        fn handle_feature_action(&mut self, action: &CanvasAction, _context: &ActionContext) -> Option<String> {
            match action {
                CanvasAction::Custom(cmd) => match cmd.as_str() {
                    "uppercase" => {
                        self.text = self.text.to_uppercase();
                        self.has_changes = true;
                        Some("Converted to uppercase".to_string())
                    }
                    "reverse" => {
                        self.text = self.text.chars().rev().collect();
                        self.has_changes = true;
                        Some("Reversed text".to_string())
                    }
                    "word_count" => {
                        let count = self.text.split_whitespace().count();
                        Some(format!("Word count: {}", count))
                    }
                    _ => None,
                },
                _ => None,
            }
        }
    }

    let mut form = FormWithCustomActions::new();
    let mut ideal_cursor = 0;

    println!("  Initial text: '{}'", form.text);

    // Execute custom actions
    let result = ActionDispatcher::dispatch(
        CanvasAction::Custom("uppercase".to_string()),
        &mut form,
        &mut ideal_cursor,
    ).await.unwrap();

    println!("  After uppercase: '{}' - {}", form.text, result.message().unwrap());

    let result = ActionDispatcher::dispatch(
        CanvasAction::Custom("reverse".to_string()),
        &mut form,
        &mut ideal_cursor,
    ).await.unwrap();

    println!("  After reverse: '{}' - {}", form.text, result.message().unwrap());

    let result = ActionDispatcher::dispatch(
        CanvasAction::Custom("word_count".to_string()),
        &mut form,
        &mut ideal_cursor,
    ).await.unwrap();

    println!("  {}", result.message().unwrap());
    println!("  ✅ Custom actions work!\n");
}

// Example 4: Batch operations
async fn example_4_batch_operations() {
    println!("📦 Example 4: Batch Operations");

    // Reuse the simple form from example 1
    #[derive(Debug)]
    struct SimpleForm {
        current_field: usize,
        cursor_pos: usize,
        name: String,
        email: String,
        has_changes: bool,
    }

    impl SimpleForm {
        fn new() -> Self {
            Self {
                current_field: 0,
                cursor_pos: 0,
                name: String::new(),
                email: String::new(),
                has_changes: false,
            }
        }
    }

    impl CanvasState for SimpleForm {
        fn current_field(&self) -> usize { self.current_field }
        fn current_cursor_pos(&self) -> usize { self.cursor_pos }
        fn set_current_field(&mut self, index: usize) { self.current_field = index.min(1); }
        fn set_current_cursor_pos(&mut self, pos: usize) { self.cursor_pos = pos; }

        fn get_current_input(&self) -> &str {
            match self.current_field {
                0 => &self.name,
                1 => &self.email,
                _ => "",
            }
        }

        fn get_current_input_mut(&mut self) -> &mut String {
            match self.current_field {
                0 => &mut self.name,
                1 => &mut self.email,
                _ => unreachable!(),
            }
        }

        fn inputs(&self) -> Vec<&String> { vec![&self.name, &self.email] }
        fn fields(&self) -> Vec<&str> { vec!["Name", "Email"] }
        fn has_unsaved_changes(&self) -> bool { self.has_changes }
        fn set_has_unsaved_changes(&mut self, changed: bool) { self.has_changes = changed; }
    }

    let mut form = SimpleForm::new();
    let mut ideal_cursor = 0;

    // Execute a sequence of actions to type "John" in the name field
    let actions = vec![
        CanvasAction::InsertChar('J'),
        CanvasAction::InsertChar('o'),
        CanvasAction::InsertChar('h'),
        CanvasAction::InsertChar('n'),
        CanvasAction::NextField, // Move to email field
        CanvasAction::InsertChar('j'),
        CanvasAction::InsertChar('o'),
        CanvasAction::InsertChar('h'),
        CanvasAction::InsertChar('n'),
        CanvasAction::InsertChar('@'),
        CanvasAction::InsertChar('e'),
        CanvasAction::InsertChar('x'),
        CanvasAction::InsertChar('a'),
        CanvasAction::InsertChar('m'),
        CanvasAction::InsertChar('p'),
        CanvasAction::InsertChar('l'),
        CanvasAction::InsertChar('e'),
        CanvasAction::InsertChar('.'),
        CanvasAction::InsertChar('c'),
        CanvasAction::InsertChar('o'),
        CanvasAction::InsertChar('m'),
    ];

    println!("  Executing {} actions in batch...", actions.len());

    let results = ActionDispatcher::dispatch_batch(
        actions,
        &mut form,
        &mut ideal_cursor,
    ).await.unwrap();

    println!("  Completed {} actions", results.len());
    println!("  Final state:");
    println!("    Name: '{}'", form.name);
    println!("    Email: '{}'", form.email);
    println!("    Current field: {}", form.fields()[form.current_field()]);
    println!("    Has changes: {}", form.has_changes);
}
