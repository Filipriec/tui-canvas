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

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    widgets::Block,
    Frame, Terminal,
};

use tui_canvas::{keybindings::CanvasKeyBindings, TextArea, TextAreaState};

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut textarea: TextAreaState) -> io::Result<()> {
    loop {
        textarea.update_cursor_style()?;
        terminal.draw(|f| ui(f, &mut textarea))?;

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Ok(());
        }

        let _ = textarea.handle_key_event(key);
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
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut textarea = TextAreaState::from_text("A simple textarea.\nType here.");
    textarea.use_wrap();
    textarea.set_keybindings(CanvasKeyBindings::vim_defaults());
    textarea.use_default_commandline();
    textarea.update_cursor_style()?;

    let res = run_app(&mut terminal, textarea);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}
