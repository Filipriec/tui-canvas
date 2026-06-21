# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- Form rendering now keeps the editable form-field background visible for every input cell, while the active cursor row extends that same color across the whole row. Selection highlighting still takes priority over the field/row background. Commit trace for comparison: introduced in `53c6aea`; compare against the previous behavior with `2c3c40a..53c6aea`.

## [0.8.10]

### Added
- `CanvasTheme` trait expanded with `label_active()`, `cursor_insert()`, `cursor_select()`, `suggestion_selected()`, `warning()`, `border()`, and `border_active()` methods, all returning `ratatui::style::Style`.
- `DefaultCanvasTheme` struct with sensible default colors for all theme roles.
- `CursorManager` struct with `update_for_mode()` and `reset()` methods for managing terminal cursor styles per editing mode.
- Form rendering: terminal block-cursor color inversion, cursorline row background highlighting, and theme-aware cursor styles per editing mode.
- Form cursor positioning accounts for horizontal scroll offset, left indicator columns, and end-of-line right-padding via `set_cursor_position_scrolled`.

### Changed
- **Breaking:** `CanvasTheme` trait gained 7 new required methods (`label_active`, `cursor_insert`, `cursor_select`, `suggestion_selected`, `warning`, `border`, `border_active`). All implementors must provide these; `DefaultCanvasTheme` supplies sensible defaults.
- Insert-mode cursor style changed to `SteadyBlock` (was platform-dependent default). Selection mode uses `BlinkingBlock`; command mode uses `SteadyUnderScore`.
- All `CanvasTheme` methods now return full `ratatui::style::Style` (foreground + background) rather than individual color components.
- Suggestions dropdown now uses theme-aware styling via `suggestion_selected()` and `background()` methods instead of hardcoded colors.

### Fixed
- Cursor position now correctly accounts for horizontal scroll offset, left indicator columns, and end-of-line right padding across form rendering and TextInput.
- Selection highlighting preserved during horizontal scroll via `clip_line_with_indicator_padded` (preserves `Span` styles across scroll boundaries).
- Form cursor no longer lands past the visible area when horizontally scrolled.

## [0.8.8]

### Changed
- Version bump only (0.8.5 → 0.8.8); no code changes.

## [0.8.5]

### Added
- `AppMode` now implements `Display`, `FromStr`, `Serialize`, and `Deserialize` for serde interop with downstream configuration formats like TOML.
- `CanvasKeyAction` now implements `Display`, `FromStr`, `Serialize`, and `Deserialize` for serde interop.
- `BuiltinCanvasKeybindingPreset` now implements `Display`, `FromStr` (with `ParseBuiltinCanvasKeybindingPresetError`), `Serialize`, and `Deserialize`.
- `TextAreaLineNumberMode` now implements `Display`, `FromStr` (with `ParseTextAreaLineNumberModeError`), `Serialize`, and `Deserialize`.
- New `textinput_helix_minimal` and `textinput_vim_minimal` examples demonstrating keybinding-paradigm usage with `TextInputState`.
- New `CanvasKeyAction::ExitSuggestions` and `exit_suggestions` keybindings added to all built-in presets (Vim, Helix, Emacs, VSCode).
- `EditorCore::is_sequence_pending()`, `TextAreaState::is_sequence_pending()`, and `TextFormState::is_sequence_pending()` APIs for detecting multi-key commands in flight (key sequences, pending counts, pending operators, literal-char captures). Intended for hosts such as tui-pages that need to decide whether to route subsequent keys to the editor or to an outer keymap.
- New `gui_utils` module providing shared rendering primitives: `display_width`, `slice_by_display_cols`, `effective_right_pad`, `compute_h_scroll_with_padding`, and styled-line clipping utilities (`clip_line_with_indicator_padded`, `clip_inline_completion_with_indicator_padded`).
- `TextInputState` now exposes `cursor()`, `ensure_visible()`, `take_edited_flag()`, `current_cursor_cols()`, and `current_display_text_for_render()` methods for display-coordinate-aware cursor management.
- `TextInput` widget gains `suggestion_style()`, `highlight_style()`, and `border_type()` builder methods.
- Characterwise selection highlighting in `TextInput` widget rendering, with highlight-preserving scroll via `clip_line_with_indicator_padded`.
- `CanvasDisplayOptions::row_input_width` callback for per-row input column-width overrides in form rendering.
- Active-row label styling via `label_active()` theme method and cursorline row background in form rendering.

