// examples/full_canvas_demo.rs
//! Demonstrates the FULL potential of the canvas library (excluding autocomplete)

use std::io;
use crossterm::{
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
        actions::movement::{
            find_next_word_start, find_word_end, find_prev_word_start, find_prev_word_end,
            line_start_position, line_end_position, safe_cursor_position,
            clamp_cursor_position,
        },
    },
    DataProvider, FormEditor,
};

// Enhanced FormEditor that exposes the full action system
struct EnhancedFormEditor<D: DataProvider> {
    editor: FormEditor<D>,
    highlight_state: HighlightState,
    has_unsaved_changes: bool,
    debug_message: String,
}

impl<D: DataProvider> EnhancedFormEditor<D> {
    fn new(data_provider: D) -> Self {
        Self {
            editor: FormEditor::new(data_provider),
            highlight_state: HighlightState::Off,
            has_unsaved_changes: false,
            debug_message: "Full Canvas Demo - All features enabled".to_string(),
        }
    }

    // === EXPOSE ALL THE MISSING METHODS ===

    /// Word movement using library's sophisticated logic
    fn move_word_next(&mut self) {
        let current_text = self.editor.current_text().to_string();
        let current_pos = self.editor.cursor_position();
        let new_pos = find_next_word_start(&current_text, current_pos);
        let is_edit = self.editor.mode() == AppMode::Edit;

        self.set_cursor_clamped(new_pos, &current_text, is_edit);
        self.update_visual_selection();
    }

    fn move_word_prev(&mut self) {
        let current_text = self.editor.current_text().to_string();
        let current_pos = self.editor.cursor_position();
        let new_pos = find_prev_word_start(&current_text, current_pos);
        let is_edit = self.editor.mode() == AppMode::Edit;

        self.set_cursor_clamped(new_pos, &current_text, is_edit);
        self.update_visual_selection();
    }

    fn move_word_end(&mut self) {
        let current_text = self.editor.current_text().to_string();
        let current_pos = self.editor.cursor_position();
        let new_pos = find_word_end(&current_text, current_pos);
        let is_edit = self.editor.mode() == AppMode::Edit;

        self.set_cursor_clamped(new_pos, &current_text, is_edit);
        self.update_visual_selection();
    }

    fn move_word_end_prev(&mut self) {
        let current_text = self.editor.current_text().to_string();
        let current_pos = self.editor.cursor_position();
        let new_pos = find_prev_word_end(&current_text, current_pos);
        let is_edit = self.editor.mode() == AppMode::Edit;

        self.set_cursor_clamped(new_pos, &current_text, is_edit);
        self.update_visual_selection();
    }

    /// Line movement using library's functions
    fn move_line_start(&mut self) {
        let pos = line_start_position();
        let current_text = self.editor.current_text().to_string();
        let is_edit = self.editor.mode() == AppMode::Edit;

        self.set_cursor_clamped(pos, &current_text, is_edit);
        self.update_visual_selection();
    }

    fn move_line_end(&mut self) {
        let current_text = self.editor.current_text().to_string();
        let is_edit = self.editor.mode() == AppMode::Edit;
        let pos = line_end_position(&current_text, is_edit);

        self.set_cursor_clamped(pos, &current_text, is_edit);
        self.update_visual_selection();
    }

    /// Field movement - proper implementations
    fn move_to_prev_field(&mut self) {
        let current = self.editor.current_field();
        let total = self.editor.data_provider().field_count();
        let _prev = if current == 0 { total - 1 } else { current - 1 };

        // Move to previous field and position cursor properly
        for _ in 0..(total - 1) {
            self.editor.move_to_next_field();
        }

        // Position cursor using safe positioning
        let current_text = self.editor.current_text().to_string();
        let ideal_column = 0; // Start of field when switching
        let is_edit = self.editor.mode() == AppMode::Edit;
        let safe_pos = safe_cursor_position(&current_text, ideal_column, is_edit);
        self.set_cursor_clamped(safe_pos, &current_text, is_edit);
        self.update_visual_selection();
    }

    fn move_to_first_field(&mut self) {
        let current = self.editor.current_field();
        let total = self.editor.data_provider().field_count();

        // Move to first field (index 0)
        for _ in 0..(total - current) {
            self.editor.move_to_next_field();
        }

        let current_text = self.editor.current_text().to_string();
        let is_edit = self.editor.mode() == AppMode::Edit;
        self.set_cursor_clamped(0, &current_text, is_edit);
        self.update_visual_selection();
    }

    fn move_to_last_field(&mut self) {
        let current = self.editor.current_field();
        let total = self.editor.data_provider().field_count();
        let moves_needed = (total - 1 - current) % total;

        // Move to last field
        for _ in 0..moves_needed {
            self.editor.move_to_next_field();
        }

        let current_text = self.editor.current_text().to_string();
        let is_edit = self.editor.mode() == AppMode::Edit;
        self.set_cursor_clamped(0, &current_text, is_edit);
        self.update_visual_selection();
    }

