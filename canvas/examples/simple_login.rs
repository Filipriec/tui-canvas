// examples/simple_login.rs
//! A simple login form demonstrating basic canvas usage
//!
//! Run with: cargo run --example simple_login

use canvas::prelude::*;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use std::io::{self, Write};

#[derive(Debug)]
struct LoginForm {
    current_field: usize,
    cursor_pos: usize,
    username: String,
    password: String,
    has_changes: bool,
}

impl LoginForm {
    fn new() -> Self {
        Self {
            current_field: 0,
            cursor_pos: 0,
            username: String::new(),
            password: String::new(),
            has_changes: false,
        }
    }

    fn reset(&mut self) {
        self.username.clear();
        self.password.clear();
        self.current_field = 0;
        self.cursor_pos = 0;
        self.has_changes = false;
    }

    fn is_valid(&self) -> bool {
        !self.username.trim().is_empty() && !self.password.trim().is_empty()
    }
}

impl CanvasState for LoginForm {
    fn current_field(&self) -> usize {
        self.current_field
    }

    fn current_cursor_pos(&self) -> usize {
        self.cursor_pos
    }

    fn set_current_field(&mut self, index: usize) {
        self.current_field = index.min(1); // Only 2 fields: username(0), password(1)
    }

    fn set_current_cursor_pos(&mut self, pos: usize) {
        self.cursor_pos = pos;
    }

    fn get_current_input(&self) -> &str {
        match self.current_field {
            0 => &self.username,
            1 => &self.password,
            _ => "",
        }
    }

    fn get_current_input_mut(&mut self) -> &mut String {
        match self.current_field {
            0 => &mut self.username,
            1 => &mut self.password,
            _ => unreachable!(),
        }
    }

    fn inputs(&self) -> Vec<&String> {
        vec![&self.username, &self.password]
    }

    fn fields(&self) -> Vec<&str> {
        vec!["Username", "Password"]
    }

    fn has_unsaved_changes(&self) -> bool {
        self.has_changes
    }

    fn set_has_unsaved_changes(&mut self, changed: bool) {
        self.has_changes = changed;
    }

    // Custom action handling
    fn handle_feature_action(&mut self, action: &CanvasAction, _context: &ActionContext) -> Option<String> {
        match action {
            CanvasAction::Custom(cmd) => match cmd.as_str() {
                "submit" => {
                    if self.is_valid() {
                        Some(format!("Login successful! Welcome, {}", self.username))
                    } else {
                        Some("Error: Username and password are required".to_string())
                    }
                }
                "clear" => {
                    self.reset();
                    Some("Form cleared".to_string())
                }
                _ => None,
            },
            _ => None,
        }
    }

    // Override display for password field
    fn get_display_value_for_field(&self, index: usize) -> &str {
        match index {
            0 => &self.username, // Username shows normally
            1 => &self.password, // We'll handle masking in the UI drawing
            _ => "",
        }
    }

    fn has_display_override(&self, index: usize) -> bool {
        index == 1 // Password field has display override
    }
}

fn draw_ui(form: &LoginForm, message: &str) -> io::Result<()> {
    // Clear screen and move cursor to top-left
    print!("\x1B[2J\x1B[1;1H");

    println!("╔═══════════════════════════════════════╗");
    println!("║              LOGIN FORM               ║");
    println!("╠═══════════════════════════════════════╣");

    // Username field
    let username_indicator = if form.current_field == 0 { "→" } else { " " };
    let username_display = if form.username.is_empty() {
        "<enter username>".to_string()
    } else {
        form.username.clone()
    };
    println!("║ {} Username: {:22} ║", username_indicator, 
             if username_display.len() > 22 { 
                 format!("{}...", &username_display[..19])
             } else { 
                 format!("{:22}", username_display) 
             });

    // Show cursor for username field
    if form.current_field == 0 && !form.username.is_empty() {
        let cursor_pos = form.cursor_pos.min(form.username.len());
        let spaces_before = 11 + cursor_pos; // "Username: " = 10 chars + 1 space
        let cursor_line = format!("║   {}█{:width$}║", 
            " ".repeat(spaces_before), 
            "",
            width = 25_usize.saturating_sub(spaces_before)
        );
        println!("{}", cursor_line);
    } else {
        println!("║{:37}║", "");
    }

    // Password field
    let password_indicator = if form.current_field == 1 { "→" } else { " " };
    let password_display = if form.password.is_empty() {
        "<enter password>".to_string()
    } else {
        "*".repeat(form.password.len())
    };
    println!("║ {} Password: {:22} ║", password_indicator, 
             if password_display.len() > 22 { 
                 format!("{}...", &password_display[..19])
             } else { 
                 format!("{:22}", password_display) 
             });

    // Show cursor for password field
    if form.current_field == 1 && !form.password.is_empty() {
        let cursor_pos = form.cursor_pos.min(form.password.len());
        let spaces_before = 11 + cursor_pos; // "Password: " = 10 chars + 1 space
        let cursor_line = format!("║   {}█{:width$}║", 
            " ".repeat(spaces_before), 
            "",
            width = 25_usize.saturating_sub(spaces_before)
        );
        println!("{}", cursor_line);
    } else {
        println!("║{:37}║", "");
    }

    println!("╠═══════════════════════════════════════╣");
    println!("║ CONTROLS:                             ║");
    println!("║   Tab/↑↓    - Navigate fields         ║");
    println!("║   Enter     - Submit form             ║");
    println!("║   Ctrl+R    - Clear form              ║");
    println!("║   Ctrl+C    - Exit                    ║");
    println!("╠═══════════════════════════════════════╣");

    // Status message
    let status = if !message.is_empty() {
        message.to_string()
    } else if form.has_changes {
        "Form modified".to_string()
    } else {
        "Ready - enter your credentials".to_string()
    };

    let status_display = if status.len() > 33 {
        format!("{}...", &status[..30])
    } else {
        format!("{:33}", status)
    };

    println!("║ Status: {} ║", status_display);
    println!("╚═══════════════════════════════════════╝");

    // Show current state info
    println!();
    println!("Current field: {} ({})", 
             form.current_field, 
             form.fields()[form.current_field]);
    println!("Cursor position: {}", form.cursor_pos);
    println!("Has changes: {}", form.has_changes);

    io::stdout().flush()?;
    Ok(())
}

