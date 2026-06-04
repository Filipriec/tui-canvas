//! Minimal modal undo/redo demo (Helix-style) for the single-line text input.
//!
//! The `TextInput` widget is not modal on its own, so this example adds a tiny
//! normal/insert layer the way a modal editor does. Each insert session
//! (`i` … `Esc`) becomes one undo step.
//!
//! Run with:
//!   cargo run --example undo_redo --features "gui,cursor-style,textinput"
//!
//! Keys (NORMAL):  i insert  ·  u undo  ·  Ctrl+U redo  ·  Ctrl+C quit
//! Keys (INSERT):  type  ·  Esc back to normal

use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};

use canvas::{
    integration::crossterm_input::{CrosstermInputOptions, CrosstermInputSession},
    CursorManager, TextInput, TextInputState,
};

struct Demo {
    input: TextInputState,
    insert: bool,
}

fn ui(f: &mut Frame, demo: &mut Demo) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(3)])
        .split(f.area());

    let mode = if demo.insert { "INSERT" } else { "NORMAL" };
    let block = Block::default().borders(Borders::ALL).title(format!(
        "[{mode}]  i insert · Esc normal · u undo · Ctrl+U redo · Ctrl+C quit"
    ));
    let widget = TextInput::default().block(block.clone());
    f.render_stateful_widget(widget, chunks[0], &mut demo.input);

    let (cx, cy) = demo.input.cursor(chunks[0], Some(&block));
    f.set_cursor_position((cx, cy));

    let status = format!(
        "undo: {}   redo: {}",
        if demo.input.can_undo() {
            "available"
        } else {
            "—"
        },
        if demo.input.can_redo() {
            "available"
        } else {
            "—"
        },
    );
    f.render_widget(
        Paragraph::new(Line::from(Span::raw(status)))
            .block(Block::default().borders(Borders::ALL).title("History")),
        chunks[1],
    );
}

fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    session: &CrosstermInputSession,
) -> anyhow::Result<()> {
    let mut demo = Demo {
        input: TextInputState::default(),
        insert: false,
    };
    demo.input.set_placeholder("Press i to start typing");

    loop {
        terminal.draw(|f| ui(f, &mut demo))?;

        let Event::Key(key) = session.read_event()? else {
            continue;
        };
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        // Ctrl+C quits from any mode.
        if ctrl && key.code == KeyCode::Char('c') {
            return Ok(());
        }

        if demo.insert {
            // INSERT mode: Esc returns to normal, everything else edits.
            match key.code {
                KeyCode::Esc => {
                    let _ = demo.input.exit_edit_mode();
                    demo.insert = false;
                }
                _ => {
                    let _ = demo.input.input(key);
                }
            }
        } else {
            // NORMAL mode: keys are commands.
            match key.code {
                KeyCode::Char('i') => {
                    demo.input.enter_edit_mode();
                    demo.insert = true;
                }
                KeyCode::Char('u') if ctrl => {
                    demo.input.redo();
                }
                KeyCode::Char('u') => {
                    demo.input.undo();
                }
                _ => {}
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    let mut session =
        CrosstermInputSession::install_with_options(CrosstermInputOptions::tui_defaults())?;
    let backend = CrosstermBackend::new(std::io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, &session);

    let _ = CursorManager::reset();
    let _ = session.uninstall();
    terminal.show_cursor()?;

    result
}
