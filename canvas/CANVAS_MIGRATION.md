# Canvas Library Migration Guide

## Overview

This guide covers the migration from the legacy canvas library structure to the new clean, modular architecture. The new design separates core canvas functionality from autocomplete features, providing better type safety and maintainability.

## Key Changes

### 1. **Modular Architecture**
```
# Old Structure (LEGACY)
src/
├── state.rs              # Mixed canvas + autocomplete
├── actions/edit.rs       # Mixed concerns
├── gui/render.rs         # Everything together
└── suggestions.rs        # Legacy file

# New Structure (CLEAN)
src/
├── canvas/               # Core canvas functionality
│   ├── state.rs         # CanvasState trait only
│   ├── actions/edit.rs  # Canvas actions only
│   └── gui.rs           # Canvas rendering
├── autocomplete/         # Rich autocomplete features
│   ├── state.rs         # AutocompleteCanvasState trait
│   ├── types.rs         # SuggestionItem, AutocompleteState
│   ├── actions.rs       # Autocomplete actions
│   └── gui.rs           # Autocomplete dropdown rendering
└── dispatcher.rs         # Action routing
```

### 2. **Trait Separation**
- **CanvasState**: Core form functionality (navigation, input, validation)
- **AutocompleteCanvasState**: Optional rich autocomplete features

### 3. **Rich Suggestions**
Replaced simple string suggestions with typed, rich suggestion objects.

## Migration Steps

### Step 1: Update Import Paths

**Find and Replace these imports:**

```rust
# OLD IMPORTS
use canvas::CanvasState;
use canvas::CanvasAction;
use canvas::ActionContext;
use canvas::HighlightState;
use canvas::CanvasTheme;
use canvas::ActionDispatcher;
use canvas::ActionResult;

# NEW IMPORTS
use canvas::canvas::CanvasState;
use canvas::canvas::CanvasAction;
use canvas::canvas::ActionContext;
use canvas::canvas::HighlightState;
use canvas::canvas::CanvasTheme;
use canvas::dispatcher::ActionDispatcher;
use canvas::canvas::ActionResult;
```

**Complex imports:**
```rust
# OLD
use canvas::{CanvasAction, ActionDispatcher, ActionResult};

# NEW
use canvas::{canvas::CanvasAction, dispatcher::ActionDispatcher, canvas::ActionResult};
```

### Step 2: Clean Up State Implementation

**Remove legacy methods from your CanvasState implementation:**

```rust
impl CanvasState for YourFormState {
    // Keep all the core methods:
    fn current_field(&self) -> usize { /* ... */ }
    fn get_current_input(&self) -> &str { /* ... */ }
    // ... etc

    // ❌ REMOVE these legacy methods:
    // fn get_suggestions(&self) -> Option<&[String]>
    // fn get_selected_suggestion_index(&self) -> Option<usize>
    // fn set_selected_suggestion_index(&mut self, index: Option<usize>)
    // fn activate_suggestions(&mut self, suggestions: Vec<String>)
    // fn deactivate_suggestions(&mut self)
}
```

### Step 3: Implement Rich Autocomplete (Optional)

**If you want rich autocomplete features:**

```rust
use canvas::autocomplete::{AutocompleteCanvasState, SuggestionItem, AutocompleteState};

impl AutocompleteCanvasState for YourFormState {
    type SuggestionData = YourDataType; // e.g., Hit, CustomRecord, etc.

    fn supports_autocomplete(&self, field_index: usize) -> bool {
        // Define which fields support autocomplete
        matches!(field_index, 2 | 3 | 5) // Example: only certain fields
    }

    fn autocomplete_state(&self) -> Option<&AutocompleteState<Self::SuggestionData>> {
        Some(&self.autocomplete)
    }

    fn autocomplete_state_mut(&mut self) -> Option<&mut AutocompleteState<Self::SuggestionData>> {
        Some(&mut self.autocomplete)
    }
}
```

**Add autocomplete field to your state:**
```rust
pub struct YourFormState {
    // ... existing fields
    pub autocomplete: AutocompleteState<YourDataType>,
}
```

### Step 4: Migrate Suggestions

**Old way (simple strings):**
```rust
let suggestions = vec!["John".to_string(), "Jane".to_string()];
form_state.activate_suggestions(suggestions);
```

**New way (rich objects):**
```rust
let suggestions = vec![
    SuggestionItem::new(
        hit1,                                    // Your data object
        "John Doe (Manager) | ID: 123".to_string(), // What user sees
        "123".to_string(),                       // What gets stored
    ),
    SuggestionItem::simple(hit2, "Jane".to_string()), // Simple version
];
form_state.set_autocomplete_suggestions(suggestions);
```

### Step 5: Update Rendering

**Old rendering:**
```rust
// Manual autocomplete rendering
if form_state.autocomplete_active {
    render_autocomplete_dropdown(/* ... */);
}
```

**New rendering:**
```rust
// Canvas handles everything
use canvas::canvas::render_canvas;

let active_field_rect = render_canvas(f, area, form_state, theme, edit_mode, highlight_state);

// Optional: Rich autocomplete (if implementing AutocompleteCanvasState)
if form_state.is_autocomplete_active() {
    if let Some(autocomplete_state) = form_state.autocomplete_state() {
        canvas::autocomplete::render_autocomplete_dropdown(
            f, f.area(), active_field_rect.unwrap(), theme, autocomplete_state
        );
    }
}
```

