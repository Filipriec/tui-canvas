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
            ideal_cursor_column: 0,
            last_action: None,
            debug_message: "Ready".to_string(),
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

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut state: DemoFormState, config: CanvasConfig) -> io::Result<()> {
    let theme = DemoTheme;

    loop {
        terminal.draw(|f| ui(f, &state, &theme))?;

        if let Event::Key(key) = event::read()? {
            // Handle quit
            if (key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL)) ||
               (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)) ||
               key.code == KeyCode::F(10) {
                break;
            }

            let is_edit_mode = state.mode == AppMode::Edit;
            let mut handled = false;

            // First priority: Try to dispatch through config system
            let mut ideal_cursor = state.ideal_cursor_column;

            if let Ok(Some(result)) = ActionDispatcher::dispatch_key(
                key.code,
                key.modifiers,
                &mut state,
                &mut ideal_cursor,
                is_edit_mode,
                false,
            ).await {
                state.ideal_cursor_column = ideal_cursor;
                state.debug_message = format!("Config handled: {:?}", key.code);

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
            }

            // Second priority: Handle character input in edit mode
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

            // Third priority: Fallback mode transitions
            if !handled {
                match (state.mode, key.code) {
                    (AppMode::ReadOnly, KeyCode::Char('i') | KeyCode::Char('a') | KeyCode::Insert) => {
                        state.enter_edit_mode();
                        if key.code == KeyCode::Char('a') {
                            state.cursor_pos = state.fields[state.current_field].len();
                        }
                        state.debug_message = format!("Entered edit mode via {:?}", key.code);
                        handled = true;
                    }
                    (AppMode::ReadOnly, KeyCode::Char('v')) => {
                        state.enter_highlight_mode();
                        state.debug_message = "Entered visual mode".to_string();
                        handled = true;
                    }
                    (_, KeyCode::Esc) => {
                        state.enter_readonly_mode();
                        state.debug_message = "Entered read-only mode".to_string();
                        handled = true;
                    }
                    _ => {}
                }
            }

            if !handled {
                state.debug_message = format!("Unhandled key: {:?}", key.code);
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
    .block(Block::default().borders(Borders::ALL).title("Status"));

    f.render_widget(status, chunks[1]);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = CanvasConfig::load();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let state = DemoFormState::new();

    let res = run_app(&mut terminal, state, config).await;

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
