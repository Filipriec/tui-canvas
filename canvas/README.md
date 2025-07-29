# Canvas 🎨

A reusable, type-safe canvas system for building form-based TUI applications with vim-like modal editing.

## ✨ Features

- **Type-Safe Actions**: No more string-based action names - everything is compile-time checked
- **Generic Design**: Implement `CanvasState` once, get navigation, editing, and suggestions for free
- **Vim-Like Experience**: Modal editing with familiar keybindings
- **Suggestion System**: Built-in autocomplete and suggestions support
- **Framework Agnostic**: Works with any TUI framework or raw terminal handling
- **Async Ready**: Full async/await support for modern Rust applications
- **Batch Operations**: Execute multiple actions atomically
- **Extensible**: Custom actions and feature-specific handling

## 🚀 Quick Start

Add to your `Cargo.toml`:

```toml
cargo add canvas
```

Implement the `CanvasState` trait:

```rust
use canvas::prelude::*;

#[derive(Debug)]
struct LoginForm {
    current_field: usize,
    cursor_pos: usize,
    username: String,
    password: String,
    has_changes: bool,
}

impl CanvasState for LoginForm {
    fn current_field(&self) -> usize { self.current_field }
    fn current_cursor_pos(&self) -> usize { self.cursor_pos }
    fn set_current_field(&mut self, index: usize) { self.current_field = index; }
    fn set_current_cursor_pos(&mut self, pos: usize) { self.cursor_pos = pos; }

    fn get_current_input(&self) -> &str {
        match self.current_field {
            0 => &self.username,
            1 => &self.password,
            _ => "",
        }
    }

    fn get_current_input_mut(&mut self) -> &mut String {
        match self.current_field {
            0 => &mut self.username,
            1 => &mut self.password,
            _ => unreachable!(),
        }
    }

    fn inputs(&self) -> Vec<&String> { vec![&self.username, &self.password] }
    fn fields(&self) -> Vec<&str> { vec!["Username", "Password"] }
    fn has_unsaved_changes(&self) -> bool { self.has_changes }
    fn set_has_unsaved_changes(&mut self, changed: bool) { self.has_changes = changed; }
}
```

Use the type-safe action dispatcher:

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut form = LoginForm::new();
    let mut ideal_cursor = 0;

    // Type a character - compile-time safe!
    ActionDispatcher::dispatch(
        CanvasAction::InsertChar('h'),
        &mut form,
        &mut ideal_cursor,
    ).await?;

    // Move to next field
    ActionDispatcher::dispatch(
        CanvasAction::NextField,
        &mut form,
        &mut ideal_cursor,
    ).await?;

    // Batch operations
    let actions = vec![
        CanvasAction::InsertChar('p'),
        CanvasAction::InsertChar('a'),
        CanvasAction::InsertChar('s'),
        CanvasAction::InsertChar('s'),
    ];

    ActionDispatcher::dispatch_batch(actions, &mut form, &mut ideal_cursor).await?;

    Ok(())
}
```

## 📚 Examples

The `examples/` directory contains comprehensive examples showing different usage patterns:

### Run the Examples

```bash
# Basic login form with TUI
cargo run --example simple_login

# Advanced configuration screen with suggestions and validation
cargo run --example config_screen

# API usage patterns and quick start guide
cargo run --example basic_usage

# Advanced integration patterns (state machines, events, validation)
cargo run --example integration_patterns
```

### Example Overview

| Example | Description | Key Features |
|---------|-------------|--------------|
| `simple_login` | Interactive login form TUI | Basic form, custom actions, password masking |
| `config_screen` | Configuration editor | Auto-suggestions, field validation, complex UI |
| `basic_usage` | API demonstration | All core patterns, non-interactive |
| `integration_patterns` | Architecture patterns | State machines, events, validation pipelines |

## 🎯 Type-Safe Actions

The Canvas system uses strongly-typed actions instead of error-prone strings:

```rust
// ✅ Type-safe - impossible to make typos
ActionDispatcher::dispatch(CanvasAction::MoveLeft, &mut form, &mut cursor).await?;

// ❌ Old way - runtime errors waiting to happen
execute_edit_action("move_left", key, &mut form, &mut cursor).await?;
execute_edit_action("move_leftt", key, &mut form, &mut cursor).await?; // Oops!
```

### Available Actions

```rust
pub enum CanvasAction {
    // Character input
    InsertChar(char),
    
    // Deletion
    DeleteBackward,
    DeleteForward,
    
    // Movement
    MoveLeft, MoveRight, MoveUp, MoveDown,
    MoveLineStart, MoveLineEnd,
    MoveWordNext, MoveWordPrev,
    
    // Navigation
    NextField, PrevField,
    MoveFirstLine, MoveLastLine,
    
    // Suggestions
    SuggestionUp, SuggestionDown,
    SelectSuggestion, ExitSuggestions,
    