### Changed
- `TextFormState` now automatically resynchronizes its internal fixed-field count from the data provider instead of panicking when the field count changes (`sync_fixed_rows` replaces `assert_fixed_rows`).
- `EditorCore::clamp_current_field_to_count` added as a shared cursor-clamping primitive used by both `sync_fixed_rows` and navigation, preventing out-of-bounds cursor positions when the field/row count shrinks.
- `DerefMut` on `TextFormState` now calls `sync_fixed_rows` before handing out the mutable core reference, so field-count changes by external data providers are picked up transparently.

### Deprecated
- **Breaking:** `BuiltinCanvasKeybindingPreset::name()` is deprecated and now panics at runtime. The method previously returned a `&'static str`; callers must migrate to `Display` / `to_string()`. The deprecated shim will be removed in 1.0.0.

### Fixed
- Fixed panic in `TextFormState` when the data provider's field count changes between operations (added dynamic resync via `DerefMut` and `with_fixed_rows`).
- Fixed stale previous-field index in `transition_to_field` when the underlying field count had been reduced (now clamped before validation).
- Fixed cursor scroll-at-end-of-line: the cursor now keeps a small trailing margin (`END_RIGHT_PAD`) at end of line instead of collapsing the right pad to zero, so typing at the end of a line does not pin the cursor to the right border.
- Fixed horizontal scroll calculation to account for right-indicator width when the cursor is at end of line, preventing off-by-one scroll jitter.
- Fixed textinput `ensure_visible` no longer resetting `h_scroll` to zero on edit when the text fits the viewport.
- Fixed right-indicator visibility in `clip_window_with_indicator_padded` and `clip_inline_completion_with_indicator_padded`: indicator now correctly hidden when all text fits within the viewport width.
- Fixed inline suggestion suffix not consistently cleared across all TextInput edit operations.
- Fixed undo coalescing in TextInput (now properly tracked via `edited_this_frame` flag, preventing stale undo history entries).

## [0.8.4]

### Changed
- Cargo.toml metadata restructured: `readme`, `repository`, `keywords`, `categories`, and `exclude` moved from workspace inheritance to crate-local values. Description updated to "Form/textarea/input for TUI".

## [0.8.3]

### Changed
- Rust edition reverted from hardcoded `"2024"` to workspace-wide `workspace = true`, resolving build issues in workspace contexts.

### Fixed
- Fixed broken `Backend` trait bounds in all examples: `Backend` → `Backend<Error = io::Error>` for compatibility with rust-analyzer and newer compiler versions.
- Fixed TOML preset/profile parsing that relied on `str::parse::<Value>()` (broken in Rust 2024 edition); replaced with `toml::from_str::<Value>()`.
- Added missing `keybindings` feature requirement to example manifest entries (`textarea_vim`, `textarea_normal`, `textarea_syntax`) so they compile correctly with `--example`.

## [0.8.2]

### Changed
- Rust edition bumped to 2024
- Dependency version bumps: `tracing` 0.1.44, `tracing-subscriber` 0.3.23, `once_cell` 1.21.4, `syntect` 5.3.0, `arboard` 3.6, `tokio-test` 0.4.5

### Fixed
- Cleaned up unused variables and dead code in form render and Emacs selection paths

## [0.8.1]

### Added
- Canvas keybinding profile overrides for downstream consumers

### Changed
- **Breaking:** Helix mode now clears selection on navigation operations, matching upstream Helix behavior. Downstream code that relied on the previous selection-preserving behavior should collapse selections explicitly before navigating.

### Fixed
- Paste clipboard integration edge case
- Canvas keybinding conflict validation

## 0.8.0

### Added

