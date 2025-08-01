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
        theme::CanvasTheme,
    },
    autocomplete::gui::render_autocomplete_dropdown,
    FormEditor, DataProvider, AutocompleteProvider, SuggestionItem,
};

use async_trait::async_trait;
use anyhow::Result;

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

// ===================================================================
// SIMPLE DATA PROVIDER - Only business data, no UI concerns!
// ===================================================================

struct ContactForm {
    // Only business data - no UI state!
    name: String,
    email: String,
    phone: String,
    city: String,
}

impl ContactForm {
    fn new() -> Self {
        Self {
            name: "John Doe".to_string(),
            email: "john@".to_string(), // Partial email for demo
            phone: "+1 234 567 8900".to_string(),
            city: "San Francisco".to_string(),
        }
    }
}

// Simple trait implementation - only 4 methods!
impl DataProvider for ContactForm {
    fn field_count(&self) -> usize { 4 }
    
    fn field_name(&self, index: usize) -> &str {
        match index {
            0 => "Name",
            1 => "Email", 
            2 => "Phone",
            3 => "City",
            _ => "",
        }
    }
    
    fn field_value(&self, index: usize) -> &str {
        match index {
            0 => &self.name,
            1 => &self.email,
            2 => &self.phone,
            3 => &self.city,
            _ => "",
        }
    }
    
    fn set_field_value(&mut self, index: usize, value: String) {
        match index {
            0 => self.name = value,
            1 => self.email = value,
            2 => self.phone = value,
            3 => self.city = value,
            _ => {}
        }
    }
    
    fn supports_autocomplete(&self, field_index: usize) -> bool {
        field_index == 1 // Only email field
    }
}

// ===================================================================
// SIMPLE AUTOCOMPLETE PROVIDER - Only data fetching!
// ===================================================================

struct EmailAutocomplete;

#[async_trait]
impl AutocompleteProvider for EmailAutocomplete {
    type SuggestionData = EmailSuggestion;
    
    async fn fetch_suggestions(&mut self, _field_index: usize, query: &str) 
        -> Result<Vec<SuggestionItem<Self::SuggestionData>>> 
    {
        // Extract domain part from email
        let (email_prefix, domain_part) = if let Some(at_pos) = query.find('@') {
            (query[..at_pos].to_string(), query[at_pos + 1..].to_string())
        } else {
            return Ok(Vec::new()); // No @ symbol
        };

        // Simulate async API call
        let suggestions = tokio::task::spawn_blocking(move || {
            // Simulate network delay
            std::thread::sleep(std::time::Duration::from_millis(200));

            // Mock email suggestions
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
                    results.push(SuggestionItem {
                        data: EmailSuggestion {
                            email: full_email.clone(),
                            provider: provider.to_string(),
                        },
                        display_text: format!("{} ({})", full_email, provider),
                        value_to_store: full_email,
                    });
                }
            }
            results
        }).await.unwrap_or_default();

        Ok(suggestions)
    }
}

// ===================================================================
// APPLICATION STATE - Much simpler!
// ===================================================================

struct AppState {
    editor: FormEditor<ContactForm>,
    autocomplete: EmailAutocomplete,
    debug_message: String,
}

impl AppState {
    fn new() -> Self {
        let contact_form = ContactForm::new();
        let mut editor = FormEditor::new(contact_form);
        
        // Start on email field (index 1) at end of existing text
        editor.set_mode(AppMode::Edit);
        // TODO: Add method to set initial field/cursor position
        
        Self {
            editor,
            autocomplete: EmailAutocomplete,
            debug_message: "Type in email field, Tab to trigger autocomplete, Enter to select, Esc to cancel".to_string(),
        }
    }
}

// ===================================================================
// INPUT HANDLING - Much cleaner!
// ===================================================================