    // Extensibility
    Custom(String),
}
```

## 🔧 Advanced Features

### Suggestions and Autocomplete

```rust
impl CanvasState for MyForm {
    fn get_suggestions(&self) -> Option<&[String]> {
        if self.suggestions.is_active {
            Some(&self.suggestions.suggestions)
        } else {
            None
        }
    }

    fn handle_feature_action(&mut self, action: &CanvasAction, _context: &ActionContext) -> Option<String> {
        match action {
            CanvasAction::InsertChar('@') => {
                // Trigger email suggestions
                let suggestions = vec![
                    format!("{}@gmail.com", self.username),
                    format!("{}@company.com", self.username),
                ];
                self.activate_suggestions(suggestions);
                None // Let generic handler insert the '@'
            }
            CanvasAction::SelectSuggestion => {
                if let Some(suggestion) = self.suggestions.get_selected() {
                    *self.get_current_input_mut() = suggestion.clone();
                    self.deactivate_suggestions();
                    Some("Applied suggestion".to_string())
                }
                None
            }
            _ => None,
        }
    }
}
```

### Custom Actions

```rust
fn handle_feature_action(&mut self, action: &CanvasAction, _context: &ActionContext) -> Option<String> {
    match action {
        CanvasAction::Custom(cmd) => match cmd.as_str() {
            "uppercase" => {
                *self.get_current_input_mut() = self.get_current_input().to_uppercase();
                Some("Converted to uppercase".to_string())
            }
            "validate_email" => {
                if self.get_current_input().contains('@') {
                    Some("Email is valid".to_string())
                } else {
                    Some("Invalid email format".to_string())
                }
            }
            _ => None,
        },
        _ => None,
    }
}
```

### Integration with TUI Frameworks

Canvas is framework-agnostic and works with any TUI library:

```rust
// Works with crossterm (see examples)
// Works with termion
// Works with ratatui/tui-rs
// Works with cursive
// Works with raw terminal I/O
```

## 🏗️ Architecture

Canvas follows a clean, layered architecture:

```
┌─────────────────────────────────────┐
│           Your Application          │
├─────────────────────────────────────┤
│         ActionDispatcher            │  ← High-level API
├─────────────────────────────────────┤
│     CanvasAction (Type-Safe)        │  ← Type safety layer
├─────────────────────────────────────┤
│        Action Handlers              │  ← Core logic
├─────────────────────────────────────┤
│        CanvasState Trait            │  ← Your implementation
└─────────────────────────────────────┘
```

## 🤝 Why Canvas?

### Before Canvas
```rust
// ❌ Error-prone string actions
execute_action("move_left", key, state)?;
execute_action("move_leftt", key, state)?; // Runtime error!

// ❌ Duplicate navigation logic everywhere
impl MyLoginForm { /* navigation code */ }
impl MyConfigForm { /* same navigation code */ }
impl MyDataForm { /* same navigation code again */ }

// ❌ Manual cursor and field management
if key == Key::Tab {
    current_field = (current_field + 1) % fields.len();
    cursor_pos = cursor_pos.min(current_input.len());
}
```

### With Canvas
```rust
// ✅ Type-safe actions
ActionDispatcher::dispatch(CanvasAction::MoveLeft, state, cursor)?;
// Typos are impossible - won't compile!

// ✅ Implement once, use everywhere
impl CanvasState for MyForm { /* minimal implementation */ }
// All navigation, editing, suggestions work automatically!

// ✅ High-level operations
ActionDispatcher::dispatch_batch(actions, state, cursor)?;
```

## 📖 Documentation

- **API Docs**: `cargo doc --open`
- **Examples**: See `examples/` directory
- **Migration Guide**: See `CANVAS_MIGRATION.md`

## 🔄 Migration from String-Based Actions

Canvas provides backwards compatibility during migration:

```rust
// Legacy support (deprecated)
execute_edit_action("move_left", key, state, cursor).await?;

// New type-safe way
ActionDispatcher::dispatch(CanvasAction::MoveLeft, state, cursor).await?;
```

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run specific example
cargo run --example simple_login

# Check type safety
cargo check
```

## 📋 Requirements

- Rust 1.70+
- Terminal with cursor support
- Optional: async runtime (tokio) for examples

## 🤔 FAQ

**Q: Does Canvas work with [my TUI framework]?**  
A: Yes! Canvas is framework-agnostic. Just implement `CanvasState` and handle the key events.

**Q: Can I extend Canvas with custom actions?**  
A: Absolutely! Use `CanvasAction::Custom("my_action")` or implement `handle_feature_action`.

**Q: Is Canvas suitable for complex forms?**  
A: Yes! See the `config_screen` example for validation, suggestions, and multi-field forms.

**Q: How do I migrate from string-based actions?**  
A: Canvas provides backwards compatibility. Migrate incrementally using the type-safe APIs.

## 📄 License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## 🙏 Contributing

Will write here something later on, too busy rn

---

Built with ❤️ for the Rust TUI community
