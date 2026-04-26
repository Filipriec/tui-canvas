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
- Single-line text input widget
- Textarea widget with cursor management
- Syntax highlighting (via syntect)
- Extensible architecture for custom behaviors

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

For terminal paste support with `crossterm`, install
`canvas::integration::crossterm_input::CrosstermInputGuard` once during app
startup and pass `Event::Paste(text)` to the widget's `handle_event(...)`
method. The current event helpers are intentionally `crossterm`-specific.

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
