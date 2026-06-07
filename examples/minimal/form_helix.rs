//! Minimal form example using the default Helix keybindings.
//!
//! Demonstrates the smallest viable setup: a `FormEditor` backed by a
//! `DataProvider`, with `CanvasKeyBindings::helix_defaults()` wiring up all
//! modal navigation, editing, and field traversal through the centralized
//! keybinding system.
//!
//! Run with:
//!   cargo run --example form_helix --features "gui,keybindings,cursor-style"

#[cfg(not(feature = "keybindings"))]
compile_error!(
    "This example requires the 'keybindings' feature. \
     Run with: cargo run --example form_helix --features \"gui,keybindings,cursor-style\""
);

use std::io;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};

use tui_canvas::{keybindings::CanvasKeyBindings, render_canvas_default, DataProvider, FormEditor};

struct App {
    editor: FormEditor<Form>,
}

struct Form {
    fields: Vec<(&'static str, String)>,
}

impl Form {
    fn new() -> Self {
        Self {
            fields: vec![
                ("Name", String::new()),
                ("Email", String::new()),
                ("Message", String::new()),
            ],
        }
    }
}

impl DataProvider for Form {
    fn field_count(&self) -> usize {
        self.fields.len()
    }
    fn field_name(&self, index: usize) -> &str {
        self.fields[index].0
    }
    fn field_value(&self, index: usize) -> &str {
        &self.fields[index].1
    }
    fn set_field_value(&mut self, index: usize, value: String) {
        self.fields[index].1 = value;
    }
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &app))?;

        let Event::Key(key) = event::read()? else {
            continue;
        };
        // Ctrl+C always quits, regardless of the canvas mode.
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Ok(());
        }
        // Everything else goes through the keybinding system.
        let _ = app.editor.handle_key_event(key);
    }
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(f.area());

    render_canvas_default(f, chunks[0], &app.editor);

    let status = format!(
        "mode: {:?}  field: {}/{}  -  hjkl select  i insert  Esc normal  Tab next  Ctrl+C quit",
        app.editor.mode(),
        app.editor.current_field() + 1,
        app.editor.data_provider().field_count(),
    );
    let bar = Paragraph::new(Line::from(Span::raw(status)))
        .block(Block::default().borders(Borders::ALL).title("form_helix"));
    f.render_widget(bar, chunks[1]);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App {
        editor: FormEditor::new(Form::new()),
    };
    // One call wires up all Helix-style modal behavior.
    app.editor
        .set_keybindings(CanvasKeyBindings::helix_defaults());

    let res = run_app(&mut terminal, app);

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
