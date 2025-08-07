// examples/full_canvas_demo.rs
//! Demonstrates the FULL potential of the canvas library using the native API

use std::io;
use crossterm::{
    cursor::SetCursorStyle,
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers,
    },
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    },
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
        gui::render_canvas_default,
        modes::{AppMode, ModeManager, HighlightState},
    },
    DataProvider, FormEditor,
};

/// Update cursor style based on current AppMode
fn update_cursor_for_mode(mode: AppMode) -> io::Result<()> {
    let style = match mode {
        AppMode::Edit => SetCursorStyle::SteadyBar,           // Thin line for insert mode
        AppMode::ReadOnly => SetCursorStyle::SteadyBlock,     // Block for normal mode  
        AppMode::Highlight => SetCursorStyle::BlinkingBlock,  // Blinking block for visual mode
        AppMode::General => SetCursorStyle::SteadyBlock,      // Block for general mode
        AppMode::Command => SetCursorStyle::SteadyUnderScore, // Underscore for command mode
    };
    
    execute!(io::stdout(), style)
}

// Enhanced FormEditor that adds visual mode and status tracking
struct EnhancedFormEditor<D: DataProvider> {
    editor: FormEditor<D>,
    highlight_state: HighlightState,
    has_unsaved_changes: bool,
    debug_message: String,
    command_buffer: String, // For multi-key vim commands like "gg"
}

impl<D: DataProvider> EnhancedFormEditor<D> {
    fn new(data_provider: D) -> Self {
        Self {
            editor: FormEditor::new(data_provider),
            highlight_state: HighlightState::Off,
            has_unsaved_changes: false,
            debug_message: "Full Canvas Demo - All features enabled".to_string(),
            command_buffer: String::new(),
        }
    }

    // === COMMAND BUFFER HANDLING ===

    fn clear_command_buffer(&mut self) {
        self.command_buffer.clear();
    }

    fn add_to_command_buffer(&mut self, ch: char) {
        self.command_buffer.push(ch);
    }

    fn get_command_buffer(&self) -> &str {
        &self.command_buffer
    }

    fn has_pending_command(&self) -> bool {
        !self.command_buffer.is_empty()
    }

    // === VISUAL/HIGHLIGHT MODE SUPPORT ===

    fn enter_visual_mode(&mut self) {
        if ModeManager::can_enter_highlight_mode(self.editor.mode()) {
            self.editor.set_mode(AppMode::Highlight);
            self.highlight_state = HighlightState::Characterwise {
                anchor: (
                    self.editor.current_field(),
                    self.editor.cursor_position(),
                ),
            };
            self.debug_message = "-- VISUAL --".to_string();
        }
    }

    fn enter_visual_line_mode(&mut self) {
        if ModeManager::can_enter_highlight_mode(self.editor.mode()) {
            self.editor.set_mode(AppMode::Highlight);
            self.highlight_state =
                HighlightState::Linewise { anchor_line: self.editor.current_field() };
            self.debug_message = "-- VISUAL LINE --".to_string();
        }
    }

    fn exit_visual_mode(&mut self) {
        self.highlight_state = HighlightState::Off;
        if self.editor.mode() == AppMode::Highlight {
            self.editor.set_mode(AppMode::ReadOnly);
            self.debug_message = "Visual mode exited".to_string();
        }
    }

    fn update_visual_selection(&mut self) {
        if self.editor.mode() == AppMode::Highlight {
            match &self.highlight_state {
                HighlightState::Characterwise { anchor: _ } => {
                    self.debug_message = format!(
                        "Visual selection: char {} in field {}",
                        self.editor.cursor_position(),
                        self.editor.current_field()
                    );
                }
                HighlightState::Linewise { anchor_line: _ } => {
                    self.debug_message = format!(
                        "Visual line selection: field {}",
                        self.editor.current_field()
                    );
                }
                _ => {}
            }
        }
    }

    // === ENHANCED MOVEMENT WITH VISUAL UPDATES ===

    fn move_left(&mut self) {
        self.editor.move_left();
        self.update_visual_selection();
    }

