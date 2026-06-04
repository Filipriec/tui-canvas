# Changelog

## Unreleased

### Added

- Added runtime-toggleable textarea line numbers with none, absolute, and relative modes. Line numbers are hidden by default.
- Added textarea search APIs for setting a query, clearing search, navigating to next/previous matches, and highlighting visible matches.

### Changed

- Reorganized public modules around feature ownership:
  - Cursor terminal integration moved from `canvas::canvas::cursor` to `canvas::cursor::terminal`.
  - Form rendering moved from `canvas::canvas::gui` to `canvas::form`.
  - Suggestions rendering moved from `canvas::suggestions::gui` to `canvas::suggestions::render`.
  - Editor feature helpers moved under `canvas::editor::features`.
  - Editor input helpers moved under `canvas::editor::input`.
- Removed the public `canvas::widgets` module. Form, textarea, and text input modules now live directly under `canvas::form`, `canvas::textarea`, and `canvas::textinput`.
- Renamed `AppMode` variants to Helix-style mode labels:
  - `ReadOnly` is now `Nor`.
  - `Edit` is now `Ins`.
  - `Highlight` is now `Sel`.
- Changed built-in keybinding preset section names and canonical mode names to `nor`, `ins`, and `sel`. Old preset mode aliases such as `read_only`, `edit`, and `highlight` are still accepted when parsing custom presets.

### Migration Notes

Downstream crates should update imports that reference old module paths.

Code matching on `AppMode` should update variant names:

```rust
// Before
AppMode::ReadOnly;
AppMode::Edit;
AppMode::Highlight;

// After
AppMode::Nor;
AppMode::Ins;
AppMode::Sel;
```

```rust
// Before
use canvas::canvas::CursorManager;
use canvas::canvas::gui::render_canvas_default;
use canvas::suggestions::gui::render_suggestions_dropdown;
use canvas::textarea::{TextArea, TextAreaState};
use canvas::textarea::highlight::{TextAreaSyntax, TextAreaSyntaxState};
use canvas::textinput::{TextInput, TextInputEventOutcome, TextInputState};

// After
use canvas::CursorManager;
use canvas::render_canvas_default;
use canvas::suggestions::render::render_suggestions_dropdown;
use canvas::{TextArea, TextAreaState};
use canvas::textarea::highlight::{TextAreaSyntax, TextAreaSyntaxState};
use canvas::{TextInput, TextInputEventOutcome, TextInputState};
```

The crate root continues to re-export the main widget and rendering types:

```rust
use canvas::{
    CursorManager, FormEditor, TextArea, TextAreaState, TextInput, TextInputState,
    render_canvas, render_canvas_default, render_canvas_with_options,
};
```

Use module paths under `canvas::textarea::*`, `canvas::textinput::*`, or
`canvas::form::*` when depending on widget submodules such as textarea syntax
highlighting or providers.
