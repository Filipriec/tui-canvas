// examples/autocomplete.rs
// Run with: cargo run --example autocomplete --features "autocomplete,gui"

use std::io;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::Color,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};

use canvas::{
    canvas::{
        gui::render_canvas,
        modes::AppMode,
        state::{ActionContext, CanvasState},
        theme::CanvasTheme,
    },
    autocomplete::{
        AutocompleteCanvasState,
        AutocompleteState,
        SuggestionItem,
        execute_with_autocomplete,
        handle_autocomplete_feature_action,
    },
    CanvasAction,
};

// Add the async_trait import
use async_trait::async_trait;

// Simple theme implementation
#[derive(Clone)]
struct DemoTheme;

impl CanvasTheme for DemoTheme {
    fn bg(&self) -> Color { Color::Reset }
    fn fg(&self) -> Color { Color::White }
    fn accent(&self) -> Color { Color::Cyan }
    fn secondary(&self) -> Color { Color::Gray }
    fn highlight(&self) -> Color { Color::Yellow }
    fn highlight_bg(&self) -> Color { Color::DarkGray }
    fn warning(&self) -> Color { Color::Red }
    fn border(&self) -> Color { Color::Gray }
}

// Custom suggestion data type
#[derive(Clone, Debug)]
struct EmailSuggestion {
    email: String,
    provider: String,
}

// Demo form state with autocomplete
struct AutocompleteFormState {
    fields: Vec<String>,
    field_names: Vec<String>,
    current_field: usize,
    cursor_pos: usize,
    mode: AppMode,
    has_changes: bool,
    debug_message: String,

    // Autocomplete state
    autocomplete: AutocompleteState<EmailSuggestion>,
}

impl AutocompleteFormState {
    fn new() -> Self {
        Self {
            fields: vec![
                "John Doe".to_string(),
                "john@".to_string(), // Partial email to demonstrate autocomplete
                "+1 234 567 8900".to_string(),
                "San Francisco".to_string(),
            ],
            field_names: vec![
                "Name".to_string(),
                "Email".to_string(),
                "Phone".to_string(),
                "City".to_string(),
            ],
            current_field: 1, // Start on email field
            cursor_pos: 5, // Position after "john@"
            mode: AppMode::Edit,
            has_changes: false,
            debug_message: "Type in email field, Tab to trigger autocomplete, Enter to select, Esc to cancel".to_string(),
            autocomplete: AutocompleteState::new(),
        }
    }
}

impl CanvasState for AutocompleteFormState {
    fn current_field(&self) -> usize { self.current_field }
    fn current_cursor_pos(&self) -> usize { self.cursor_pos }
    fn set_current_field(&mut self, index: usize) {
        self.current_field = index.min(self.fields.len().saturating_sub(1));
        // Clear autocomplete when changing fields
        if self.is_autocomplete_active() {
            self.clear_autocomplete_suggestions();
        }
    }
    fn set_current_cursor_pos(&mut self, pos: usize) {
        let max_pos = if self.mode == AppMode::Edit {
            self.fields[self.current_field].len()
        } else {
            self.fields[self.current_field].len().saturating_sub(1)
        };
        self.cursor_pos = pos.min(max_pos);
    }
    fn current_mode(&self) -> AppMode { self.mode }
    fn get_current_input(&self) -> &str { &self.fields[self.current_field] }
    fn get_current_input_mut(&mut self) -> &mut String { &mut self.fields[self.current_field] }
    fn inputs(&self) -> Vec<&String> { self.fields.iter().collect() }
    fn fields(&self) -> Vec<&str> { self.field_names.iter().map(|s| s.as_str()).collect() }
    fn has_unsaved_changes(&self) -> bool { self.has_changes }
    fn set_has_unsaved_changes(&mut self, changed: bool) { self.has_changes = changed; }

