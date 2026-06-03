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

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};

use canvas::{
    integration::crossterm_input::{CrosstermInputOptions, CrosstermInputSession},
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

    fn handle_event(&mut self, event: Event) {
        match self.input.handle_event(event) {
            TextInputEventOutcome::Submitted => {
                self.status = format!("Submitted: {}", self.input.text());
            }
            TextInputEventOutcome::Handled | TextInputEventOutcome::Ignored => {}
        }
    }
}

fn handle_key_press(key_event: KeyEvent, app: &mut TextInputDemo) -> bool {
    if (key_event.code == KeyCode::Char('q') && key_event.modifiers.contains(KeyModifiers::CONTROL))
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

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    session: &CrosstermInputSession,
) -> anyhow::Result<()> {
    let mut app = TextInputDemo::new();

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        match session.read_event()? {
            Event::Key(key) => {
                if !handle_key_press(key, &mut app) {
                    return Ok(());
                }
            }
            other => app.handle_event(other),
        }
    }
}

fn main() -> anyhow::Result<()> {
    let mut session =
        CrosstermInputSession::install_with_options(CrosstermInputOptions::tui_defaults())?;
    let backend = CrosstermBackend::new(std::io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, &session);
    let _ = CursorManager::reset();
    let _ = session.uninstall();
    terminal.show_cursor()?;

    result
}
