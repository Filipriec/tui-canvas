//! Demonstrates the single-line text input widget in normal mode.
//!
//! Run with:
//! cargo run --example textinput_normal --features "gui,cursor-style,textinput,textmode-normal"

#[cfg(not(feature = "cursor-style"))]
compile_error!(
    "This example requires the 'cursor-style' feature. \
     Run with: cargo run --example textinput_normal --features \"gui,cursor-style,textinput,textmode-normal\""
);

#[cfg(not(feature = "textinput"))]
compile_error!(
    "This example requires the 'textinput' feature. \
     Run with: cargo run --example textinput_normal --features \"gui,cursor-style,textinput,textmode-normal\""
);

use std::io;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};

use canvas::{
    textinput::{TextInput, TextInputEventOutcome, TextInputState},
    CursorManager,
};

struct TextInputDemo {
    input: TextInputState,
    status: String,
}

impl TextInputDemo {
    fn new() -> Self {
        let mut input = TextInputState::default();
        input.set_placeholder("Type a command and press Enter");

        Self {
            input,
            status: "Enter submits. Ctrl+Q exits.".to_string(),
        }
    }

    fn handle_input(&mut self, key: KeyEvent) {
        match self.input.input(key) {
            TextInputEventOutcome::Submitted => {
                self.status = format!("Submitted: {}", self.input.text());
            }
            TextInputEventOutcome::Handled | TextInputEventOutcome::Ignored => {}
        }
    }
}

fn handle_key_press(key_event: KeyEvent, app: &mut TextInputDemo) -> bool {
    if (key_event.code == KeyCode::Char('q')
        && key_event.modifiers.contains(KeyModifiers::CONTROL))
        || (key_event.code == KeyCode::Char('c')
            && key_event.modifiers.contains(KeyModifiers::CONTROL))
        || key_event.code == KeyCode::F(10)
    {
        return false;
    }

    app.handle_input(key_event);
    true
}

fn ui(f: &mut Frame, app: &mut TextInputDemo) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(f.area());

    let input_block = Block::default()
        .borders(Borders::ALL)
        .title("Command")
        .border_style(Style::default().fg(Color::Cyan));

    let input_widget = TextInput::default().block(input_block.clone());
    f.render_stateful_widget(input_widget, chunks[0], &mut app.input);

    let (cx, cy) = app.input.cursor(chunks[0], Some(&input_block));
    f.set_cursor_position((cx, cy));

    let status = Paragraph::new(Line::from(Span::raw(app.status.clone())))
        .block(Block::default().borders(Borders::ALL).title("Status"));
    f.render_widget(status, chunks[1]);
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>) -> anyhow::Result<()> {
    let mut app = TextInputDemo::new();

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            if !handle_key_press(key, &mut app) {
                return Ok(());
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal);
    let _ = CursorManager::reset();

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}
