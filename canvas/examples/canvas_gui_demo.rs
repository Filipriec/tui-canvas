// examples/canvas_gui_demo.rs

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Clear},
    Frame, Terminal,
};
use std::{error::Error, io};

// Import canvas library components
use canvas::{
    canvas::{
        state::{CanvasState, ActionContext},
        gui::render_canvas,
        theme::Theme,
    },
    autocomplete::{
        state::AutocompleteCanvasState,
        gui::render_autocomplete,
        types::{AutocompleteState, SuggestionItem},
    },
    config::config::CanvasConfig,
    dispatcher::ActionDispatcher,
};

// Example form data structure
#[derive(Debug)]
struct LoginForm {
    fields: Vec<String>,
    field_labels: Vec<String>,
    current_field: usize,
    cursor_position: usize,
    autocomplete_state: AutocompleteState,
}

impl LoginForm {
    fn new() -> Self {
        Self {
            fields: vec![
                String::new(),  // username
                String::new(),  // password  
                String::new(),  // email
            ],
            field_labels: vec![
                "Username".to_string(),
                "Password".to_string(),
                "Email".to_string(),
            ],
            current_field: 0,
            cursor_position: 0,
            autocomplete_state: AutocompleteState::default(),
        }
    }
}

// Implement CanvasState trait for your form
impl CanvasState for LoginForm {
    fn field_count(&self) -> usize {
        self.fields.len()
    }

    fn current_field(&self) -> usize {
        self.current_field
    }

    fn set_current_field(&mut self, field_index: usize) {
        if field_index < self.fields.len() {
            self.current_field = field_index;
        }
    }

    fn cursor_position(&self) -> usize {
        self.cursor_position
    }

    fn set_cursor_position(&mut self, position: usize) {
        if let Some(field) = self.fields.get(self.current_field) {
            self.cursor_position = position.min(field.len());
        }
    }

    fn field_value(&self, field_index: usize) -> Option<&str> {
        self.fields.get(field_index).map(|s| s.as_str())
    }

    fn set_field_value(&mut self, field_index: usize, value: String) {
        if let Some(field) = self.fields.get_mut(field_index) {
            *field = value;
        }
    }

    fn field_label(&self, field_index: usize) -> Option<&str> {
        self.field_labels.get(field_index).map(|s| s.as_str())
    }

    fn handle_action(&mut self, _action: &str, _context: ActionContext) -> Result<(), Box<dyn Error>> {
        // Custom action handling can go here
        Ok(())
    }
}

// Implement autocomplete support
impl AutocompleteCanvasState for LoginForm {
    type SuggestionData = String;

    fn supports_autocomplete(&self, field_index: usize) -> bool {
        // Only username and email fields support autocomplete
        field_index == 0 || field_index == 2
    }

    fn autocomplete_state(&self) -> &AutocompleteState {
        &self.autocomplete_state
    }

    fn autocomplete_state_mut(&mut self) -> &mut AutocompleteState {
        &mut self.autocomplete_state
    }

    fn activate_autocomplete(&mut self) {
        if self.supports_autocomplete(self.current_field) {
            self.autocomplete_state.activate(self.current_field);
            
            // Simulate loading suggestions
            let suggestions = match self.current_field {
                0 => vec![ // Username suggestions
                    SuggestionItem::simple("admin"),
                    SuggestionItem::simple("user"),
                    SuggestionItem::simple("guest"),
                ],
                2 => vec![ // Email suggestions
                    SuggestionItem::simple("user@example.com"),
                    SuggestionItem::simple("admin@domain.com"),
                    SuggestionItem::simple("test@test.org"),
                ],
                _ => vec![],
            };
            
            self.autocomplete_state.set_suggestions(suggestions);
        }
    }

    fn apply_autocomplete_selection(&mut self) {
        if let Some(suggestion) = self.autocomplete_state.selected_suggestion() {
            self.set_field_value(self.current_field, suggestion.insert_value.clone());
            self.cursor_position = suggestion.insert_value.len();
            self.autocomplete_state.deactivate();
        }
    }
}

// Simple theme implementation
struct SimpleTheme;

impl Theme for SimpleTheme {
    fn field_style(&self, is_current: bool, _is_highlighted: bool) -> Style {
        if is_current {
            Style::default().bg(Color::DarkGray).fg(Color::White)
        } else {
            Style::default().fg(Color::Gray)
        }
    }

    fn label_style(&self, is_current: bool) -> Style {
        if is_current {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::Blue)
        }
    }

    fn cursor_style(&self) -> Style {
        Style::default().bg(Color::White).fg(Color::Black)
    }
}

struct App {
    form: LoginForm,
    config: CanvasConfig,
    dispatcher: ActionDispatcher,
    theme: SimpleTheme,
    should_quit: bool,
}

impl App {
    fn new() -> Result<Self, Box<dyn Error>> {
        Ok(App {
            form: LoginForm::new(),
            config: CanvasConfig::default(),
            dispatcher: ActionDispatcher::new(),
            theme: SimpleTheme,
            should_quit: false,
        })
    }

    fn handle_key(&mut self, key: KeyCode) -> Result<(), Box<dyn Error>> {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Tab => {
                // Activate autocomplete on tab
                self.form.activate_autocomplete();
            }
            KeyCode::Enter => {
                // Apply autocomplete selection or just move to next field
                if self.form.autocomplete_state().is_ready() {
                    self.form.apply_autocomplete_selection();
                } else {
                    let next_field = (self.form.current_field() + 1) % self.form.field_count();
                    self.form.set_current_field(next_field);
                    self.form.set_cursor_position(0);
                }
            }
            _ => {
                // Use canvas dispatcher for all other keys
                self.dispatcher.dispatch_key(key, &mut self.form, &self.config)?;
            }
        }
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run
    let mut app = App::new()?;
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
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

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<(), Box<dyn Error>> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            app.handle_key(key.code)?;
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(f.size());

    // Header
    let header_block = Block::default()
        .borders(Borders::ALL)
        .title("Canvas Library - Login Form Demo");
    f.render_widget(header_block, chunks[0]);

    // Main form area - use canvas GUI rendering
    let form_area = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(0)])
        .split(chunks[1])[0];

    // Use canvas library's GUI rendering
    render_canvas(f, form_area, &app.form, &app.theme);

    // Render autocomplete overlay if active
    if app.form.autocomplete_state().is_active() {
        render_autocomplete(f, form_area, &app.form, &app.theme);
    }

    // Footer with help
    let footer_block = Block::default()
        .borders(Borders::ALL)
        .title("Controls");
    
    let help_text = ratatui::widgets::Paragraph::new(
        "↑↓ - Navigate fields | ←→ - Move cursor | Tab - Autocomplete | Enter - Select/Next | Esc/q - Quit"
    )
    .block(footer_block)
    .style(Style::default().fg(Color::Gray));
    
    f.render_widget(help_text, chunks[2]);
}
