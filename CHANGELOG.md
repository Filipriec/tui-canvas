# Changelog

## Unreleased

### Added

- Added full Helix editing paradigm support when using `BuiltinCanvasKeybindingPreset::Helix` via `TextFormState::use_keybinding_preset` or `TextAreaState::use_keybinding_preset`. Helix mode uses selection-first editing (`d`/`c`/`y`/`p` on the primary selection), `U` for redo, `x`/`X` for line selection extension, and collapses the primary selection after normal-mode movements. Vim, Helix, and Emacs behaviors are implemented in separate paradigm modules (`editor/paradigm/{vim,helix,emacs}`, `textarea/paradigm/{vim,helix,emacs}`) with top-level dispatch only.
- Added full Emacs editing paradigm support when using `BuiltinCanvasKeybindingPreset::Emacs`. Emacs mode uses mark/region editing (`C-SPC` set mark, `C-w` kill region, `M-w` copy region, `C-y` yank, `Esc` deactivate mark) with the same shared `nor`/`sel`/`ins` modes as Vim and Helix.
- Added opt-in default textarea command-line integration through `TextAreaState::use_default_commandline()`, including built-in `:set number`, `:set relativenumber`, `:set nonumber`, `:noh`, `/`, and `?` behavior with automatic bottom-row reservation and cursor routing.

## v0.7.5

### Changed

- Split the old form/editor layering into `EditorCore<D>` plus sibling product states. `EditorCore` owns shared cursor, mode, selection, keybinding, validation, suggestions, and history state; product states own editing policy.
- Added `TextFormState<D>` as the fixed-row product. Its fixed-row policy keeps row count stable: Enter/Tab traverse fields, line deletion clears the current field, and row insert/remove behavior is not part of the form product.
- `TextFormState<D>` now stores and enforces its initial fixed field count across product input, keybinding dispatch, paste, execute, undo, and redo paths. Vim line operations such as `dd` and counted `2dd`, plus Helix line-selection deletion such as `x` then `d`, clear fixed slots without shifting later fields upward; structural textarea operations such as join, line move, line duplicate, cut line, and linewise paste are inert for textform.
- `TextAreaState<P>` now owns `EditorCore<P>` directly. Textarea-only behavior such as split, join, insert/delete rows, and multiline paste remains textarea policy.
- `TextInputState<P>` now owns a single-row `TextFormState<P>`.
- Moved key-event sequence/count/fallback dispatch to a product-policy layer so `TextAreaState` and `TextFormState` share routing without sharing Enter/Tab/row-mutation semantics.
- Moved Helix word-selection motion onto `EditorCore`, so `TextFormState` gets Helix `w`/`b`/`e` selection highlighting without inheriting textarea row-mutation behavior.
- `TextFormState` now distinguishes Helix characterwise selections from linewise field selections: characterwise `d`/`c` edits text inside fixed fields, while linewise `x d` clears fixed slots without shifting or merging fields.

### Migration Notes

- Replace `FormEditor`/old fixed-row state imports with `TextFormState`.
- Replace `TextAreaState::editor()` / `editor_mut()` with `TextAreaState::core()` / `core_mut()`.
- Replace `TextInputState::editor()` / `editor_mut()` with `TextInputState::form()` / `form_mut()`.
- `TextAreaEditor`, `TextInputEditor`, `FormInputEventOutcome`, and the engine-level `TextForm` alias were removed. Use `EditorCore<P>` only for internals, `TextFormState<P>` for fixed-row forms, and `TextFormEventOutcome` for fixed-row form input events.

## v0.7.2

### Added

- Added runtime-toggleable textarea line numbers with none, absolute, and relative modes. Line numbers are hidden by default.
- Added textarea search APIs for setting a query, clearing search, navigating to next/previous matches, and highlighting visible matches.
- Added a reusable Vim/Helix-style command line component with `:`, `/`, and `?` modes, submit/cancel events, command history, command registration/parsing helpers, and default bottom-row rendering.

### Changed

- Form label, default input, and per-row input visual widths are now configurable through `CanvasDisplayOptions`; see `src/form/render.rs`.
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
    CursorManager, TextFormState, TextArea, TextAreaState, TextInput, TextInputState,
    render_canvas, render_canvas_default, render_canvas_with_options,
};
```

Use module paths under `canvas::textarea::*`, `canvas::textinput::*`, or
`canvas::form::*` when depending on widget submodules such as textarea syntax
highlighting or providers.
