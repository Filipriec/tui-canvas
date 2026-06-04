# Changelog

## Unreleased

### Changed

- Reorganized public modules around feature ownership:
  - Cursor terminal integration moved from `canvas::canvas::cursor` to `canvas::cursor::terminal`.
  - Form rendering moved from `canvas::canvas::gui` to `canvas::widgets::form`.
  - Text area modules moved from `canvas::textarea` to `canvas::widgets::textarea`.
  - Text input modules moved from `canvas::textinput` to `canvas::widgets::textinput`.
  - Suggestions rendering moved from `canvas::suggestions::gui` to `canvas::suggestions::render`.
  - Editor feature helpers moved under `canvas::editor::features`.
  - Editor input helpers moved under `canvas::editor::input`.

### Migration Notes

Downstream crates should update imports that reference old module paths.

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
use canvas::widgets::textarea::highlight::{TextAreaSyntax, TextAreaSyntaxState};
use canvas::{TextInput, TextInputEventOutcome, TextInputState};
```

The crate root continues to re-export the main widget and rendering types:

```rust
use canvas::{
    CursorManager, FormEditor, TextArea, TextAreaState, TextInput, TextInputState,
    render_canvas, render_canvas_default, render_canvas_with_options,
};
```

Use module paths under `canvas::widgets::*` when depending on widget submodules
such as textarea syntax highlighting or providers.