async fn handle_key_press(key: KeyCode, modifiers: KeyModifiers, state: &mut AppState) -> bool {
    if key == KeyCode::F(10) || (key == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL)) {
        return false; // Quit
    }

    // Handle input based on key
    let result = match key {
        // === AUTOCOMPLETE KEYS ===
        KeyCode::Tab => {
            if state.editor.is_autocomplete_active() {
                state.editor.autocomplete_next();
                Ok("Navigated to next suggestion".to_string())
            } else if state.editor.data_provider().supports_autocomplete(state.editor.current_field()) {
                state.editor.trigger_autocomplete(&mut state.autocomplete).await
                    .map(|_| "Triggered autocomplete".to_string())
            } else {
                state.editor.move_to_next_field();
                Ok("Moved to next field".to_string())
            }
        }

        KeyCode::Enter => {
            if state.editor.is_autocomplete_active() {
                if let Some(applied) = state.editor.apply_autocomplete() {
                    Ok(format!("Applied: {}", applied))
                } else {
                    Ok("No suggestion to apply".to_string())
                }
            } else {
                state.editor.move_to_next_field();
                Ok("Moved to next field".to_string())
            }
        }

        KeyCode::Esc => {
            if state.editor.is_autocomplete_active() {
                // Autocomplete will be cleared automatically by mode change
                Ok("Cancelled autocomplete".to_string())
            } else {
                // Toggle between edit and readonly mode
                let new_mode = match state.editor.mode() {
                    AppMode::Edit => AppMode::ReadOnly,
                    _ => AppMode::Edit,
                };
                state.editor.set_mode(new_mode);
                Ok(format!("Switched to {:?} mode", new_mode))
            }
        }

        // === MOVEMENT KEYS ===
        KeyCode::Left => {
            state.editor.move_left();
            Ok("Moved left".to_string())
        }
        KeyCode::Right => {
            state.editor.move_right();
            Ok("Moved right".to_string())
        }
        KeyCode::Up => {
            state.editor.move_to_next_field(); // TODO: Add move_up method
            Ok("Moved up".to_string())
        }
        KeyCode::Down => {
            state.editor.move_to_next_field(); // TODO: Add move_down method  
            Ok("Moved down".to_string())
        }

        // === TEXT INPUT ===
        KeyCode::Char(c) if !modifiers.contains(KeyModifiers::CONTROL) => {
            state.editor.insert_char(c)
                .map(|_| format!("Inserted '{}'", c))
        }

        KeyCode::Backspace => {
            // TODO: Add delete_backward method to FormEditor
            Ok("Backspace (not implemented yet)".to_string())
        }

        _ => Ok(format!("Unhandled key: {:?}", key)),
    };

    // Update debug message
    match result {
        Ok(msg) => state.debug_message = msg,
        Err(e) => state.debug_message = format!("Error: {}", e),
    }

    true
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut state: AppState) -> io::Result<()> {
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

fn ui(f: &mut Frame, state: &AppState, theme: &DemoTheme) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),
            Constraint::Length(5),
        ])
        .split(f.area());

    // Render the canvas form - much simpler!
    let active_field_rect = render_canvas(
        f,
        chunks[0],
        &state.editor,
        theme,
    );

    // Render autocomplete dropdown if active
    if let Some(input_rect) = active_field_rect {
        render_autocomplete_dropdown(
            f,
            chunks[0],
            input_rect,
            theme,
            &state.editor,
        );
    }

    // Status info
    let autocomplete_status = if state.editor.is_autocomplete_active() {
        if state.editor.ui_state().is_autocomplete_loading() {
            "Loading suggestions..."
        } else if !state.editor.suggestions().is_empty() {
            "Use Tab to navigate, Enter to select, Esc to cancel"
        } else {
            "No suggestions found"
        }
    } else {
        "Tab to trigger autocomplete"
    };

    let status_lines = vec![
        Line::from(Span::raw(format!("Mode: {:?} | Field: {}/{} | Cursor: {}",
            state.editor.mode(), 
            state.editor.current_field() + 1, 
            state.editor.data_provider().field_count(), 
            state.editor.cursor_position()))),
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

    let state = AppState::new();
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