- Added full Helix editing paradigm support when using `BuiltinCanvasKeybindingPreset::Helix` via `TextFormState::use_keybinding_preset` or `TextAreaState::use_keybinding_preset`. Helix mode uses selection-first editing (`d`/`c`/`y`/`p` on the primary selection), `U` for redo, `x`/`X` for line selection extension, and collapses the primary selection after normal-mode movements. Vim, Helix, and Emacs behaviors are implemented in separate paradigm modules (`editor/paradigm/{vim,helix,emacs}`, `textarea/paradigm/{vim,helix,emacs}`) with top-level dispatch only.
- Added full Emacs editing paradigm support when using `BuiltinCanvasKeybindingPreset::Emacs`. Emacs mode uses mark/region editing (`C-SPC` set mark, `C-w` kill region, `M-w` copy region, `C-y` yank, `Esc` deactivate mark) with the same shared `nor`/`sel`/`ins` modes as Vim and Helix.
- Added opt-in default textarea command-line integration through `TextAreaState::use_default_commandline()`, including built-in `:set number`, `:set relativenumber`, `:set nonumber`, `:noh`, `/`, and `?` behavior with automatic bottom-row reservation and cursor routing.

### Changed

- `TextAreaState` and `TextFormState` now share a single editing engine on `EditorCore` instead of `TextFormState` duplicating the text area's selection, delete, change, yank, paste, and operator logic.
- Universal selection primitives (yank selection, delete character range, extract characterwise text) moved onto `EditorCore` in `editor/selection.rs` and are used by both products.
- Row-structure operations that differ by product are split into `editor/rows/fixed.rs` (forms: clear/overwrite slots, field count never changes) and `editor/rows/dynamic.rs` (areas: add/remove/merge rows). Each product calls its own variant directly; there is no runtime policy branch inside a shared function.
- Helix selection helpers (establish/collapse primary selection, exit highlight mode, extend selection by line, finish selection edit) are defined once on `EditorCore` and shared by both products.
- Renamed the duplicated `collapse_selection_helix` and `collapse_helix_selection_to_cursor` into a single paradigm-agnostic `collapse_selection_to_cursor`.

### Removed

- Removed `TextFormState`'s duplicated editing logic (selection, delete, change, yank, paste, Vim/Helix operators). It inherits the shared engine and keeps only form-specific structural handling.

### Fixed

- `d` (delete selection) returns to Normal mode in both products, whether the selection was created with `x` or `v`.
- Helix `a` (append) on the last character of a line positions the cursor after the character instead of clamping onto it.
- Helix `Esc` no longer collapses the selection; the primary selection persists in Normal mode and is collapsed with `;`.
- Text form: deleting a fixed-row selection clears the slot in place instead of shifting later fields up, yank leaves select mode, and multi-line paste writes into fixed slots without adding or removing rows.

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
- Helix paste now works in `TextFormState`: characterwise registers insert into field text, while linewise registers write into fixed slots without inserting, removing, or shifting rows.
- `TextFormState` keybinding dispatch now uses an explicit fixed-row action policy. Shared cursor/text actions still route through `EditorCore`, while structural line actions are handled, no-op'd, or rejected without falling through to textarea-style row mutation.

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

[Unreleased]: https://gitlab.com/filipriec/tui-canvas/-/compare/v0.8.10...HEAD
[0.8.10]: https://gitlab.com/filipriec/tui-canvas/-/compare/v0.8.8...v0.8.10
[0.8.8]: https://gitlab.com/filipriec/tui-canvas/-/compare/v0.8.5...v0.8.8
[0.8.5]: https://gitlab.com/filipriec/tui-canvas/-/compare/v0.8.4...v0.8.5
[0.8.4]: https://gitlab.com/filipriec/tui-canvas/-/compare/v0.8.3...v0.8.4
[0.8.3]: https://gitlab.com/filipriec/tui-canvas/-/compare/v0.8.2...v0.8.3
[0.8.2]: https://gitlab.com/filipriec/tui-canvas/-/compare/v0.8.1...v0.8.2
[0.8.1]: https://gitlab.com/filipriec/tui-canvas/-/compare/v0.8.0...v0.8.1
