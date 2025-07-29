// examples/config_screen.rs
//! Advanced configuration screen with suggestions and validation
//!
//! This example demonstrates:
//! - Multiple field types
//! - Auto-suggestions
//! - Field validation
//! - Custom actions
//!
//! Run with: cargo run --example config_screen

use canvas::prelude::*;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use std::io::{self, Write};

#[derive(Debug)]
struct ConfigForm {
    current_field: usize,
    cursor_pos: usize,

    // Configuration fields
    server_host: String,
    server_port: String,
    database_url: String,
    log_level: String,
    max_connections: String,

    has_changes: bool,
    suggestions: SuggestionState,
}

impl ConfigForm {
    fn new() -> Self {
        Self {
            current_field: 0,
            cursor_pos: 0,
            server_host: "localhost".to_string(),
            server_port: "8080".to_string(),
            database_url: String::new(),
            log_level: "info".to_string(),
            max_connections: "100".to_string(),
            has_changes: false,
            suggestions: SuggestionState::default(),
        }
    }

    fn field_names() -> Vec<&'static str> {
        vec![
            "Server Host",
            "Server Port",
            "Database URL",
            "Log Level",
            "Max Connections"
        ]
    }

    fn get_field_value(&self, index: usize) -> &String {
        match index {
            0 => &self.server_host,
            1 => &self.server_port,
            2 => &self.database_url,
            3 => &self.log_level,
            4 => &self.max_connections,
            _ => panic!("Invalid field index: {}", index),
        }
    }

    fn get_field_value_mut(&mut self, index: usize) -> &mut String {
        match index {
            0 => &mut self.server_host,
            1 => &mut self.server_port,
            2 => &mut self.database_url,
            3 => &mut self.log_level,
            4 => &mut self.max_connections,
            _ => panic!("Invalid field index: {}", index),
        }
    }

    fn validate_field(&self, index: usize) -> Option<String> {
        let value = self.get_field_value(index);
        match index {
            0 => { // Server Host
                if value.trim().is_empty() {
                    Some("Server host cannot be empty".to_string())
                } else {
                    None
                }
            }
            1 => { // Server Port
                if let Ok(port) = value.parse::<u16>() {
                    if port == 0 {
                        Some("Port must be greater than 0".to_string())
                    } else {
                        None
                    }
                } else {
                    Some("Port must be a valid number (1-65535)".to_string())
                }
            }
            2 => { // Database URL
                if !value.is_empty() && !value.starts_with("postgresql://") && !value.starts_with("mysql://") && !value.starts_with("sqlite://") {
                    Some("Database URL should start with postgresql://, mysql://, or sqlite://".to_string())
                } else {
                    None
                }
            }
            3 => { // Log Level
                let valid_levels = ["trace", "debug", "info", "warn", "error"];
                if !valid_levels.contains(&value.to_lowercase().as_str()) {
                    Some("Log level must be one of: trace, debug, info, warn, error".to_string())
                } else {
                    None
                }
            }
            4 => { // Max Connections
                if let Ok(connections) = value.parse::<u32>() {
                    if connections == 0 {
                        Some("Max connections must be greater than 0".to_string())
                    } else if connections > 10000 {
                        Some("Max connections seems too high (>10000)".to_string())
                    } else {
                        None
                    }
                } else {
                    Some("Max connections must be a valid number".to_string())
                }
            }
            _ => None,
        }
    }

    fn get_suggestions_for_field(&self, index: usize, current_value: &str) -> Vec<String> {
        match index {
            0 => { // Server Host
                vec![
                    "localhost".to_string(),
                    "127.0.0.1".to_string(),
                    "0.0.0.0".to_string(),
                    format!("{}.local", current_value),
                ]
            }
            1 => { // Server Port
                vec![
                    "8080".to_string(),
                    "3000".to_string(),
                    "8000".to_string(),
                    "80".to_string(),
                    "443".to_string(),
                ]
            }
            2 => { // Database URL
                if current_value.is_empty() {
                    vec![
                        "postgresql://localhost:5432/mydb".to_string(),
                        "mysql://localhost:3306/mydb".to_string(),
                        "sqlite://./database.db".to_string(),
                    ]
                } else {
                    vec![]
                }
            }
            3 => { // Log Level
                vec![
                    "trace".to_string(),
                    "debug".to_string(),
                    "info".to_string(),
                    "warn".to_string(),
                    "error".to_string(),
                ]
                .into_iter()
                .filter(|level| level.starts_with(&current_value.to_lowercase()))
                .collect()
            }
            4 => { // Max Connections
                vec![
                    "10".to_string(),
                    "50".to_string(),
                    "100".to_string(),
                    "200".to_string(),
                    "500".to_string(),
                ]
            }
            _ => vec![],
        }
    }
}