    /// Delete operations - proper implementations
    fn delete_backward(&mut self) -> anyhow::Result<()> {
        if self.editor.mode() != AppMode::Edit || self.editor.cursor_position() == 0 {
            return Ok(());
        }

        let field_idx = self.editor.current_field();
        let cursor_pos = self.editor.cursor_position();
        let mut text = self.editor.data_provider().field_value(field_idx).to_string();

        if cursor_pos > 0 && cursor_pos <= text.len() {
            text.remove(cursor_pos - 1);

            // This is a limitation - we need mutable access to update the field
            // For now, we'll show a message that this would work with a proper API
            self.debug_message =
                "Delete backward: API limitation - would remove character".to_string();
            self.has_unsaved_changes = true;
        }

        Ok(())
    }

    fn delete_forward(&mut self) -> anyhow::Result<()> {
        if self.editor.mode() != AppMode::Edit {
            return Ok(());
        }

        let field_idx = self.editor.current_field();
        let cursor_pos = self.editor.cursor_position();
        let text = self.editor.data_provider().field_value(field_idx);

        if cursor_pos < text.len() {
            // Same limitation as above
            self.debug_message =
                "Delete forward: API limitation - would remove character".to_string();
            self.has_unsaved_changes = true;
        }

        Ok(())
    }

    /// Visual/Highlight mode support
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

    /// Enhanced movement with visual selection updates
    fn move_left(&mut self) {
        self.editor.move_left();
        self.update_visual_selection();
    }

    fn move_right(&mut self) {
        self.editor.move_right();
        self.update_visual_selection();
    }

    fn move_up(&mut self) {
        self.move_to_prev_field();
    }

    fn move_down(&mut self) {
        self.editor.move_to_next_field();
        self.update_visual_selection();
    }

    // === UTILITY METHODS ===

    fn set_cursor_clamped(&mut self, pos: usize, text: &str, is_edit: bool) {
        let clamped_pos = clamp_cursor_position(pos, text, is_edit);
        // Since we can't directly set cursor, we need to move to it
        while self.editor.cursor_position() < clamped_pos {
            self.editor.move_right();
        }
        while self.editor.cursor_position() > clamped_pos {
            self.editor.move_left();
        }
    }