    fn move_right(&mut self) {
        self.editor.move_right();
        self.update_visual_selection();
    }

    fn move_up(&mut self) {
        self.editor.move_up();
        self.update_visual_selection();
    }

    fn move_down(&mut self) {
        self.editor.move_down();
        self.update_visual_selection();
    }

    fn move_word_next(&mut self) {
        self.editor.move_word_next();
        self.update_visual_selection();
    }

    fn move_word_prev(&mut self) {
        self.editor.move_word_prev();
        self.update_visual_selection();
    }

    fn move_word_end(&mut self) {
        self.editor.move_word_end();
        self.update_visual_selection();
    }

    fn move_word_end_prev(&mut self) {
        self.editor.move_word_end_prev();
        self.update_visual_selection();
    }

    fn move_line_start(&mut self) {
        self.editor.move_line_start();
        self.update_visual_selection();
    }

    fn move_line_end(&mut self) {
        self.editor.move_line_end();
        self.update_visual_selection();
    }

    fn move_first_line(&mut self) {
        self.editor.move_first_line();
        self.update_visual_selection();
    }

    fn move_last_line(&mut self) {
        self.editor.move_last_line();
        self.update_visual_selection();
    }

    fn prev_field(&mut self) {
        self.editor.prev_field();
        self.update_visual_selection();
    }

    fn next_field(&mut self) {
        self.editor.next_field();
        self.update_visual_selection();
    }

    // === DELETE OPERATIONS ===

    fn delete_backward(&mut self) -> anyhow::Result<()> {
        let result = self.editor.delete_backward();
        if result.is_ok() {
            self.has_unsaved_changes = true;
            self.debug_message = "Deleted character backward".to_string();
        }
        Ok(result?)
    }

    fn delete_forward(&mut self) -> anyhow::Result<()> {
        let result = self.editor.delete_forward();
        if result.is_ok() {
            self.has_unsaved_changes = true;
            self.debug_message = "Deleted character forward".to_string();
        }
        Ok(result?)
    }

    // === MODE TRANSITIONS ===

    fn enter_edit_mode(&mut self) {
        self.editor.enter_edit_mode();
        self.debug_message = "-- INSERT --".to_string();
    }

    fn exit_edit_mode(&mut self) {
        self.editor.exit_edit_mode();
        self.exit_visual_mode();
        self.debug_message = "".to_string();
    }

    fn insert_char(&mut self, ch: char) -> anyhow::Result<()> {
        let result = self.editor.insert_char(ch);
        if result.is_ok() {
            self.has_unsaved_changes = true;
        }
        Ok(result?)
    }

    // === DELEGATE TO ORIGINAL EDITOR ===

    fn current_field(&self) -> usize {
        self.editor.current_field()
    }

    fn cursor_position(&self) -> usize {
        self.editor.cursor_position()
    }

    fn mode(&self) -> AppMode {
        self.editor.mode()
    }

    fn current_text(&self) -> &str {
        self.editor.current_text()
    }

    fn data_provider(&self) -> &D {
        self.editor.data_provider()
    }

    fn ui_state(&self) -> &canvas::EditorState {
        self.editor.ui_state()
    }

    fn set_mode(&mut self, mode: AppMode) {
        self.editor.set_mode(mode);
        if mode != AppMode::Highlight {
            self.exit_visual_mode();
        }
    }

    // === STATUS AND DEBUG ===

    fn set_debug_message(&mut self, msg: String) {
        self.debug_message = msg;
    }

    fn debug_message(&self) -> &str {
        &self.debug_message
    }

    fn highlight_state(&self) -> &HighlightState {
        &self.highlight_state
    }

    fn has_unsaved_changes(&self) -> bool {
        self.has_unsaved_changes
    }
}

// Demo form data with interesting text for word movement
struct FullDemoData {
    fields: Vec<(String, String)>,
}

