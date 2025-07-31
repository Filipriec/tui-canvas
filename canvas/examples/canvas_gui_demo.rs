// examples/canvas_gui_demo.rs

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
    CanvasAction, execute,
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
    debug_message: String,
}

impl DemoFormState {
    fn new() -> Self {
        Self {
            fields: vec![
                "John Doe".to_string(),
                "john.doe@example.com".to_string(),
                "+1 234 567 8900".to_string(),
                "123 Main Street Apt 4B".to_string(),
                "San Francisco".to_string(),
                "This is a test comment with multiple words".to_string(),
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
            debug_message: "Ready - Use hjkl to move, w for next word, i to edit".to_string(),
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

/// Simple key mapping - users have full control!
async fn handle_key_press(key: KeyCode, modifiers: KeyModifiers, state: &mut DemoFormState) -> bool {
    let is_edit_mode = state.mode == AppMode::Edit;
    
    // Handle quit first
    if (key == KeyCode::Char('q') && modifiers.contains(KeyModifiers::CONTROL)) ||
       (key == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL)) ||
       key == KeyCode::F(10) {
        return false; // Signal to quit
    }

    // Users directly map keys to actions - no configuration needed!
    let action = match (state.mode, key, modifiers) {
        // === READ-ONLY MODE KEYS ===
        (AppMode::ReadOnly, KeyCode::Char('h'), _) => Some(CanvasAction::MoveLeft),
        (AppMode::ReadOnly, KeyCode::Char('j'), _) => Some(CanvasAction::MoveDown),
        (AppMode::ReadOnly, KeyCode::Char('k'), _) => Some(CanvasAction::MoveUp),
        (AppMode::ReadOnly, KeyCode::Char('l'), _) => Some(CanvasAction::MoveRight),
        (AppMode::ReadOnly, KeyCode::Char('w'), _) => Some(CanvasAction::MoveWordNext),
        (AppMode::ReadOnly, KeyCode::Char('b'), _) => Some(CanvasAction::MoveWordPrev),
        (AppMode::ReadOnly, KeyCode::Char('e'), _) => Some(CanvasAction::MoveWordEnd),
        (AppMode::ReadOnly, KeyCode::Char('0'), _) => Some(CanvasAction::MoveLineStart),
        (AppMode::ReadOnly, KeyCode::Char('$'), _) => Some(CanvasAction::MoveLineEnd),
        (AppMode::ReadOnly, KeyCode::Tab, _) => Some(CanvasAction::NextField),
        (AppMode::ReadOnly, KeyCode::BackTab, _) => Some(CanvasAction::PrevField),
        
        // === EDIT MODE KEYS ===
        (AppMode::Edit, KeyCode::Left, _) => Some(CanvasAction::MoveLeft),
        (AppMode::Edit, KeyCode::Right, _) => Some(CanvasAction::MoveRight),
        (AppMode::Edit, KeyCode::Up, _) => Some(CanvasAction::MoveUp),
        (AppMode::Edit, KeyCode::Down, _) => Some(CanvasAction::MoveDown),
        (AppMode::Edit, KeyCode::Home, _) => Some(CanvasAction::MoveLineStart),
        (AppMode::Edit, KeyCode::End, _) => Some(CanvasAction::MoveLineEnd),
        (AppMode::Edit, KeyCode::Backspace, _) => Some(CanvasAction::DeleteBackward),
        (AppMode::Edit, KeyCode::Delete, _) => Some(CanvasAction::DeleteForward),
        (AppMode::Edit, KeyCode::Tab, _) => Some(CanvasAction::NextField),
        (AppMode::Edit, KeyCode::BackTab, _) => Some(CanvasAction::PrevField),
        
        // Vim-style movement in edit mode (optional)
        (AppMode::Edit, KeyCode::Char('h'), m) if m.contains(KeyModifiers::CONTROL) => Some(CanvasAction::MoveLeft),
        (AppMode::Edit, KeyCode::Char('l'), m) if m.contains(KeyModifiers::CONTROL) => Some(CanvasAction::MoveRight),
        
        // Word movement with Ctrl in edit mode
        (AppMode::Edit, KeyCode::Left, m) if m.contains(KeyModifiers::CONTROL) => Some(CanvasAction::MoveWordPrev),
        (AppMode::Edit, KeyCode::Right, m) if m.contains(KeyModifiers::CONTROL) => Some(CanvasAction::MoveWordNext),
        
        // === MODE TRANSITIONS === 
        (AppMode::ReadOnly, KeyCode::Char('i'), _) => Some(CanvasAction::Custom("enter_edit_mode".to_string())),
        (AppMode::ReadOnly, KeyCode::Char('a'), _) => {
            // 'a' moves to end of line then enters edit mode
            if let Ok(_) = execute(CanvasAction::MoveLineEnd, state).await {
                Some(CanvasAction::Custom("enter_edit_mode".to_string()))
            } else {
                None
            }
        },
        (AppMode::ReadOnly, KeyCode::Char('v'), _) => Some(CanvasAction::Custom("enter_highlight_mode".to_string())),
        (_, KeyCode::Esc, _) => Some(CanvasAction::Custom("enter_readonly_mode".to_string())),
        
        // === CHARACTER INPUT IN EDIT MODE ===
        (AppMode::Edit, KeyCode::Char(c), m) if !m.contains(KeyModifiers::CONTROL) && !m.contains(KeyModifiers::ALT) => {
            Some(CanvasAction::InsertChar(c))
        },
        
        // === ARROW KEYS IN READ-ONLY MODE ===
        (AppMode::ReadOnly, KeyCode::Left, _) => Some(CanvasAction::MoveLeft),
        (AppMode::ReadOnly, KeyCode::Right, _) => Some(CanvasAction::MoveRight),
        (AppMode::ReadOnly, KeyCode::Up, _) => Some(CanvasAction::MoveUp),
        (AppMode::ReadOnly, KeyCode::Down, _) => Some(CanvasAction::MoveDown),
        
        _ => None,
    };

    // Execute the action if we found one
    if let Some(action) = action {
        match execute(action.clone(), state).await {
            Ok(result) => {
                if result.is_success() {
                    // Mark as changed for editing actions
                    if is_edit_mode {
                        match action {
                            CanvasAction::InsertChar(_) | CanvasAction::DeleteBackward | CanvasAction::DeleteForward => {
                                state.set_has_unsaved_changes(true);
                            }
                            _ => {}
                        }
                    }
                    
                    if let Some(msg) = result.message() {
                        state.debug_message = msg.to_string();
                    } else {
                        state.debug_message = format!("Executed: {}", action.description());
                    }
                } else if let Some(msg) = result.message() {
                    state.debug_message = format!("Error: {}", msg);
                }
            }
            Err(e) => {
                state.debug_message = format!("Error executing action: {}", e);
            }
        }
    } else {
        state.debug_message = format!("Unhandled key: {:?} (mode: {:?})", key, state.mode);
    }

    true // Continue running
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut state: DemoFormState) -> io::Result<()> {
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

fn ui(f: &mut Frame, state: &DemoFormState, theme: &DemoTheme) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),
            Constraint::Length(4),
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

    // Render status bar
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

    let position_text = format!("Field: {}/{} | Cursor: {} | Actions: {}", 
        state.current_field + 1,
        state.fields.len(),
        state.cursor_pos,
        CanvasAction::movement_actions().len() + CanvasAction::editing_actions().len());

    let help_text = match state.mode {
        AppMode::ReadOnly => "hjkl/arrows: Move | Tab/Shift+Tab: Fields | w/b/e: Words | 0/$: Line | i/a: Edit | v: Visual | F10: Quit",
        AppMode::Edit => "Type to edit | Arrows/Ctrl+arrows: Move | Tab: Next field | Backspace/Delete: Delete | Home/End: Line | Esc: Normal | F10: Quit",
        AppMode::Highlight => "hjkl/arrows: Select | w/b/e: Words | 0/$: Line | Esc: Normal | F10: Quit",
        _ => "Esc: Normal | F10: Quit",
    };

    let status = Paragraph::new(vec![
        Line::from(Span::styled(status_text, Style::default().fg(theme.accent()))),
        Line::from(Span::styled(position_text, Style::default().fg(theme.fg()))),
        Line::from(Span::styled(state.debug_message.clone(), Style::default().fg(theme.warning()))),
        Line::from(Span::styled(help_text, Style::default().fg(theme.secondary()))),
    ])
    .block(Block::default().borders(Borders::ALL).title("Status"));

    f.render_widget(status, chunks[1]);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let state = DemoFormState::new();

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
