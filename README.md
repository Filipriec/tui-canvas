# Canvas

Canvas is a Rust library for building form‑based, single-line input, and textarea‑driven terminal user interfaces.
It provides the core logic for text editing, validation, suggestions, and cursor management.

The library does not enforce a specific terminal UI framework:
- Core functionality works without any rendering backend.
- Terminal rendering support is available through the `gui` feature, which enables integration with `ratatui` and `crossterm`.
- Applications may also integrate Canvas with other backends by handling input and rendering independently.

---

## Overview

Canvas is designed for applications that require structured text input in a terminal environment.
It provides:

- Text editing modes (Vim‑like or normal)
- Validation (regex, masks, limits, formatting)
- Suggestions (asynchronous dropdowns)
- Computed fields (derived values)
- Undo/redo history (built in; enabled by default)
- Single-line text input widget
- Textarea widget with cursor management
- Syntax highlighting (via syntect)
- Extensible architecture for custom behaviors

### Server-Stored Validation

Canvas validation types are structured in a way that maps cleanly to a
server-owned validation schema.

Recommended integration model:

- The server stores one validation document per field.
- Clients fetch all validation documents for a table in one batch.
- Clients translate the fetched data into `canvas::ValidationConfig`.
- Raw stored values remain unmasked; masks are display metadata only.

Typical mapping:

- server `limits` -> `canvas::validation::CharacterLimits`
- server `pattern` -> `canvas::validation::PatternFilters`
- server `allowed_values` -> canvas allowed-values config
- server `mask` -> `canvas::validation::DisplayMask`
- server formatter metadata -> a host-provided `canvas::validation::CustomFormatter`

Canvas does not ship a built-in formatter registry: it exposes the
`CustomFormatter` trait (display formatting) and the `PositionMapper` trait
(cursor mapping between raw and formatted text). The host application is
responsible for mapping server formatter metadata to the appropriate
`CustomFormatter` implementation for each field.

This keeps validation definition centralized on the server while still letting
Canvas handle local editing, masking, and feedback.

---

## Installation

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
canvas = { version = "0.x", features = ["gui", "cursor-style", "textarea", "validation"] }
```

---

## Features

The library is feature‑gated. Enable only what you need:

- `gui` – terminal rendering support (ratatui + crossterm)
- `cursor-style` – styled cursor support
- `validation` – regex, masks, limits, formatting
- `suggestions` – asynchronous suggestions dropdowns
- `computed` – derived fields
- `textinput` – single-line text input widget
- `textarea` – textarea widget
- `syntect` – syntax highlighting support
- `textmode-vim` – Vim‑like editing (default)
- `textmode-normal` – normal editing mode

**Note:** `textmode-vim` and `textmode-normal` are mutually exclusive. Enable exactly one.

The default feature set is `["textmode-vim"]`.

---

## Host Focus Handoff

If your app has its own focus manager (buttons, panes, overlays), use:

- `canvas::integration::focus_handoff`

This module centralizes boundary handoff (`top` / `bottom`) so host apps can
connect canvas navigation to outer focus movement without matching on internal
details.

Typical flow:

```rust
use canvas::integration::focus_handoff::{
    BoundaryExit, HostKeyEventOutcome, handle_key_event_for_host,
};

match handle_key_event_for_host(&mut editor, key_event) {
    HostKeyEventOutcome::ExitCanvas(BoundaryExit::Bottom) => {
        // move host focus to the next non-canvas control
    }
    HostKeyEventOutcome::ExitCanvas(BoundaryExit::Top) => {
        // move host focus to the previous non-canvas control
    }
    _ => {}
}
```

For typed action pipelines, use:
- `execute_action_for_host`

### Integration API reference

`canvas::integration::focus_handoff` (always available):

- `handle_key_event_for_host(&mut editor, key_event) -> HostKeyEventOutcome`
  — drive canvas from a raw key event and learn when focus should leave canvas.
- `execute_action_for_host(&mut editor, action) -> HostActionOutcome` and
  `execute_action_for_host_with_options(..)` — run a typed action and get a
  host-oriented outcome.
- `map_key_event_outcome_for_host(KeyEventOutcome) -> HostKeyEventOutcome` —
  translate a raw canvas outcome if you call the editor directly.
- `BoundaryExit`, `HostKeyEventOutcome`, `HostActionOutcome` — outcome enums.
- `boundary_from_key_outcome(..)`, `key_outcome_for_vertical_navigation(..)` —
  lower-level helpers for custom vertical-navigation wiring.

`canvas::integration::crossterm_input` (requires the `crossterm` feature, pulled
in by `gui` / `cursor-style`):

- `CrosstermInputSession` — installs raw mode + bracketed paste; `install()`,
  `install_with_options(CrosstermInputOptions)`, `read_event()`, `poll_event()`,
  `uninstall()`; restores terminal state on `Drop`.
- `CrosstermInputOptions` — builder: `tui_defaults()`, `with_raw_mode(..)`,
  `with_alternate_screen(..)`, `with_mouse_capture(..)`, `with_bracketed_paste(..)`.
- `CrosstermInputGuard` — minimal RAII guard (`install()` / `uninstall()`) that
  just toggles terminal state without owning event reads.

See `cargo doc --open` for full signatures.

---

## Running Examples

The repository includes several examples. Each requires specific feature flags.
Use the following commands to run them:

```bash
# Textarea with Vim mode
cargo run --example textarea_vim --features "gui cursor-style textarea textmode-vim"

# Textarea with Normal mode
cargo run --example textarea_normal --features "gui cursor-style textarea textmode-normal"

# Single-line text input
cargo run --example textinput_normal --features "gui cursor-style textinput textmode-normal"

# Textarea with syntax highlighting
cargo run --example textarea_syntax --features "gui cursor-style textarea syntect textmode-normal"

# Validation examples
cargo run --example validation_1 --features "gui validation cursor-style"
cargo run --example validation_2 --features "gui validation cursor-style"
cargo run --example validation_3 --features "gui validation cursor-style"
cargo run --example validation_4 --features "gui validation cursor-style"
cargo run --example validation_5 --features "gui validation cursor-style"

# Suggestions
cargo run --example suggestions --features "suggestions gui cursor-style"
cargo run --example suggestions2 --features "suggestions gui cursor-style"

# Cursor auto movement
cargo run --example canvas_cursor_auto --features "gui cursor-style"

# Computed fields
cargo run --example computed_fields --features "gui computed"
```

For terminal paste support with `crossterm`, the smoothest path is to install
`canvas::integration::crossterm_input::CrosstermInputSession` and read events
through it. That enables raw mode and bracketed paste by default, and you can
opt into alternate screen and mouse capture with
`CrosstermInputOptions::tui_defaults()`. The current event helpers are
intentionally `crossterm`-specific. For the simple path, bind those events to
`FormEditor::handle_event(...)`, `TextAreaState::handle_event(...)`, or
`TextInputState::handle_event(...)`.

---

## Documentation

- API documentation: `cargo doc --open`
- Migration notes: `CANVAS_MIGRATION.md`

---

## License

Licensed under either of:
- Apache License, Version 2.0
- MIT License

at your option.

---

## Contributing

Contributions are welcome. Please follow the existing code structure and feature‑gating conventions.
