//! Minimal single-line text input example using the default Helix keybindings.
//!
//! Run with:
//!   cargo run --example textinput_helix_minimal --features "gui,textinput,keybindings,cursor-style"

#[cfg(not(feature = "keybindings"))]
compile_error!(
    "This example requires the 'keybindings' feature. \
     Run with: cargo run --example textinput_helix_minimal --features \"gui,textinput,keybindings,cursor-style\""
);

#[cfg(not(feature = "textinput"))]
compile_error!(
    "This example requires the 'textinput' feature. \
     Run with: cargo run --example textinput_helix_minimal --features \"gui,textinput,keybindings,cursor-style\""
);

#[cfg(not(feature = "gui"))]
compile_error!(
    "This example requires the 'gui' feature. \
     Run with: cargo run --example textinput_helix_minimal --features \"gui,textinput,keybindings,cursor-style\""
);

#[cfg(not(feature = "cursor-style"))]
compile_error!(
    "This example requires the 'cursor-style' feature. \
     Run with: cargo run --example textinput_helix_minimal --features \"gui,textinput,keybindings,cursor-style\""
);

use std::io;

use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::{
    Frame, Terminal,
    backend::{Backend, CrosstermBackend},
    widgets::Block,
};

use tui_canvas::{
    TextInput, TextInputState,
    integration::crossterm_input::{CrosstermInputOptions, CrosstermInputSession},
    keybindings::BuiltinCanvasKeybindingPreset,
};

fn run_app<B: Backend<Error = io::Error>>(
    terminal: &mut Terminal<B>,
    session: &CrosstermInputSession,
    mut input: TextInputState,
) -> io::Result<()> {
    loop {
        input.update_cursor_style()?;
        terminal.draw(|f| ui(f, &mut input))?;

        match session.read_event()? {
            Event::Key(key) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    return Ok(());
                }
                let _ = input.handle_key_event(key);
            }
            // Bracketed paste (enabled by the session) and any other non-key
            // events are routed through the high-level `handle_event`, which
            // inserts the pasted text in one shot rather than key-by-key.
            other => {
                let _ = input.handle_event(other);
            }
        }
    }
}

fn ui(f: &mut Frame, input: &mut TextInputState) {
    let area = f.area();
    let block = Block::bordered().title("textinput_helix_minimal");

    f.render_stateful_widget(TextInput::default().block(block.clone()), area, input);

    let (x, y) = input.cursor(area, Some(&block));
    f.set_cursor_position((x, y));
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut session =
        CrosstermInputSession::install_with_options(CrosstermInputOptions::tui_defaults())?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut input = TextInputState::from_text("A Helix-powered input. Type here.");
    input.set_placeholder("i to edit, Esc to normal, Ctrl+C to quit");
    input.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);
    input.update_cursor_style()?;

    let res = run_app(&mut terminal, &session, input);

    let _ = session.uninstall();
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}
