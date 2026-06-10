//! Minimal multi-line textarea example.
//!
//! Run with:
//!   cargo run --example textarea_vim_minimal --features "textarea,keybindings,cursor-style,commandline"

#[cfg(not(feature = "keybindings"))]
compile_error!(
    "This example requires the 'keybindings' feature. \
     Run with: cargo run --example textarea_vim_minimal --features \"textarea,keybindings,cursor-style,commandline\""
);

#[cfg(not(feature = "textarea"))]
compile_error!(
    "This example requires the 'textarea' feature. \
     Run with: cargo run --example textarea_vim_minimal --features \"textarea,keybindings,cursor-style,commandline\""
);

#[cfg(not(feature = "cursor-style"))]
compile_error!(
    "This example requires the 'cursor-style' feature. \
     Run with: cargo run --example textarea_vim_minimal --features \"textarea,keybindings,cursor-style,commandline\""
);

#[cfg(not(feature = "commandline"))]
compile_error!(
    "This example requires the 'commandline' feature. \
     Run with: cargo run --example textarea_vim_minimal --features \"textarea,keybindings,cursor-style,commandline\""
);

use std::io;

use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    widgets::Block,
    Frame, Terminal,
};

use tui_canvas::{
    integration::crossterm_input::{CrosstermInputOptions, CrosstermInputSession},
    keybindings::CanvasKeyBindings,
    TextArea, TextAreaState,
};

fn run_app<B: Backend<Error = io::Error>>(
    terminal: &mut Terminal<B>,
    session: &CrosstermInputSession,
    mut textarea: TextAreaState,
) -> io::Result<()> {
    loop {
        textarea.update_cursor_style()?;
        terminal.draw(|f| ui(f, &mut textarea))?;

        match session.read_event()? {
            Event::Key(key) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
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
    let block = Block::bordered().title("textarea");

    f.render_stateful_widget(TextArea::default().block(block.clone()), area, textarea);

    let (x, y) = textarea.cursor(area, Some(&block));
    f.set_cursor_position((x, y));
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut session =
        CrosstermInputSession::install_with_options(CrosstermInputOptions::tui_defaults())?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut textarea = TextAreaState::from_text("A simple textarea.\nType here.");
    textarea.use_wrap();
    textarea.set_keybindings(CanvasKeyBindings::vim_defaults());
    textarea.use_default_commandline();
    textarea.update_cursor_style()?;

    let res = run_app(&mut terminal, &session, textarea);

    let _ = session.uninstall();
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}