impl FullDemoData {
    fn new() -> Self {
        Self {
            fields: vec![
                ("Name".to_string(), "John-Paul McDonald".to_string()),
                (
                    "Email".to_string(),
                    "user@example-domain.com".to_string(),
                ),
                ("Phone".to_string(), "+1 (555) 123-4567".to_string()),
                ("Address".to_string(), "123 Main St, Apt 4B".to_string()),
                (
                    "Tags".to_string(),
                    "urgent,important,follow-up".to_string(),
                ),
                (
                    "Notes".to_string(),
                    "This is a sample note with multiple words, punctuation! And symbols @#$"
                        .to_string(),
                ),
            ],
        }
    }
}

impl DataProvider for FullDemoData {
    fn field_count(&self) -> usize {
        self.fields.len()
    }

    fn field_name(&self, index: usize) -> &str {
        &self.fields[index].0
    }

    fn field_value(&self, index: usize) -> &str {
        &self.fields[index].1
    }

    fn set_field_value(&mut self, index: usize, value: String) {
        self.fields[index].1 = value;
    }

    fn supports_suggestions(&self, _field_index: usize) -> bool {
        false
    }

    fn display_value(&self, _index: usize) -> Option<&str> {
        None
    }
}

/// Full vim-like key handling using the native FormEditor API
fn handle_key_press(
    key: KeyCode,
    modifiers: KeyModifiers,
    editor: &mut EnhancedFormEditor<FullDemoData>,
) -> anyhow::Result<bool> {
    let old_mode = editor.mode(); // Store mode before processing

    // Quit handling
    if (key == KeyCode::Char('q') && modifiers.contains(KeyModifiers::CONTROL))
        || (key == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL))
        || key == KeyCode::F(10)
    {
        return Ok(false);
    }

    match (old_mode, key, modifiers) {
        // === MODE TRANSITIONS ===
        (AppMode::ReadOnly, KeyCode::Char('i'), _) => {
            editor.enter_edit_mode();
            editor.clear_command_buffer();
        }
        (AppMode::ReadOnly, KeyCode::Char('a'), _) => {
            editor.move_right(); // Move after current character
            editor.enter_edit_mode();
            editor.set_debug_message("-- INSERT -- (append)".to_string());
            editor.clear_command_buffer();
        }
        (AppMode::ReadOnly, KeyCode::Char('A'), _) => {
            editor.move_line_end();
            editor.enter_edit_mode();
            editor.set_debug_message("-- INSERT -- (end of line)".to_string());
            editor.clear_command_buffer();
        }
        (AppMode::ReadOnly, KeyCode::Char('o'), _) => {
            editor.move_line_end();
            editor.enter_edit_mode();
            editor.set_debug_message("-- INSERT -- (open line)".to_string());
            editor.clear_command_buffer();
        }
        (AppMode::ReadOnly, KeyCode::Char('v'), _) => {
            editor.enter_visual_mode();
            editor.clear_command_buffer();
        }
        (AppMode::ReadOnly, KeyCode::Char('V'), _) => {
            editor.enter_visual_line_mode();
            editor.clear_command_buffer();
        }
        (_, KeyCode::Esc, _) => {
            editor.exit_edit_mode();
            editor.clear_command_buffer();
        }

        // === MOVEMENT: VIM-STYLE NAVIGATION ===

        // Basic movement (hjkl and arrows)
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('h'), _)
        | (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Left, _) => {
            editor.move_left();
            editor.set_debug_message("← left".to_string());
            editor.clear_command_buffer();
        }
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('l'), _)
        | (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Right, _) => {
            editor.move_right();
            editor.set_debug_message("→ right".to_string());
            editor.clear_command_buffer();
        }
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('j'), _)
        | (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Down, _) => {
            editor.move_down();
            editor.set_debug_message("↓ next field".to_string());
            editor.clear_command_buffer();
        }
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('k'), _)
        | (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Up, _) => {
            editor.move_up();
            editor.set_debug_message("↑ previous field".to_string());
            editor.clear_command_buffer();
        }

        // Word movement - Full vim word navigation
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('w'), _) => {
            editor.move_word_next();
            editor.set_debug_message("w: next word start".to_string());
            editor.clear_command_buffer();
        }
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('b'), _) => {
            editor.move_word_prev();
            editor.set_debug_message("b: previous word start".to_string());
            editor.clear_command_buffer();
        }
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('e'), _) => {
            editor.move_word_end();
            editor.set_debug_message("e: word end".to_string());
            editor.clear_command_buffer();
        }
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('W'), _) => {
            editor.move_word_end_prev();
            editor.set_debug_message("W: previous word end".to_string());
            editor.clear_command_buffer();
        }

        // Line movement
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('0'), _)
        | (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Home, _) => {
            editor.move_line_start();
            editor.set_debug_message("0: line start".to_string());
        }
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('$'), _)
        | (AppMode::ReadOnly | AppMode::Highlight, KeyCode::End, _) => {
            editor.move_line_end();
            editor.set_debug_message("$: line end".to_string());
        }

        // Field/document movement
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('g'), _) => {
            if editor.get_command_buffer() == "g" {
                // Second 'g' - execute "gg" command
                editor.move_first_line();
                editor.set_debug_message("gg: first field".to_string());
                editor.clear_command_buffer();
            } else {
                // First 'g' - start command buffer
                editor.clear_command_buffer();
                editor.add_to_command_buffer('g');
                editor.set_debug_message("g".to_string());
            }
        }
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('G'), _) => {
            editor.move_last_line();
            editor.set_debug_message("G: last field".to_string());
            editor.clear_command_buffer();
        }

        // === EDIT MODE MOVEMENT ===
        (AppMode::Edit, KeyCode::Left, m) if m.contains(KeyModifiers::CONTROL) => {
            editor.move_word_prev();
            editor.set_debug_message("Ctrl+← word back".to_string());
        }
        (AppMode::Edit, KeyCode::Right, m) if m.contains(KeyModifiers::CONTROL) => {
            editor.move_word_next();
            editor.set_debug_message("Ctrl+→ word forward".to_string());
        }
        (AppMode::Edit, KeyCode::Left, _) => {
            editor.move_left();
        }
        (AppMode::Edit, KeyCode::Right, _) => {
            editor.move_right();
        }
        (AppMode::Edit, KeyCode::Up, _) => {
            editor.move_up();
        }
        (AppMode::Edit, KeyCode::Down, _) => {
            editor.move_down();
        }
        (AppMode::Edit, KeyCode::Home, _) => {
            editor.move_line_start();
        }
        (AppMode::Edit, KeyCode::End, _) => {
            editor.move_line_end();
        }

        // === DELETE OPERATIONS ===
        (AppMode::Edit, KeyCode::Backspace, _) => {
            editor.delete_backward()?;
        }
        (AppMode::Edit, KeyCode::Delete, _) => {
            editor.delete_forward()?;
        }

        // Delete operations in normal mode (vim x)
        (AppMode::ReadOnly, KeyCode::Char('x'), _) => {
            editor.delete_forward()?;
            editor.set_debug_message("x: deleted character".to_string());
        }
        (AppMode::ReadOnly, KeyCode::Char('X'), _) => {
            editor.delete_backward()?;
            editor.set_debug_message("X: deleted character backward".to_string());
        }

        // === TAB NAVIGATION ===
        (_, KeyCode::Tab, _) => {
            editor.next_field();
            editor.set_debug_message("Tab: next field".to_string());
        }
        (_, KeyCode::BackTab, _) => {
            editor.prev_field();
            editor.set_debug_message("Shift+Tab: previous field".to_string());
        }

        // === CHARACTER INPUT ===
        (AppMode::Edit, KeyCode::Char(c), m) if !m.contains(KeyModifiers::CONTROL) => {
            editor.insert_char(c)?;
        }

        // === DEBUG/INFO COMMANDS ===
        (AppMode::ReadOnly, KeyCode::Char('?'), _) => {
            editor.set_debug_message(format!(
                "Field {}/{}, Pos {}, Mode: {:?}",
                editor.current_field() + 1,
                editor.data_provider().field_count(),
                editor.cursor_position(),
                editor.mode()
            ));
        }

        _ => {
            // If we have a pending command and this key doesn't complete it, clear the buffer
            if editor.has_pending_command() {
                editor.clear_command_buffer();
                editor.set_debug_message("Invalid command sequence".to_string());
            } else {
                editor.set_debug_message(format!(
                    "Unhandled: {:?} + {:?} in {:?} mode",
                    key, modifiers, old_mode
                ));
            }
        }
    }

    // Update cursor if mode changed
    let new_mode = editor.mode();
    if old_mode != new_mode {
        update_cursor_for_mode(new_mode)?;
    }

    Ok(true)
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut editor: EnhancedFormEditor<FullDemoData>,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &editor))?;

        if let Event::Key(key) = event::read()? {
            match handle_key_press(key.code, key.modifiers, &mut editor) {
                Ok(should_continue) => {
                    if !should_continue {
                        break;
                    }
                }
                Err(e) => {
                    editor.set_debug_message(format!("Error: {}", e));
                }
            }
        }
    }

    Ok(())
}