impl CanvasState for ConfigForm {
    fn current_field(&self) -> usize {
        self.current_field
    }

    fn current_cursor_pos(&self) -> usize {
        self.cursor_pos
    }

    fn set_current_field(&mut self, index: usize) {
        self.current_field = index.min(4); // 5 fields total (0-4)
        // Deactivate suggestions when changing fields
        self.deactivate_suggestions();
    }

    fn set_current_cursor_pos(&mut self, pos: usize) {
        self.cursor_pos = pos;
    }

    fn get_current_input(&self) -> &str {
        self.get_field_value(self.current_field)
    }

    fn get_current_input_mut(&mut self) -> &mut String {
        self.get_field_value_mut(self.current_field)
    }

    fn inputs(&self) -> Vec<&String> {
        vec![
            &self.server_host,
            &self.server_port,
            &self.database_url,
            &self.log_level,
            &self.max_connections,
        ]
    }

    fn fields(&self) -> Vec<&str> {
        Self::field_names()
    }

    fn has_unsaved_changes(&self) -> bool {
        self.has_changes
    }

    fn set_has_unsaved_changes(&mut self, changed: bool) {
        self.has_changes = changed;
    }

    // Suggestion support
    fn get_suggestions(&self) -> Option<&[String]> {
        if self.suggestions.is_active {
            Some(&self.suggestions.suggestions)
        } else {
            None
        }
    }

    fn get_selected_suggestion_index(&self) -> Option<usize> {
        self.suggestions.selected_index
    }

    fn set_selected_suggestion_index(&mut self, index: Option<usize>) {
        self.suggestions.selected_index = index;
    }

    fn activate_suggestions(&mut self, suggestions: Vec<String>) {
        self.suggestions.activate_with_suggestions(suggestions);
    }

    fn deactivate_suggestions(&mut self) {
        self.suggestions.deactivate();
    }

    fn handle_feature_action(&mut self, action: &CanvasAction, _context: &ActionContext) -> Option<String> {
        match action {
            CanvasAction::SelectSuggestion => {
                // Fix: Clone the suggestion first to avoid borrow checker issues
                if let Some(suggestion) = self.suggestions.get_selected().cloned() {
                    *self.get_current_input_mut() = suggestion.clone();
                    self.set_current_cursor_pos(suggestion.len());
                    self.deactivate_suggestions();
                    self.set_has_unsaved_changes(true);
                    return Some("Applied suggestion".to_string());
                }
                None
            }

            CanvasAction::Custom(cmd) => match cmd.as_str() {
                "trigger_suggestions" => {
                    let current_value = self.get_current_input();
                    let suggestions = self.get_suggestions_for_field(self.current_field, current_value);
                    if !suggestions.is_empty() {
                        self.activate_suggestions(suggestions);
                        Some("Showing suggestions".to_string())
                    } else {
                        Some("No suggestions available".to_string())
                    }
                }
                "validate_current" => {
                    if let Some(error) = self.validate_field(self.current_field) {
                        Some(format!("Validation Error: {}", error))
                    } else {
                        Some("Field is valid".to_string())
                    }
                }
                "validate_all" => {
                    let mut errors = Vec::new();
                    for i in 0..5 {
                        if let Some(error) = self.validate_field(i) {
                            errors.push(format!("{}: {}", Self::field_names()[i], error));
                        }
                    }
                    if errors.is_empty() {
                        Some("All fields are valid!".to_string())
                    } else {
                        Some(format!("Errors found: {}", errors.join("; ")))
                    }
                }
                "save_config" => {
                    // Validate all fields first
                    for i in 0..5 {
                        if self.validate_field(i).is_some() {
                            return Some("Cannot save: Please fix validation errors first".to_string());
                        }
                    }
                    self.set_has_unsaved_changes(false);
                    Some("Configuration saved successfully!".to_string())
                }
                _ => None,
            },

            // Auto-trigger suggestions for certain fields
            CanvasAction::InsertChar(_) => {
                // After character insertion, check if we should show suggestions
                match self.current_field {
                    3 => { // Log level - always show suggestions for autocomplete
                        let current_value = self.get_current_input();
                        let suggestions = self.get_suggestions_for_field(self.current_field, current_value);
                        if !suggestions.is_empty() {
                            self.activate_suggestions(suggestions);
                        }
                    }
                    _ => {}
                }
                None // Let the generic handler insert the character
            }

            _ => None,
        }
    }
}

