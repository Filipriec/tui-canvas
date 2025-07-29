# Canvas Crate Migration Documentation

## Files Moved from Client to Canvas

### Core Canvas Files

| **Canvas Location** | **Original Client Location** | **Purpose** |
|-------------------|---------------------------|-----------|
| `canvas/src/state.rs` | `client/src/state/pages/canvas_state.rs` | Core CanvasState trait |
| `canvas/src/actions/edit.rs` | `client/src/functions/modes/edit/form_e.rs` | Generic edit actions |
| `canvas/src/renderer.rs` | `client/src/components/handlers/canvas.rs` | Canvas rendering logic |
| `canvas/src/modes/highlight.rs` | `client/src/state/app/highlight.rs` | Highlight state types |
| `canvas/src/modes/manager.rs` | `client/src/modes/handlers/mode_manager.rs` | Mode management |

## Import Replacements Needed in Client

### 1. CanvasState Trait Usage
**Replace these imports:**
```rust
// OLD
use crate::state::pages::canvas_state::CanvasState;

// NEW  
use canvas::canvas::CanvasState;
```

**Files that need updating:**
- `src/modes/canvas/edit.rs` - Line 9
- `src/modes/canvas/read_only.rs` - Line 5
- `src/ui/handlers/render.rs` - Line 17
- `src/state/pages/auth.rs` - All CanvasState impls
- `src/state/pages/form.rs` - CanvasState impl
- `src/state/pages/add_table.rs` - CanvasState impl  
- `src/state/pages/add_logic.rs` - CanvasState impl

### 2. Edit Actions Usage
**Replace these imports:**
```rust
// OLD
use crate::functions::modes::edit::form_e::{execute_edit_action, execute_common_action};

// NEW
use canvas::{execute_edit_action, execute_common_action};
```

**Files that need updating:**
- `src/modes/canvas/edit.rs` - Lines 3-5
- `src/functions/modes/edit/auth_e.rs`
- `src/functions/modes/edit/add_table_e.rs`
- `src/functions/modes/edit/add_logic_e.rs`

### 3. Canvas Rendering Usage  
**Replace these imports:**
```rust
// OLD  
use crate::components::handlers::canvas::render_canvas;

// NEW
use canvas::render_canvas;
```

**Files that need updating:**
- Any component that renders forms (login, register, add_table, add_logic, forms)

### 4. Mode System Usage
**Replace these imports:**
```rust
// OLD
use crate::modes::handlers::mode_manager::{AppMode, ModeManager};
use crate::state::app::highlight::HighlightState;

// NEW
use canvas::{AppMode, ModeManager, HighlightState};
```

**Files that need updating:**
- `src/modes/handlers/event.rs` - Line 14
- `src/ui/handlers/ui.rs` - Mode derivation calls
- All mode handling files

## Theme Integration Required

The canvas crate expects a `CanvasTheme` trait. You need to implement this for your existing theme:

```rust
// In client/src/config/colors/themes.rs
use canvas::canvas::CanvasTheme;
use ratatui::style::Color;

impl CanvasTheme for Theme {
    fn primary_fg(&self) -> Color { self.fg }
    fn primary_bg(&self) -> Color { self.bg }
    fn accent(&self) -> Color { self.accent }
    fn warning(&self) -> Color { self.warning }
    fn secondary(&self) -> Color { self.secondary }
    fn highlight(&self) -> Color { self.highlight }
    fn highlight_bg(&self) -> Color { self.highlight_bg }
}
```

## Systematic Replacement Strategy

### Phase 1: Fix Compilation (Do This First)
1. Update `client/Cargo.toml` to depend on canvas
2. Add theme implementation  
3. Replace imports in core files

### Phase 2: Replace Feature-Specific Usage
1. Update auth components
2. Update form components  
3. Update admin components
4. Update mode handlers

### Phase 3: Remove Old Files (After Everything Works)
1. Delete `src/state/pages/canvas_state.rs`
2. Delete `src/functions/modes/edit/form_e.rs`  
3. Delete `src/components/handlers/canvas.rs`
4. Delete `src/state/app/highlight.rs`
5. Delete `src/modes/handlers/mode_manager.rs`

## Files Safe to Delete After Migration

**These can be removed once imports are updated:**
- `client/src/state/pages/canvas_state.rs`
- `client/src/functions/modes/edit/form_e.rs`
- `client/src/components/handlers/canvas.rs`  
- `client/src/state/app/highlight.rs`
- `client/src/modes/handlers/mode_manager.rs`

## Quick Start Commands

```bash
# 1. Add canvas dependency to client
cd client
echo 'canvas = { path = "../canvas" }' >> Cargo.toml

# 2. Test compilation
cargo check

# 3. Fix imports one file at a time
# Start with: src/config/colors/themes.rs (add CanvasTheme impl)
# Then: src/modes/canvas/edit.rs (replace form_e imports)
# Then: src/modes/canvas/read_only.rs (replace canvas_state import)

# 4. After all imports fixed, delete old files
rm src/state/pages/canvas_state.rs
rm src/functions/modes/edit/form_e.rs  
rm src/components/handlers/canvas.rs
rm src/state/app/highlight.rs
rm src/modes/handlers/mode_manager.rs
```

## Expected Compilation Errors

You'll get errors like:
- `cannot find type 'CanvasState' in this scope`
- `cannot find function 'execute_edit_action' in this scope`  
- `cannot find type 'AppMode' in this scope`

Fix these by replacing the imports as documented above.
