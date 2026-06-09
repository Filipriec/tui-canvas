//! Minimal modeless textarea using the default VSCode keybindings.
//!
//! VSCode is modeless, so this example must be built with `textmode-normal`
//! (which is mutually exclusive with the default `textmode-vim`), hence
//! `--no-default-features`:
//!
//!   cargo run --example textarea_vscode_minimal --no-default-features \
//!     --features "gui,textarea,keybindings,cursor-style,commandline,textmode-normal,clipboard"
//!
//! `clipboard` is included so Ctrl+C / Ctrl+X mirror to the OS clipboard; it's
//! a default feature that `--no-default-features` would otherwise drop.
//!
//! Try:
//!   - arrows / Ctrl+Left-Right to move, Ctrl+Backspace to delete a word
//!   - Shift+arrows (and Shift+Ctrl+arrows) to select; Ctrl+A selects all
//!   - Ctrl+C / Ctrl+X / Ctrl+V to copy / cut / paste (whole line with no
//!     selection); typing or Backspace replaces a selection
//!   - paste from the terminal (Ctrl+Shift+V / middle click) inserts the text
//!     in one shot via bracketed paste
//!   - Ctrl+Z / Ctrl+Y to undo / redo, Ctrl+Shift+K to delete the line
//!   - Alt+Up/Down to move the line, Shift+Alt+Up/Down to duplicate it

#[cfg(not(feature = "keybindings"))]
compile_error!(
    "This example requires the 'keybindings' feature. \
     Run with: cargo run --example textarea_vscode_minimal --no-default-features \
     --features \"gui,textarea,keybindings,cursor-style,commandline,textmode-normal\""
);

#[cfg(not(feature = "textarea"))]
compile_error!(
    "This example requires the 'textarea' feature. \
     Run with: cargo run --example textarea_vscode_minimal --no-default-features \
     --features \"gui,textarea,keybindings,cursor-style,commandline,textmode-normal\""
);

#[cfg(not(feature = "cursor-style"))]
compile_error!(
    "This example requires the 'cursor-style' feature. \
     Run with: cargo run --example textarea_vscode_minimal --no-default-features \
     --features \"gui,textarea,keybindings,cursor-style,commandline,textmode-normal\""
);

#[cfg(not(feature = "commandline"))]
compile_error!(
    "This example requires the 'commandline' feature. \
     Run with: cargo run --example textarea_vscode_minimal --no-default-features \
     --features \"gui,textarea,keybindings,cursor-style,commandline,textmode-normal\""
);

use std::io;

use crossterm::{
    event::{
        Event, KeyCode, KeyModifiers, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::supports_keyboard_enhancement,
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    widgets::Block,
    Frame, Terminal,
};

use tui_canvas::{
    integration::crossterm_input::{CrosstermInputOptions, CrosstermInputSession},
    keybindings::BuiltinCanvasKeybindingPreset,
    TextArea, TextAreaState,
};

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    session: &CrosstermInputSession,
    mut textarea: TextAreaState,
) -> io::Result<()> {
    loop {
        textarea.update_cursor_style()?;
        terminal.draw(|f| ui(f, &mut textarea))?;

        match session.read_event()? {
            Event::Key(key) => {
                // Ctrl+Q quits (Ctrl+C is the VSCode copy chord and is left to
                // the editor).
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('q') {
                    return Ok(());
                }
                let _ = textarea.handle_key_event(key);
            }
            // Bracketed paste (enabled by the session) and any other non-key
            // events are routed through the high-level `handle_event`, which
            // inserts the pasted text in one shot rather than key-by-key.
            other => {
                let _ = textarea.handle_event(other);
            }
        }
    }
}

fn ui(f: &mut Frame, textarea: &mut TextAreaState) {
    let area = f.area();
    let block = Block::bordered().title("textarea_vscode_minimal (Ctrl+Q to quit)");

    f.render_stateful_widget(TextArea::default().block(block.clone()), area, textarea);

    let (x, y) = textarea.cursor_with_commandline(area, Some(&block));
    f.set_cursor_position((x, y));
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The session owns raw mode, the alternate screen, mouse capture, and
    // bracketed paste (so terminal pastes arrive as a single `Event::Paste`).
    let mut session =
        CrosstermInputSession::install_with_options(CrosstermInputOptions::tui_defaults())?;

    // Chords like Ctrl+Backspace and Ctrl+Shift+K are indistinguishable from
    // plain Backspace / Ctrl+K in a legacy terminal. The Kitty keyboard
    // protocol (supported by Alacritty, Kitty, foot, WezTerm, …) makes the
    // terminal report the real modifiers so those bindings can fire.
    let enhanced = supports_keyboard_enhancement().unwrap_or(false);
    if enhanced {
        execute!(
            io::stdout(),
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
            )
        )?;
    }

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut textarea = TextAreaState::from_text("A simple VSCode-style textarea.\nType here.");
    textarea.use_wrap();
    textarea.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Vscode);
    textarea.use_default_commandline();
    textarea.update_cursor_style()?;

    let res = run_app(&mut terminal, &session, textarea);

    if enhanced {
        execute!(io::stdout(), PopKeyboardEnhancementFlags)?;
    }
    let _ = session.uninstall();
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}