fn draw_ui(form: &ConfigForm, message: &str) -> io::Result<()> {
    print!("\x1B[2J\x1B[1;1H");

    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║                      CONFIGURATION EDITOR                     ║");
    println!("╠════════════════════════════════════════════════════════════════╣");

    let field_names = ConfigForm::field_names();

    for (i, field_name) in field_names.iter().enumerate() {
        let is_current = i == form.current_field;
        let indicator = if is_current { ">" } else { " " };
        let value = form.get_field_value(i);
        let display_value = if value.is_empty() {
            format!("<enter {}>", field_name.to_lowercase())
        } else {
            value.clone()
        };

        // Truncate long values for display
        let display_value = if display_value.len() > 35 {
            format!("{}...", &display_value[..32])
        } else {
            display_value
        };

        println!("║ {} {:15}: {:35} ║", indicator, field_name, display_value);

        // Show cursor for current field
        if is_current {
            let cursor_pos = form.cursor_pos.min(value.len());
            let cursor_line = format!("║   {}{}║",
                " ".repeat(18 + cursor_pos),
                "▊"
            );
            println!("{:66}", cursor_line);
        }

        // Show validation error if any
        if let Some(error) = form.validate_field(i) {
            let error_display = if error.len() > 58 {
                format!("{}...", &error[..55])
            } else {
                error
            };
            println!("║   ⚠️  {:58} ║", error_display);
        } else if is_current {
            println!("║{:64}║", "");
        }
    }

    println!("╠════════════════════════════════════════════════════════════════╣");

    // Show suggestions if active
    if let Some(suggestions) = form.get_suggestions() {
        println!("║ SUGGESTIONS:                                                   ║");
        for (i, suggestion) in suggestions.iter().enumerate() {
            let selected = form.get_selected_suggestion_index() == Some(i);
            let marker = if selected { "→" } else { " " };
            let display_suggestion = if suggestion.len() > 55 {
                format!("{}...", &suggestion[..52])
            } else {
                suggestion.clone()
            };
            println!("║ {} {:58} ║", marker, display_suggestion);
        }
        println!("╠════════════════════════════════════════════════════════════════╣");
    }

    println!("║ CONTROLS:                                                      ║");
    println!("║   Tab/↑↓      - Navigate fields                                ║");
    println!("║   Ctrl+Space  - Show suggestions                               ║");
    println!("║   ↑↓          - Navigate suggestions (when shown)             ║");
    println!("║   Enter       - Select suggestion / Validate field            ║");
    println!("║   Ctrl+S      - Save configuration                             ║");
    println!("║   Ctrl+V      - Validate all fields                           ║");
    println!("║   Ctrl+C      - Exit                                           ║");
    println!("╠════════════════════════════════════════════════════════════════╣");

    // Status
    let status = if !message.is_empty() {
        message.to_string()
    } else if form.has_changes {
        "Configuration modified - press Ctrl+S to save".to_string()
    } else {
        "Ready".to_string()
    };

    let status_display = if status.len() > 58 {
        format!("{}...", &status[..55])
    } else {
        status
    };

    println!("║ Status: {:55} ║", status_display);
    println!("╚════════════════════════════════════════════════════════════════╝");

    io::stdout().flush()?;
    Ok(())
}

