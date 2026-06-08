//! Minimal fixed-row form example using the default Helix keybindings.
//!
//! This uses `TextFormState`, so each row is a fixed slot. Deleting a row clears
//! that slot instead of shifting later rows upward.
//!
//! Run with:
//!   cargo run --example form_helix --features "gui,keybindings,cursor-style"

#[cfg(not(feature = "keybindings"))]
compile_error!(
    "This example requires the 'keybindings' feature. \
     Run with: cargo run --example form_helix --features \"gui,keybindings,cursor-style\""
);

#[cfg(not(feature = "gui"))]
compile_error!(
    "This example requires the 'gui' feature. \
     Run with: cargo run --example form_helix --features \"gui,keybindings,cursor-style\""
);

#[cfg(not(feature = "cursor-style"))]
compile_error!(
    "This example requires the 'cursor-style' feature. \
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

use tui_canvas::{
    keybindings::BuiltinCanvasKeybindingPreset, render_canvas_default, DataProvider, TextFormState,
};

const ROW_NAMES: [&str; 3] = ["Name", "Email", "Message"];

struct App {
    form: TextFormState<FixedRowsProvider>,
}

#[derive(Debug, Clone)]
struct FixedRowsProvider {
    rows: Vec<(&'static str, String)>,
}

impl Default for FixedRowsProvider {
    fn default() -> Self {
        Self {
            rows: ROW_NAMES
                .iter()
                .copied()
                .map(|name| (name, String::new()))
                .collect(),
        }
    }
}

impl FixedRowsProvider {
    fn assign_lines(&mut self, lines: impl IntoIterator<Item = String>) {
        for row in &mut self.rows {
            row.1.clear();
        }

        let mut last = self.rows.len().saturating_sub(1);
        for (index, line) in lines.into_iter().enumerate() {
            if index < self.rows.len() {
                self.rows[index].1 = line.replace('\n', "");
                last = index;
            } else if let Some((_, value)) = self.rows.get_mut(last) {
                if !value.is_empty() {
                    value.push(' ');
                }
                value.push_str(&line.replace('\n', ""));
            }
        }
    }
}

impl DataProvider for FixedRowsProvider {
    fn field_count(&self) -> usize {
        self.rows.len()
    }

    fn field_name(&self, index: usize) -> &str {
        self.rows[index].0
    }

    fn field_value(&self, index: usize) -> &str {
        &self.rows[index].1
    }

    fn set_field_value(&mut self, index: usize, value: String) {
        if let Some((_, row)) = self.rows.get_mut(index) {
            *row = value.replace('\n', "");
        }
    }

    fn restore_content(&mut self, fields: &[String]) {
        self.assign_lines(fields.iter().cloned());
    }
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        app.form.update_cursor_style()?;
        terminal.draw(|f| ui(f, &mut app))?;

        match event::read()? {
            Event::Key(key)
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && key.code == KeyCode::Char('c') =>
            {
                return Ok(());
            }
            Event::Key(key) => {
                let _ = app.form.handle_key_event(key);
            }
            Event::Paste(text) => {
                let _ = app.form.paste(&text);
            }
            _ => {}
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(f.area());

    render_canvas_default(f, chunks[0], app.form.core());

    let status = format!(
        "mode: {:?}  row: {}/{}  -  Helix keys  fixed rows  Ctrl+C quit",
        app.form.mode(),
        app.form.current_field() + 1,
        app.form.data_provider().field_count(),
    );
    let bar = Paragraph::new(Line::from(Span::raw(status)))
        .block(Block::default().borders(Borders::ALL).title("form_helix"));
    f.render_widget(bar, chunks[1]);

    let (x, y) = app.form.cursor(chunks[0], None);
    f.set_cursor_position((x, y));
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut form = TextFormState::<FixedRowsProvider>::default();
    form.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);
    form.update_cursor_style()?;

    let res = run_app(&mut terminal, App { form });

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