    fn handle_feature_action(&mut self, action: &CanvasAction, _context: &ActionContext) -> Option<String> {
        // Handle autocomplete actions first
        if let Some(result) = handle_autocomplete_feature_action(action, self) {
            return Some(result);
        }

        // Handle other custom actions
        match action {
            CanvasAction::Custom(cmd) => {
                match cmd.as_str() {
                    "toggle_mode" => {
                        self.mode = match self.mode {
                            AppMode::Edit => AppMode::ReadOnly,
                            AppMode::ReadOnly => AppMode::Edit,
                            _ => AppMode::Edit,
                        };
                        Some(format!("Switched to {:?} mode", self.mode))
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

// Add the #[async_trait] attribute to the implementation
#[async_trait]
impl AutocompleteCanvasState for AutocompleteFormState {
    type SuggestionData = EmailSuggestion;

    fn supports_autocomplete(&self, field_index: usize) -> bool {
        // Only enable autocomplete for email field (index 1)
        field_index == 1
    }

    fn autocomplete_state(&self) -> Option<&AutocompleteState<Self::SuggestionData>> {
        Some(&self.autocomplete)
    }

    fn autocomplete_state_mut(&mut self) -> Option<&mut AutocompleteState<Self::SuggestionData>> {
        Some(&mut self.autocomplete)
    }

    fn should_trigger_autocomplete(&self) -> bool {
        let current_input = self.get_current_input();
        let current_field = self.current_field();

        // Trigger for email field when we have "@" and at least 1 more character
        self.supports_autocomplete(current_field) &&
        current_input.contains('@') &&
        current_input.len() > current_input.find('@').unwrap_or(0) + 1 &&
        !self.is_autocomplete_active()
    }

    /// This is where the magic happens - user implements their own async fetching
    async fn trigger_autocomplete_suggestions(&mut self) {
        // 1. Activate UI (shows loading spinner)
        self.activate_autocomplete();
        self.set_autocomplete_loading(true);

        // 2. Get current input for querying
        let query = self.get_current_input().to_string();

        // 3. Extract domain part from email
        let domain_part = if let Some(at_pos) = query.find('@') {
            query[at_pos + 1..].to_string()
        } else {
            self.set_autocomplete_loading(false);
            return; // No @ symbol, can't suggest
        };

        // 4. SIMULATE ASYNC API CALL (in real code, this would be HTTP request)
        let email_prefix = query[..query.find('@').unwrap()].to_string();
        let suggestions = tokio::task::spawn_blocking(move || {
            // Simulate network delay
            std::thread::sleep(std::time::Duration::from_millis(200));

            // Create mock suggestions based on domain input
            let popular_domains = vec![
                ("gmail.com", "Gmail"),
                ("yahoo.com", "Yahoo Mail"),
                ("outlook.com", "Outlook"),
                ("hotmail.com", "Hotmail"),
                ("company.com", "Company Email"),
                ("university.edu", "University"),
            ];

            let mut results = Vec::new();

            for (domain, provider) in popular_domains {
                if domain.starts_with(&domain_part) || domain_part.is_empty() {
                    let full_email = format!("{}@{}", email_prefix, domain);
                    results.push(SuggestionItem::new(
                        EmailSuggestion {
                            email: full_email.clone(),
                            provider: provider.to_string(),
                        },
                        format!("{} ({})", full_email, provider), // display text
                        full_email, // value to store
                    ));
                }
            }

            results
        }).await.unwrap_or_default();

        // 5. Provide suggestions back to library
        self.set_autocomplete_suggestions(suggestions);
    }
}

async fn handle_key_press(key: KeyCode, modifiers: KeyModifiers, state: &mut AutocompleteFormState) -> bool {
    if key == KeyCode::F(10) || (key == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL)) {
        return false; // Quit
    }

    let action = match key {
        // === AUTOCOMPLETE KEYS ===
        KeyCode::Tab => {
            if state.is_autocomplete_active() {
                Some(CanvasAction::SuggestionDown) // Navigate suggestions
            } else if state.supports_autocomplete(state.current_field()) {
                Some(CanvasAction::TriggerAutocomplete) // Manual trigger
            } else {
                Some(CanvasAction::NextField) // Normal tab
            }
        }

        KeyCode::BackTab => {
            if state.is_autocomplete_active() {
                Some(CanvasAction::SuggestionUp)
            } else {
                Some(CanvasAction::PrevField)
            }
        }

        KeyCode::Enter => {
            if state.is_autocomplete_active() {
                Some(CanvasAction::SelectSuggestion) // Apply suggestion
            } else {
                Some(CanvasAction::NextField)
            }
        }

        KeyCode::Esc => {
            if state.is_autocomplete_active() {
                Some(CanvasAction::ExitSuggestions) // Close autocomplete
            } else {
                Some(CanvasAction::Custom("toggle_mode".to_string()))
            }
        }

        // === STANDARD CANVAS KEYS ===
        KeyCode::Left => Some(CanvasAction::MoveLeft),
        KeyCode::Right => Some(CanvasAction::MoveRight),
        KeyCode::Up => Some(CanvasAction::MoveUp),
        KeyCode::Down => Some(CanvasAction::MoveDown),
        KeyCode::Home => Some(CanvasAction::MoveLineStart),
        KeyCode::End => Some(CanvasAction::MoveLineEnd),
        KeyCode::Backspace => Some(CanvasAction::DeleteBackward),
        KeyCode::Delete => Some(CanvasAction::DeleteForward),

        // Character input
        KeyCode::Char(c) if !modifiers.contains(KeyModifiers::CONTROL) => {
            Some(CanvasAction::InsertChar(c))
        }

        _ => None,
    };

    if let Some(action) = action {
        match execute_with_autocomplete(action.clone(), state).await {
            Ok(result) => {
                if let Some(msg) = result.message() {
                    state.debug_message = msg.to_string();
                } else {
                    state.debug_message = format!("Executed: {:?}", action);
                }
                true
            }
            Err(e) => {
                state.debug_message = format!("Error: {}", e);
                true
            }
        }
    } else {
        state.debug_message = format!("Unhandled key: {:?}", key);
        true
    }
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut state: AutocompleteFormState) -> io::Result<()> {
    let theme = DemoTheme;

    loop {
        terminal.draw(|f| ui(f, &state, &theme))?;

        if let Event::Key(key) = event::read()? {
            let should_continue = handle_key_press(key.code, key.modifiers, &mut state).await;
            if !should_continue {
                break;
            }
        }
    }

    Ok(())
}

fn ui(f: &mut Frame, state: &AutocompleteFormState, theme: &DemoTheme) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),
            Constraint::Length(5),
        ])
        .split(f.area());

    // Render the canvas form
    let active_field_rect = render_canvas(
        f,
        chunks[0],
        state,
        theme,
        state.mode == AppMode::Edit,
        &canvas::HighlightState::Off,
    );

    // Render autocomplete dropdown on top if active
    if let Some(input_rect) = active_field_rect {
        canvas::render_autocomplete_dropdown(
            f,
            chunks[0],
            input_rect,
            theme,
            &state.autocomplete,
        );
    }

    // Status info
    let autocomplete_status = if state.is_autocomplete_active() {
        if state.autocomplete.is_loading {
            "Loading suggestions..."
        } else if state.has_autocomplete_suggestions() {
            "Use Tab/Shift+Tab to navigate, Enter to select, Esc to cancel"
        } else {
            "No suggestions found"
        }
    } else {
        "Tab to trigger autocomplete"
    };

    let status_lines = vec![
        Line::from(Span::raw(format!("Mode: {:?} | Field: {}/{} | Cursor: {}",
            state.mode, state.current_field + 1, state.fields.len(), state.cursor_pos))),
        Line::from(Span::raw(format!("Autocomplete: {}", autocomplete_status))),
        Line::from(Span::raw(state.debug_message.clone())),
        Line::from(Span::raw("F10: Quit | Tab: Trigger/Navigate autocomplete | Enter: Select | Esc: Cancel/Toggle mode")),
    ];

    let status = Paragraph::new(status_lines)
        .block(Block::default().borders(Borders::ALL).title("Status & Help"));

    f.render_widget(status, chunks[1]);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let state = AutocompleteFormState::new();

    let res = run_app(&mut terminal, state).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}
