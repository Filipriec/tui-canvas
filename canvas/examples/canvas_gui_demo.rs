use std::io;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
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
    canvas::{
        gui::render_canvas,
        modes::{AppMode, HighlightState, ModeManager},
        state::{ActionContext, CanvasState},
        theme::CanvasTheme,
    },
    config::CanvasConfig,
    dispatcher::ActionDispatcher,
    CanvasAction,
};

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

// Demo form state
struct DemoFormState {
    fields: Vec<String>,
    field_names: Vec<String>,
    current_field: usize,
    cursor_pos: usize,
    mode: AppMode,
    highlight_state: HighlightState,
    has_changes: bool,
    ideal_cursor_column: usize,
    last_action: Option<String>,
    debug_message: String,
}

impl DemoFormState {
    fn new() -> Self {
        Self {
            fields: vec![
                "John Doe".to_string(), // Name - has words to test with
                "john.doe@example.com".to_string(), // Email - has punctuation
                "+1 234 567 8900".to_string(), // Phone - has spaces and numbers
                "123 Main Street Apt 4B".to_string(), // Address - multiple words
                "San Francisco".to_string(), // City - two words
                "This is a test comment with multiple words".to_string(), // Comments - lots of words
            ],
            field_names: vec![
                "Name".to_string(),
                "Email".to_string(),
                "Phone".to_string(),
                "Address".to_string(),
                "City".to_string(),
                "Comments".to_string(),
            ],
            current_field: 0,
            cursor_pos: 0,
            mode: AppMode::ReadOnly,
            highlight_state: HighlightState::Off,
            has_changes: false,
            ideal_cursor_column: 0,
            last_action: None,
            debug_message: "Ready - Form loaded with sample data".to_string(),
        }
    }

    fn enter_edit_mode(&mut self) {
        if ModeManager::can_enter_edit_mode(self.mode) {
            self.mode = AppMode::Edit;
            self.debug_message = "Entered EDIT mode".to_string();
        }
    }

    fn enter_readonly_mode(&mut self) {
        if ModeManager::can_enter_read_only_mode(self.mode) {
            self.mode = AppMode::ReadOnly;
            self.highlight_state = HighlightState::Off;
            self.debug_message = "Entered READ-ONLY mode".to_string();
        }
    }

    fn enter_highlight_mode(&mut self) {
        if ModeManager::can_enter_highlight_mode(self.mode) {
            self.mode = AppMode::Highlight;
            self.highlight_state = HighlightState::Characterwise {
                anchor: (self.current_field, self.cursor_pos),
            };
            self.debug_message = "Entered VISUAL mode".to_string();
        }
    }

    fn log_action(&mut self, action: &str) {
        self.last_action = Some(action.to_string());
        self.debug_message = format!("Action: {}", action);
    }
}

impl CanvasState for DemoFormState {
    fn current_field(&self) -> usize {
        self.current_field
    }

    fn current_cursor_pos(&self) -> usize {
        self.cursor_pos
    }

    fn set_current_field(&mut self, index: usize) {
        self.current_field = index.min(self.fields.len().saturating_sub(1));
        // Reset cursor to end of field when switching
        self.cursor_pos = self.fields[self.current_field].len();
    }

    fn set_current_cursor_pos(&mut self, pos: usize) {
        let max_pos = self.fields[self.current_field].len();
        self.cursor_pos = pos.min(max_pos);
    }

    fn current_mode(&self) -> AppMode {
        self.mode
    }

    fn get_current_input(&self) -> &str {
        &self.fields[self.current_field]
    }

    fn get_current_input_mut(&mut self) -> &mut String {
        &mut self.fields[self.current_field]
    }

    fn inputs(&self) -> Vec<&String> {
        self.fields.iter().collect()
    }

    fn fields(&self) -> Vec<&str> {
        self.field_names.iter().map(|s| s.as_str()).collect()
    }

    fn has_unsaved_changes(&self) -> bool {
        self.has_changes
    }

    fn set_has_unsaved_changes(&mut self, changed: bool) {
        self.has_changes = changed;
    }