### Step 6: Update Method Calls

**Replace legacy method calls:**
```rust
# OLD
form_state.deactivate_suggestions();

# NEW - Option A: Add your own method
impl YourFormState {
    pub fn deactivate_autocomplete(&mut self) {
        self.autocomplete_active = false;
        self.autocomplete_suggestions.clear();
        self.selected_suggestion_index = None;
    }
}
form_state.deactivate_autocomplete();

# NEW - Option B: Use rich autocomplete trait
form_state.deactivate_autocomplete(); // If implementing AutocompleteCanvasState
```

## Benefits of New Architecture

### 1. **Clean Separation of Concerns**
- Canvas: Form rendering, navigation, input handling
- Autocomplete: Rich suggestions, dropdown management, async loading

### 2. **Type Safety**
```rust
// Old: Stringly typed
let suggestions: Vec<String> = vec!["user1".to_string()];

// New: Fully typed with your domain objects
let suggestions: Vec<SuggestionItem<UserRecord>> = vec![
    SuggestionItem::new(user_record, display_text, stored_value)
];
```

### 3. **Rich UX Capabilities**
- **Display vs Storage**: Show "John Doe (Manager)" but store user ID
- **Loading States**: Built-in spinner/loading indicators
- **Async Support**: Designed for async suggestion fetching
- **Display Overrides**: Show friendly text while storing normalized data

### 4. **Future-Proof**
- Easy to add new autocomplete features
- Canvas features don't interfere with autocomplete
- Modular: Use only what you need

## Advanced Features

### Display Overrides
Perfect for foreign key relationships:

```rust
// User selects "John Doe (Manager) | ID: 123"
// Field stores: "123" (for database)
// User sees: "John Doe" (friendly display)

impl CanvasState for FormState {
    fn get_display_value_for_field(&self, index: usize) -> &str {
        if let Some(display_text) = self.link_display_map.get(&index) {
            return display_text.as_str(); // Shows "John Doe"
        }
        self.inputs().get(index).map(|s| s.as_str()).unwrap_or("") // Shows "123"
    }
    
    fn has_display_override(&self, index: usize) -> bool {
        self.link_display_map.contains_key(&index)
    }
}
```

### Progressive Enhancement
Start simple, add features when needed:

```rust
// Week 1: Basic usage
SuggestionItem::simple(data, "John".to_string());

// Week 5: Rich display
SuggestionItem::new(data, "John Doe (Manager)".to_string(), "John".to_string());

// Week 10: Store IDs, show names
SuggestionItem::new(user, "John Doe (Manager)".to_string(), "123".to_string());
```

## Breaking Changes Summary

1. **Import paths changed**: Add `canvas::` or `dispatcher::` prefixes
2. **Legacy suggestion methods removed**: Replace with rich autocomplete or custom methods
3. **No more simple suggestions**: Use `SuggestionItem` for typed suggestions
4. **Trait split**: `AutocompleteCanvasState` is now separate and optional

## Troubleshooting

### Common Compilation Errors

**Error**: `no method named 'get_suggestions' found`
**Fix**: Remove legacy method from `CanvasState` implementation

**Error**: `no 'CanvasState' in the root`
**Fix**: Change `use canvas::CanvasState` to `use canvas::canvas::CanvasState`

**Error**: `trait bound 'FormState: CanvasState' is not satisfied`
**Fix**: Make sure your state properly implements the new `CanvasState` trait

### Migration Checklist

- [ ] Updated all import paths
- [ ] Removed legacy methods from CanvasState implementation
- [ ] Added custom autocomplete methods if needed
- [ ] Updated suggestion usage to SuggestionItem
- [ ] Updated rendering calls
- [ ] Tested form functionality
- [ ] Tested autocomplete functionality (if using)

## Example: Complete Migration

**Before:**
```rust
use canvas::{CanvasState, CanvasAction};

impl CanvasState for FormState {
    fn get_suggestions(&self) -> Option<&[String]> { /* ... */ }
    fn deactivate_suggestions(&mut self) { /* ... */ }
    // ... other methods
}
```

**After:**
```rust
use canvas::canvas::{CanvasState, CanvasAction};
use canvas::autocomplete::{AutocompleteCanvasState, SuggestionItem};

impl CanvasState for FormState {
    // Only core canvas methods, no suggestion methods
    fn current_field(&self) -> usize { /* ... */ }
    fn get_current_input(&self) -> &str { /* ... */ }
    // ... other core methods only
}

impl AutocompleteCanvasState for FormState {
    type SuggestionData = Hit;
    
    fn supports_autocomplete(&self, field_index: usize) -> bool {
        self.fields[field_index].is_link
    }
    
    fn autocomplete_state(&self) -> Option<&AutocompleteState<Self::SuggestionData>> {
        Some(&self.autocomplete)
    }
    
    fn autocomplete_state_mut(&mut self) -> Option<&mut AutocompleteState<Self::SuggestionData>> {
        Some(&mut self.autocomplete)
    }
}
```

This migration results in cleaner, more maintainable, and more powerful code!