#[tokio::main]
async fn main() -> io::Result<()> {
    println!("Starting Canvas Login Demo...");
    println!("Setting up terminal...");

    // Setup terminal
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;

    let mut form = LoginForm::new();
    let mut ideal_cursor = 0;
    let mut message = String::new();

    // Initial draw
    if let Err(e) = draw_ui(&form, &message) {
        // Cleanup on error
        let _ = disable_raw_mode();
        let _ = io::stdout().execute(LeaveAlternateScreen);
        return Err(e);
    }

    println!("Canvas Login Demo started. Use Ctrl+C to exit.");

    loop {
        match event::read() {
            Ok(Event::Key(key)) => {
                // Clear message after key press
                if !message.is_empty() {
                    message.clear();
                }

                match key {
                    // Exit
                    KeyEvent { code: KeyCode::Char('c'), modifiers: KeyModifiers::CONTROL, .. } => {
                        break;
                    }

                    // Clear form
                    KeyEvent { code: KeyCode::Char('r'), modifiers: KeyModifiers::CONTROL, .. } => {
                        match ActionDispatcher::dispatch(
                            CanvasAction::Custom("clear".to_string()),
                            &mut form,
                            &mut ideal_cursor,
                        ).await {
                            Ok(result) => {
                                if let Some(msg) = result.message() {
                                    message = msg.to_string();
                                }
                            }
                            Err(e) => {
                                message = format!("Error: {}", e);
                            }
                        }
                    }

                    // Submit form
                    KeyEvent { code: KeyCode::Enter, .. } => {
                        match ActionDispatcher::dispatch(
                            CanvasAction::Custom("submit".to_string()),
                            &mut form,
                            &mut ideal_cursor,
                        ).await {
                            Ok(result) => {
                                if let Some(msg) = result.message() {
                                    message = msg.to_string();
                                }
                            }
                            Err(e) => {
                                message = format!("Error: {}", e);
                            }
                        }
                    }

                    // Regular key handling - let canvas handle it!
                    _ => {
                        if let Some(action) = CanvasAction::from_key(key.code) {
                            match ActionDispatcher::dispatch(action, &mut form, &mut ideal_cursor).await {
                                Ok(result) => {
                                    if !result.is_success() {
                                        if let Some(msg) = result.message() {
                                            message = format!("Error: {}", msg);
                                        }
                                    }
                                }
                                Err(e) => {
                                    message = format!("Error: {}", e);
                                }
                            }
                        }
                    }
                }

                // Redraw UI
                if let Err(e) = draw_ui(&form, &message) {
                    eprintln!("Error drawing UI: {}", e);
                    break;
                }
            }
            Ok(_) => {
                // Ignore other events (mouse, resize, etc.)
            }
            Err(e) => {
                message = format!("Event error: {}", e);
                if let Err(_) = draw_ui(&form, &message) {
                    break;
                }
            }
        }
    }

    // Cleanup
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    println!("Thanks for using Canvas Login Demo!");
    println!("Final form state:");
    println!("  Username: '{}'", form.username);
    println!("  Password: '{}'", "*".repeat(form.password.len()));
    println!("  Valid: {}", form.is_valid());

    Ok(())
}