fn ui(f: &mut Frame, editor: &EnhancedFormEditor<FullDemoData>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(8)])
        .split(f.area());

    render_enhanced_canvas(f, chunks[0], editor);
    render_status_and_help(f, chunks[1], editor);
}

fn render_enhanced_canvas(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    editor: &EnhancedFormEditor<FullDemoData>,
) {
    render_canvas_default(f, area, &editor.editor);
}

fn render_status_and_help(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    editor: &EnhancedFormEditor<FullDemoData>,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(5)])
        .split(area);

    // Status bar
    let mode_text = match editor.mode() {
        AppMode::Edit => "INSERT",
        AppMode::ReadOnly => "NORMAL",
        AppMode::Highlight => match editor.highlight_state() {
            HighlightState::Characterwise { .. } => "VISUAL",
            HighlightState::Linewise { .. } => "VISUAL LINE",
            _ => "VISUAL",
        },
        _ => "NORMAL",
    };

    let status_text = if editor.has_pending_command() {
        format!("-- {} -- {} [{}]", mode_text, editor.debug_message(), editor.get_command_buffer())
    } else if editor.has_unsaved_changes() {
        format!("-- {} -- [Modified] {}", mode_text, editor.debug_message())
    } else {
        format!("-- {} -- {}", mode_text, editor.debug_message())
    };

    let status = Paragraph::new(Line::from(Span::raw(status_text)))
        .block(Block::default().borders(Borders::ALL).title("Status"));

    f.render_widget(status, chunks[0]);

    // Help text
    let help_text = match editor.mode() {
        AppMode::ReadOnly => {
            if editor.has_pending_command() {
                match editor.get_command_buffer() {
                    "g" => "Press 'g' again for first field, or any other key to cancel",
                    _ => "Pending command... (Esc to cancel)"
                }
            } else {
                "Normal: hjkl/arrows=move, w/b/e=words, 0/$=line, gg/G=first/last, i/a/A=insert, v/V=visual, x/X=delete, ?=info"
            }
        }
        AppMode::Edit => {
            "Insert: arrows=move, Ctrl+arrows=words, Backspace/Del=delete, Esc=normal, Tab/Shift+Tab=fields"
        }
        AppMode::Highlight => {
            "Visual: hjkl/arrows=extend selection, w/b/e=word selection, Esc=normal"
        }
        _ => "Press ? for help"
    };

    let help = Paragraph::new(Line::from(Span::raw(help_text)))
        .block(Block::default().borders(Borders::ALL).title("Commands"))
        .style(Style::default().fg(Color::Gray));

    f.render_widget(help, chunks[1]);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let data = FullDemoData::new();
    let mut editor = EnhancedFormEditor::new(data);
    editor.set_mode(AppMode::ReadOnly); // Start in normal mode
    
    // Set initial cursor style
    update_cursor_for_mode(editor.mode())?;

    let res = run_app(&mut terminal, editor);

    // Reset cursor style on exit
    execute!(io::stdout(), SetCursorStyle::DefaultUserShape)?;

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