#[tokio::main]
async fn main() -> io::Result<()> {
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;

    let mut form = ConfigForm::new();
    let mut ideal_cursor = 0;
    let mut message = String::new();

    draw_ui(&form, &message)?;

    loop {
        if let Event::Key(key) = event::read()? {
            if !message.is_empty() {
                message.clear();
            }

            match key {
                // Exit
                KeyEvent { code: KeyCode::Char('c'), modifiers: KeyModifiers::CONTROL, .. } => {
                    break;
                }

                // Show suggestions
                KeyEvent { code: KeyCode::Char(' '), modifiers: KeyModifiers::CONTROL, .. } => {
                    let result = ActionDispatcher::dispatch(
                        CanvasAction::Custom("trigger_suggestions".to_string()),
                        &mut form,
                        &mut ideal_cursor,
                    ).await.unwrap();

                    if let Some(msg) = result.message() {
                        message = msg.to_string();
                    }
                }

                // Validate current field or select suggestion
                KeyEvent { code: KeyCode::Enter, .. } => {
                    if form.get_suggestions().is_some() {
                        // Select suggestion
                        let result = ActionDispatcher::dispatch(
                            CanvasAction::SelectSuggestion,
                            &mut form,
                            &mut ideal_cursor,
                        ).await.unwrap();

                        if let Some(msg) = result.message() {
                            message = msg.to_string();
                        }
                    } else {
                        // Validate current field
                        let result = ActionDispatcher::dispatch(
                            CanvasAction::Custom("validate_current".to_string()),
                            &mut form,
                            &mut ideal_cursor,
                        ).await.unwrap();

                        if let Some(msg) = result.message() {
                            message = msg.to_string();
                        }
                    }
                }

                // Save configuration
                KeyEvent { code: KeyCode::Char('s'), modifiers: KeyModifiers::CONTROL, .. } => {
                    let result = ActionDispatcher::dispatch(
                        CanvasAction::Custom("save_config".to_string()),
                        &mut form,
                        &mut ideal_cursor,
                    ).await.unwrap();

                    if let Some(msg) = result.message() {
                        message = msg.to_string();
                    }
                }

                // Validate all fields
                KeyEvent { code: KeyCode::Char('v'), modifiers: KeyModifiers::CONTROL, .. } => {
                    let result = ActionDispatcher::dispatch(
                        CanvasAction::Custom("validate_all".to_string()),
                        &mut form,
                        &mut ideal_cursor,
                    ).await.unwrap();

                    if let Some(msg) = result.message() {
                        message = msg.to_string();
                    }
                }

                // Handle up/down for suggestions
                KeyEvent { code: KeyCode::Up, .. } => {
                    let action = if form.get_suggestions().is_some() {
                        CanvasAction::SuggestionUp
                    } else {
                        CanvasAction::MoveUp
                    };

                    let _ = ActionDispatcher::dispatch(action, &mut form, &mut ideal_cursor).await;
                }

                KeyEvent { code: KeyCode::Down, .. } => {
                    let action = if form.get_suggestions().is_some() {
                        CanvasAction::SuggestionDown
                    } else {
                        CanvasAction::MoveDown
                    };

                    let _ = ActionDispatcher::dispatch(action, &mut form, &mut ideal_cursor).await;
                }

                // Handle escape to close suggestions
                KeyEvent { code: KeyCode::Esc, .. } => {
                    if form.get_suggestions().is_some() {
                        let _ = ActionDispatcher::dispatch(
                            CanvasAction::ExitSuggestions,
                            &mut form,
                            &mut ideal_cursor,
                        ).await;
                    }
                }

                // Regular key handling
                _ => {
                    if let Some(action) = CanvasAction::from_key(key.code) {
                        let result = ActionDispatcher::dispatch(action, &mut form, &mut ideal_cursor).await.unwrap();

                        if !result.is_success() {
                            if let Some(msg) = result.message() {
                                message = format!("Error: {}", msg);
                            }
                        }
                    }
                }
            }

            draw_ui(&form, &message)?;
        }
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    println!("Configuration editor closed!");

    Ok(())
}
