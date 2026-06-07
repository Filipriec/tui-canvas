//! Minimal fixed-row form example using the default Helix keybindings.
//!
//! This uses the textarea editing engine, but the provider keeps a fixed set of
//! rows. Newline/open-line actions shift content inside those rows instead of
//! growing the form.
//!
//! Run with:
//!   cargo run --example form_helix --features "textarea,keybindings,cursor-style,commandline"

#[cfg(not(feature = "keybindings"))]
compile_error!(
    "This example requires the 'keybindings' feature. \
     Run with: cargo run --example form_helix --features \"textarea,keybindings,cursor-style,commandline\""
);

#[cfg(not(feature = "textarea"))]
compile_error!(
    "This example requires the 'textarea' feature. \
     Run with: cargo run --example form_helix --features \"textarea,keybindings,cursor-style,commandline\""
);

#[cfg(not(feature = "cursor-style"))]
compile_error!(
    "This example requires the 'cursor-style' feature. \
     Run with: cargo run --example form_helix --features \"textarea,keybindings,cursor-style,commandline\""
);

#[cfg(not(feature = "commandline"))]
compile_error!(
    "This example requires the 'commandline' feature. \
     Run with: cargo run --example form_helix --features \"textarea,keybindings,cursor-style,commandline\""
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
    widgets::{Block, Borders, Paragraph, StatefulWidget},
    Frame, Terminal,
};

use tui_canvas::{
    keybindings::BuiltinCanvasKeybindingPreset, render_canvas_default, CommandLine, DataProvider,
    TextAreaDataProvider, TextAreaState,
};

const ROW_NAMES: [&str; 3] = ["Name", "Email", "Message"];

struct App {
    form: TextAreaState<FixedRowsProvider>,
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
    fn row_len_chars(&self, index: usize) -> usize {
        self.rows[index].1.chars().count()
    }

    fn split_at_char(value: &str, at_char: usize) -> (String, String) {
        let at_byte = value
            .char_indices()
            .nth(at_char)
            .map(|(byte_idx, _)| byte_idx)
            .unwrap_or_else(|| value.len());
        (value[..at_byte].to_string(), value[at_byte..].to_string())
    }

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

impl TextAreaDataProvider for FixedRowsProvider {
    fn from_text(text: String) -> Self {
        let mut provider = Self::default();
        provider.set_text(text);
        provider
    }

    fn to_text(&self) -> String {
        self.rows
            .iter()
            .map(|(_, value)| value.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn set_text(&mut self, text: String) {
        self.assign_lines(text.lines().map(str::to_string));
    }

    fn line_count(&self) -> usize {
        self.rows.len()
    }

    fn split_line_at(&mut self, line_idx: usize, at_char: usize) -> usize {
        if line_idx + 1 >= self.rows.len() {
            return line_idx;
        }

        let (head, tail) = Self::split_at_char(&self.rows[line_idx].1, at_char);
        for index in ((line_idx + 2)..self.rows.len()).rev() {
            self.rows[index].1 = self.rows[index - 1].1.clone();
        }
        self.rows[line_idx].1 = head;
        self.rows[line_idx + 1].1 = tail;
        line_idx + 1
    }

    fn join_with_next(&mut self, line_idx: usize) -> Option<usize> {
        if line_idx + 1 >= self.rows.len() {
            return None;
        }

        let new_col = self.row_len_chars(line_idx);
        let next = self.rows[line_idx + 1].1.clone();
        self.rows[line_idx].1.push_str(&next);
        for index in (line_idx + 1)..self.rows.len() - 1 {
            self.rows[index].1 = self.rows[index + 1].1.clone();
        }
        if let Some((_, last)) = self.rows.last_mut() {
            last.clear();
        }
        Some(new_col)
    }

    fn join_with_prev(&mut self, line_idx: usize) -> Option<(usize, usize)> {
        if line_idx == 0 || line_idx >= self.rows.len() {
            return None;
        }

        let prev_idx = line_idx - 1;
        let new_col = self.row_len_chars(prev_idx);
        let current = self.rows[line_idx].1.clone();
        self.rows[prev_idx].1.push_str(&current);
        for index in line_idx..self.rows.len() - 1 {
            self.rows[index].1 = self.rows[index + 1].1.clone();
        }
        if let Some((_, last)) = self.rows.last_mut() {
            last.clear();
        }
        Some((prev_idx, new_col))
    }

    fn insert_blank_line_after(&mut self, line_idx: usize) -> usize {
        if line_idx + 1 >= self.rows.len() {
            return line_idx;
        }

        for index in ((line_idx + 2)..self.rows.len()).rev() {
            self.rows[index].1 = self.rows[index - 1].1.clone();
        }
        self.rows[line_idx + 1].1.clear();
        line_idx + 1
    }

    fn insert_blank_line_before(&mut self, line_idx: usize) -> usize {
        let line_idx = line_idx.min(self.rows.len().saturating_sub(1));
        for index in ((line_idx + 1)..self.rows.len()).rev() {
            self.rows[index].1 = self.rows[index - 1].1.clone();
        }
        self.rows[line_idx].1.clear();
        line_idx
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

    render_canvas_default(f, chunks[0], app.form.editor());

    let status = format!(
        "mode: {:?}  row: {}/{}  -  Helix keys  fixed rows  Ctrl+C quit",
        app.form.mode(),
        app.form.current_field() + 1,
        app.form.data_provider().field_count(),
    );
    let bar = Paragraph::new(Line::from(Span::raw(status)))
        .block(Block::default().borders(Borders::ALL).title("form_helix"));
    f.render_widget(bar, chunks[1]);

    let commandline_active = app
        .form
        .commandline()
        .map(|commandline| commandline.state().is_active())
        .unwrap_or(false);
    if let Some(commandline) = app.form.commandline_mut() {
        CommandLine::default()
            .bottom()
            .render(f.area(), f.buffer_mut(), commandline.state_mut());
    }
    if commandline_active {
        let (x, y) = app.form.cursor_with_commandline(f.area(), None);
        f.set_cursor_position((x, y));
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut form = TextAreaState::<FixedRowsProvider>::default();
    form.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);
    form.use_default_commandline();
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