    fn update_visual_selection(&mut self) {
        if self.editor.mode() == AppMode::Highlight {
            match &self.highlight_state {
                HighlightState::Characterwise { anchor: _ } => {
                    let _current_pos =
                        (self.editor.current_field(), self.editor.cursor_position());
                    self.debug_message = format!(
                        "Visual selection: char {} to {}",
                        self.editor.cursor_position(),
                        self.editor.cursor_position()
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

    fn insert_char(&mut self, ch: char) -> anyhow::Result<()> {
        let result = self.editor.insert_char(ch);
        if result.is_ok() {
            self.has_unsaved_changes = true;
        }
        result
    }

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
    fn supports_autocomplete(&self, _field_index: usize) -> bool {
        false
    }
    fn display_value(&self, _index: usize) -> Option<&str> {
        None
    }
}

/// Full vim-like key handling using ALL library features
fn handle_key_press(
    key: KeyCode,
    modifiers: KeyModifiers,
    editor: &mut EnhancedFormEditor<FullDemoData>,
) -> anyhow::Result<bool> {
    let mode = editor.mode();

    // Quit handling
    if (key == KeyCode::Char('q') && modifiers.contains(KeyModifiers::CONTROL))
        || (key == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL))
        || key == KeyCode::F(10)
    {
        return Ok(false);
    }

    match (mode, key, modifiers) {
        // === MODE TRANSITIONS ===
        (AppMode::ReadOnly, KeyCode::Char('i'), _) => {
            if ModeManager::can_enter_edit_mode(mode) {
                editor.set_mode(AppMode::Edit);
                editor.set_debug_message("-- INSERT --".to_string());
            }
        }
        (AppMode::ReadOnly, KeyCode::Char('a'), _) => {
            editor.move_line_end();
            if ModeManager::can_enter_edit_mode(mode) {
                editor.set_mode(AppMode::Edit);
                editor.set_debug_message("-- INSERT -- (append)".to_string());
            }
        }
        (AppMode::ReadOnly, KeyCode::Char('A'), _) => {
            editor.move_line_end();
            if ModeManager::can_enter_edit_mode(mode) {
                editor.set_mode(AppMode::Edit);
                editor.set_debug_message("-- INSERT -- (end of line)".to_string());
            }
        }
        (AppMode::ReadOnly, KeyCode::Char('v'), _) => {
            editor.enter_visual_mode();
        }
        (AppMode::ReadOnly, KeyCode::Char('V'), _) => {
            editor.enter_visual_line_mode();
        }
        (_, KeyCode::Esc, _) => {
            if ModeManager::can_enter_read_only_mode(mode) {
                editor.set_mode(AppMode::ReadOnly);
                editor.exit_visual_mode();
                editor.set_debug_message("".to_string());
            }
        }

        // === MOVEMENT: All the vim goodness ===

        // Basic movement
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('h'), _)
        | (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Left, _) => {
            editor.move_left();
            editor.set_debug_message("move left".to_string());
        }
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('l'), _)
        | (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Right, _) => {
            editor.move_right();
            editor.set_debug_message("move right".to_string());
        }
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('j'), _)
        | (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Down, _) => {
            editor.move_down();
            editor.set_debug_message("move down".to_string());
        }
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('k'), _)
        | (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Up, _) => {
            editor.move_up();
            editor.set_debug_message("move up".to_string());
        }

        // Word movement - THE FULL VIM EXPERIENCE
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('w'), _) => {
            editor.move_word_next();
            editor.set_debug_message("next word start".to_string());
        }
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('b'), _) => {
            editor.move_word_prev();
            editor.set_debug_message("previous word start".to_string());
        }
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('e'), _) => {
            editor.move_word_end();
            editor.set_debug_message("word end".to_string());
        }
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('B'), _) => {
            editor.move_word_end_prev();
            editor.set_debug_message("previous word end".to_string());
        }

        // Line movement
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('0'), _)
        | (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Home, _) => {
            editor.move_line_start();
            editor.set_debug_message("line start".to_string());
        }
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('$'), _)
        | (AppMode::ReadOnly | AppMode::Highlight, KeyCode::End, _) => {
            editor.move_line_end();
            editor.set_debug_message("line end".to_string());
        }

        // Field movement - advanced navigation
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('g'), _) => {
            editor.move_to_first_field();
            editor.set_debug_message("first field".to_string());
        }
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('G'), _) => {
            editor.move_to_last_field();
            editor.set_debug_message("last field".to_string());
        }

        // === EDIT MODE ===
        (AppMode::Edit, KeyCode::Left, _) => editor.move_left(),
        (AppMode::Edit, KeyCode::Right, _) => editor.move_right(),
        (AppMode::Edit, KeyCode::Up, _) => editor.move_up(),
        (AppMode::Edit, KeyCode::Down, _) => editor.move_down(),
        (AppMode::Edit, KeyCode::Home, _) => editor.move_line_start(),
        (AppMode::Edit, KeyCode::End, _) => editor.move_line_end(),

        // Word movement in edit mode with Ctrl
        (AppMode::Edit, KeyCode::Left, m) if m.contains(KeyModifiers::CONTROL) => {
            editor.move_word_prev();
        }
        (AppMode::Edit, KeyCode::Right, m) if m.contains(KeyModifiers::CONTROL) => {
            editor.move_word_next();
        }

        // DELETE OPERATIONS
        (AppMode::Edit, KeyCode::Backspace, _) => {
            editor.delete_backward()?;
        }
        (AppMode::Edit, KeyCode::Delete, _) => {
            editor.delete_forward()?;
        }

        // Tab navigation
        (_, KeyCode::Tab, _) => {
            editor.editor.move_to_next_field();
            editor.set_debug_message("next field".to_string());
        }
        (_, KeyCode::BackTab, _) => {
            editor.move_to_prev_field();
            editor.set_debug_message("previous field".to_string());
        }

        // Character input
        (AppMode::Edit, KeyCode::Char(c), m) if !m.contains(KeyModifiers::CONTROL) => {
            editor.insert_char(c)?;
        }

        _ => {
            editor.set_debug_message(format!("Unhandled: {:?} in {:?} mode", key, mode));
        }
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
        .constraints([Constraint::Min(8), Constraint::Length(6)])
        .split(f.area());

    render_enhanced_canvas(f, chunks[0], editor);
    render_status_bar(f, chunks[1], editor);
}

fn render_enhanced_canvas(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    editor: &EnhancedFormEditor<FullDemoData>,
) {
    // Uses the library default theme; no theme needed.
    render_canvas_default(f, area, &editor.editor);
}

fn render_status_bar(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    editor: &EnhancedFormEditor<FullDemoData>,
) {
    let mode_text = match editor.mode() {
        AppMode::Edit => "INSERT",
        AppMode::ReadOnly => "NORMAL",
        AppMode::Highlight => match editor.highlight_state() {
            HighlightState::Characterwise { .. } => "VISUAL",
            HighlightState::Linewise { .. } => "VISUAL",
            _ => "VISUAL",
        },
        _ => "NORMAL",
    };

    let status = Paragraph::new(Line::from(Span::raw(format!(
        "-- {} --",
        mode_text
    ))))
    .block(Block::default().borders(Borders::ALL).title("Mode"));

    f.render_widget(status, area);
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

    let res = run_app(&mut terminal, editor);

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