    fn handle_feature_action(&mut self, action: &CanvasAction, _context: &ActionContext) -> Option<String> {
        // FOCUS: Debug specifically for 'w' key (move_word_next)
        if let CanvasAction::MoveWordNext = action {
            let current_input = self.get_current_input();
            let old_cursor = self.cursor_pos;
            self.debug_message = format!("🔍 MoveWordNext: cursor {} -> text '{}' (len {})", 
                old_cursor, current_input, current_input.len());
            
            // Return None to let the handler process it, but we'll see this debug message
            return None;
        }

        match action {
            CanvasAction::Custom(cmd) => {
                match cmd.as_str() {
                    "enter_edit_mode" => {
                        self.enter_edit_mode();
                        Some("Entered edit mode".to_string())
                    }
                    "enter_readonly_mode" => {
                        self.enter_readonly_mode();
                        Some("Entered read-only mode".to_string())
                    }
                    "enter_highlight_mode" => {
                        self.enter_highlight_mode();
                        Some("Entered highlight mode".to_string())
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut state: DemoFormState, config: CanvasConfig) -> io::Result<()> {
    let theme = DemoTheme;

    loop {
        terminal.draw(|f| ui(f, &state, &theme))?;

        if let Event::Key(key) = event::read()? {
            // BASIC DEBUG: Show EVERY key press for j, k, w
            match key.code {
                KeyCode::Char('j') | KeyCode::Char('k') | KeyCode::Char('w') => {
                    println!("🔥 KEY PRESSED: {:?} with modifiers {:?}", key.code, key.modifiers);
                }
                _ => {}
            }
            
            // Handle quit - multiple options
            if (key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL)) ||
               (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)) ||
               key.code == KeyCode::F(10) {
                break;
            }

            let is_edit_mode = state.mode == AppMode::Edit;
            let mut handled = false;

            // Debug: Show what key was pressed and check config lookup
            let key_debug = format!("{:?}", key.code);
            let config_action = if is_edit_mode {
                config.get_edit_action(key.code, key.modifiers)
            } else {
                config.get_read_only_action(key.code, key.modifiers)
            };
            
            // FOCUS: Special debug for j, k, w keys
            match key.code {
                KeyCode::Char('j') => {
                    println!("🔥 J KEY: Config action: {:?}", config_action);
                    state.debug_message = format!("🔍 'j' KEY: Mode={} | Config action: {:?}", 
                        if is_edit_mode { "EDIT" } else { "READ-ONLY" }, config_action);
                }
                KeyCode::Char('k') => {
                    println!("🔥 K KEY: Config action: {:?}", config_action);
                    state.debug_message = format!("🔍 'k' KEY: Mode={} | Config action: {:?}", 
                        if is_edit_mode { "EDIT" } else { "READ-ONLY" }, config_action);
                }
                KeyCode::Char('w') => {
                    println!("🔥 W KEY: Config action: {:?}", config_action);
                    state.debug_message = format!("🔍 'w' KEY: Mode={} | Config action: {:?} | Current pos: {} | Text: '{}'", 
                        if is_edit_mode { "EDIT" } else { "READ-ONLY" },
                        config_action,
                        state.cursor_pos,
                        state.get_current_input());
                }
                _ => {
                    state.debug_message = format!("Key: {} | Mods: {:?} | Mode: {} | Config found: {:?}", 
                        key_debug, key.modifiers,
                        if is_edit_mode { "EDIT" } else { "READ-ONLY" },
                        config_action);
                }
            }

            // First priority: Try to dispatch through your config system
            let mut ideal_cursor = state.ideal_cursor_column;
            let old_cursor_pos = state.cursor_pos; // Track cursor before action
            
            // EXTRA DEBUG for w key
            if key.code == KeyCode::Char('w') {
                println!("🔥 W KEY: About to call ActionDispatcher::dispatch_key");
                println!("🔥 W KEY: cursor before = {}, text = '{}'", old_cursor_pos, state.get_current_input());
            }
            
            if let Ok(Some(result)) = ActionDispatcher::dispatch_key(
                key.code,
                key.modifiers,
                &mut state,
                &mut ideal_cursor,
                is_edit_mode,
                false, // no autocomplete suggestions
            ).await {
                state.ideal_cursor_column = ideal_cursor;
                
                let new_cursor_pos = state.cursor_pos; // Track cursor after action
                
                // FOCUS: Special debug for 'w' key
                if key.code == KeyCode::Char('w') {
                    println!("SUCCESS W KEY PROCESSED: cursor {} -> {} | text: '{}'", old_cursor_pos, new_cursor_pos, state.get_current_input());
                    state.debug_message = format!("SUCCESS 'w' PROCESSED: cursor {} -> {} | text: '{}'", 
                        old_cursor_pos, new_cursor_pos, state.get_current_input());
                } else {
                    state.debug_message = format!("SUCCESS Config handled: {} -> {}", key_debug,
                        result.message().unwrap_or("success"));
                }
                
                // Mark as changed for text modification keys in edit mode
                if is_edit_mode {
                    match key.code {
                        KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Delete => {
                            state.set_has_unsaved_changes(true);
                        }
                        _ => {}
                    }
                }
                handled = true;
            } else {
                // Debug dispatch failures
                if key.code == KeyCode::Char('w') {
                    println!("FAILED W KEY: ActionDispatcher::dispatch_key returned None or Error");
                    
                    // Try calling dispatch_with_config directly to see the error
                    let action = CanvasAction::MoveWordNext;
                    println!("FAILED W KEY: Trying direct dispatch of MoveWordNext action");
                    
                    match ActionDispatcher::dispatch_with_config(
                        action,
                        &mut state,
                        &mut ideal_cursor,
                        Some(&config),
                    ).await {
                        Ok(result) => {
                            println!("FAILED W KEY: Direct dispatch SUCCESS: {:?}", result);
                            state.debug_message = "Direct dispatch worked!".to_string();
                        }
                        Err(e) => {
                            println!("FAILED W KEY: Direct dispatch ERROR: {:?}", e);
                            state.debug_message = format!("Direct dispatch error: {:?}", e);
                        }
                    }
                }
            }

            // Second priority: Handle character input in edit mode (if not handled by config)
            if !handled && is_edit_mode {
                if let KeyCode::Char(c) = key.code {
                    if !key.modifiers.contains(KeyModifiers::CONTROL) && !key.modifiers.contains(KeyModifiers::ALT) {
                        let action = CanvasAction::InsertChar(c);
                        let mut ideal_cursor = state.ideal_cursor_column;
                        if let Ok(_) = ActionDispatcher::dispatch_with_config(
                            action,
                            &mut state,
                            &mut ideal_cursor,
                            Some(&config),
                        ).await {
                            state.ideal_cursor_column = ideal_cursor;
                            state.set_has_unsaved_changes(true);
                            state.debug_message = format!("Inserted char: '{}'", c);
                            handled = true;
                        }
                    }
                }
            }

            // Third priority: Fallback mode transitions (if not handled by config)
            if !handled {
                match (state.mode, key.code) {
                    // ReadOnly -> Edit mode fallbacks
                    (AppMode::ReadOnly, KeyCode::Char('i') | KeyCode::Char('a') | KeyCode::Insert) => {
                        state.enter_edit_mode();
                        if key.code == KeyCode::Char('a') {
                            state.cursor_pos = state.fields[state.current_field].len();
                        }
                        state.debug_message = format!("Fallback: entered edit mode via {:?}", key.code);
                        handled = true;
                    }
                    // ReadOnly -> Visual mode fallback
                    (AppMode::ReadOnly, KeyCode::Char('v')) => {
                        state.enter_highlight_mode();
                        state.debug_message = "Fallback: entered visual mode via 'v'".to_string();
                        handled = true;
                    }
                    // Any mode -> ReadOnly fallback
                    (_, KeyCode::Esc) => {
                        state.enter_readonly_mode();
                        state.debug_message = "Fallback: entered read-only mode via Esc".to_string();
                        handled = true;
                    }
                    _ => {}
                }
            }

            // If nothing handled the key, show more debug info
            if !handled {
                let available_actions: Vec<String> = if is_edit_mode {
                    config.keybindings.edit.keys().cloned().collect()
                } else {
                    config.keybindings.read_only.keys().cloned().collect()
                };
                
                state.debug_message = format!("❌ Unhandled: {} | Available actions: {}", 
                    key_debug, 
                    available_actions.join(", "));
            }
        }
    }

    Ok(())
}

fn ui(f: &mut Frame, state: &DemoFormState, theme: &DemoTheme) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),    // Main form area
            Constraint::Length(4), // Status area (increased for debug info)
        ])
        .split(f.area());

    // Render the canvas form
    render_canvas(
        f,
        chunks[0],
        state,
        theme,
        state.mode == AppMode::Edit,
        &state.highlight_state,
    );

    // Render status bar with debug info
    let mode_text = match state.mode {
        AppMode::Edit => "EDIT",
        AppMode::ReadOnly => "NORMAL",
        AppMode::Highlight => "VISUAL",
        AppMode::General => "GENERAL",
        AppMode::Command => "COMMAND",
    };

    let status_text = if state.has_changes {
        format!("-- {} -- [Modified]", mode_text)
    } else {
        format!("-- {} --", mode_text)
    };

    let position_text = format!("Field: {}/{} | Cursor: {} | Column: {}", 
        state.current_field + 1, 
        state.fields.len(), 
        state.cursor_pos, 
        state.ideal_cursor_column);

    let help_text = match state.mode {
        AppMode::ReadOnly => "hjkl/arrows: Move | Tab/Shift+Tab: Fields | w/b/e: Words | 0/$: Line | gg/G: File | i/a: Edit | v: Visual | F10: Quit",
        AppMode::Edit => "Type to edit | hjkl/arrows: Move | Tab/Enter: Next field | Backspace/Delete: Delete | Home/End: Line | Esc: Normal | F10: Quit", 
        AppMode::Highlight => "hjkl/arrows: Select | w/b/e: Words | 0/$: Line | Esc: Normal | F10: Quit",
        _ => "Esc: Normal | F10: Quit",
    };

    let status = Paragraph::new(vec![
        Line::from(Span::styled(status_text, Style::default().fg(theme.accent()))),
        Line::from(Span::styled(position_text, Style::default().fg(theme.fg()))),
        Line::from(Span::styled(state.debug_message.clone(), Style::default().fg(theme.warning()))),
        Line::from(Span::styled(help_text, Style::default().fg(theme.secondary()))),
    ])
    .block(Block::default().borders(Borders::ALL).title("Status & Debug"));

    f.render_widget(status, chunks[1]);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = CanvasConfig::load();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create demo state
    let state = DemoFormState::new();

    // Run app
    let res = run_app(&mut terminal, state, config).await;

    // Restore terminal
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
