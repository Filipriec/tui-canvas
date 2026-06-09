// examples/canvas-cursor-auto.rs
//! Demonstrates automatic cursor management with the canvas library
//!
//! This example REQUIRES the `cursor-style` feature to compile.
//!
//! Run with:
//! cargo run --example canvas_cursor_auto --features "gui,cursor-style"
//!
//! This will fail without cursor-style:
//! cargo run --example canvas-cursor-auto --features "gui"

// REQUIRE cursor-style feature - example won't compile without it
#[cfg(not(feature = "cursor-style"))]
compile_error!(
    "This example requires the 'cursor-style' feature. \
     Run with: cargo run --example canvas-cursor-auto --features \"gui,cursor-style\""
);

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
use std::io;

use tui_canvas::{
    render_canvas_default,
    AppMode,
    CursorManager, // This import only exists when cursor-style feature is enabled
    DataProvider, TextFormState,
};

// TextFormState wrapper for automatic cursor demo
struct AutoCursorTextForm<D: DataProvider> {
    editor: TextFormState<D>,
    has_unsaved_changes: bool,
    debug_message: String,
    command_buffer: String, // For multi-key vim commands like "gg"
}

impl<D: DataProvider> AutoCursorTextForm<D> {
    fn new(data_provider: D) -> Self {
        Self {
            editor: TextFormState::new(data_provider),
            has_unsaved_changes: false,
            debug_message: "🎯 Automatic Cursor Demo - cursor-style feature enabled!".to_string(),
            command_buffer: String::new(),
        }
    }

    // Command buffer handling

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

    // Visual highlight mode support

    fn enter_visual_mode(&mut self) {
        // Use the library method instead of manual state setting
        self.editor.enter_highlight_mode();
        self.debug_message = "🔥 VISUAL MODE - Cursor: Blinking Block █".to_string();
    }

    fn enter_visual_line_mode(&mut self) {
        // Use the library method instead of manual state setting
        self.editor.enter_highlight_line_mode();
        self.debug_message = "🔥 VISUAL LINE MODE - Cursor: Blinking Block █".to_string();
    }

    fn exit_visual_mode(&mut self) {
        // Use the library method
        self.editor.exit_highlight_mode();
        self.debug_message = "🔒 NORMAL MODE - Cursor: Steady Block █".to_string();
    }

    fn update_visual_selection(&mut self) {
        if self.editor.is_highlight_mode() {
            use tui_canvas::canvas::state::SelectionState;
            match self.editor.selection_state() {
                SelectionState::Characterwise { anchor } => {
                    self.debug_message = format!(
                        "🎯 Visual selection: anchor=({},{}) current=({},{}) - Cursor: Blinking Block █",
                        anchor.0, anchor.1,
                        self.editor.current_field(),
                        self.editor.cursor_position()
                    );
                }
                SelectionState::Linewise { anchor_field } => {
                    self.debug_message = format!(
                        "🎯 Visual LINE selection: anchor={} current={} - Cursor: Blinking Block █",
                        anchor_field,
                        self.editor.current_field()
                    );
                }
                _ => {}
            }
        }
    }

    // Movement with visual updates

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

    // Delete operations

    fn delete_backward(&mut self) -> anyhow::Result<()> {
        let result = self.editor.delete_backward();
        if result.is_ok() {
            self.has_unsaved_changes = true;
            self.debug_message = "⌫ Deleted character backward".to_string();
        }
        result
    }

    fn delete_forward(&mut self) -> anyhow::Result<()> {
        let result = self.editor.delete_forward();
        if result.is_ok() {
            self.has_unsaved_changes = true;
            self.debug_message = "⌦ Deleted character forward".to_string();
        }
        result
    }

    // Mode transitions with automatic cursor management

    fn enter_edit_mode(&mut self) {
        self.editor.enter_edit_mode(); // 🎯 Library automatically sets cursor to bar |
        self.debug_message = "✏️  INSERT MODE - Cursor: Steady Bar |".to_string();
    }

    fn enter_append_mode(&mut self) {
        self.editor.enter_append_mode(); // 🎯 Library automatically positions cursor and sets mode
        self.debug_message = "✏️  INSERT (append) - Cursor: Steady Bar |".to_string();
    }

    fn exit_edit_mode(&mut self) {
        self.editor.exit_edit_mode(); // 🎯 Library automatically sets cursor to block █
        self.exit_visual_mode();
        self.debug_message = "🔒 NORMAL MODE - Cursor: Steady Block █".to_string();
    }

    fn insert_char(&mut self, ch: char) -> anyhow::Result<()> {
        let result = self.editor.insert_char(ch);
        if result.is_ok() {
            self.has_unsaved_changes = true;
        }
        result
    }

    // Manual cursor override demonstration

    /// Demonstrate manual cursor control.
    fn demo_manual_cursor_control(&mut self) -> std::io::Result<()> {
        // Users can still manually control cursor if needed
        CursorManager::update_for_mode(AppMode::Command)?;
        self.debug_message = "🔧 Manual override: Command cursor _".to_string();
        Ok(())
    }

    fn restore_automatic_cursor(&mut self) -> std::io::Result<()> {
        // Restore automatic cursor based on current mode
        CursorManager::update_for_mode(self.editor.mode())?;
        self.debug_message = "🎯 Restored automatic cursor management".to_string();
        Ok(())
    }

    // Delegate to original editor

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
        let field_index = self.editor.current_field();
        self.editor.data_provider().field_value(field_index)
    }

    fn data_provider(&self) -> &D {
        self.editor.data_provider()
    }

    fn ui_state(&self) -> &tui_canvas::EditorState {
        self.editor.ui_state()
    }

    fn set_mode(&mut self, mode: AppMode) {
        self.editor.set_mode(mode); // 🎯 Library automatically updates cursor
        if mode != AppMode::Sel {
            self.exit_visual_mode();
        }
    }

    // Status and debug

    fn set_debug_message(&mut self, msg: String) {
        self.debug_message = msg;
    }

    fn debug_message(&self) -> &str {
        &self.debug_message
    }

    fn has_unsaved_changes(&self) -> bool {
        self.has_unsaved_changes
    }

    fn open_line_below(&mut self) -> anyhow::Result<()> {
        let result = self.editor.open_line_below();
        if result.is_ok() {
            self.debug_message = "✏️  INSERT (open line below) - Cursor: Steady Bar |".to_string();
        }
        result
    }

    fn open_line_above(&mut self) -> anyhow::Result<()> {
        let result = self.editor.open_line_above();
        if result.is_ok() {
            self.debug_message = "✏️  INSERT (open line above) - Cursor: Steady Bar |".to_string();
        }
        result
    }

    fn move_big_word_next(&mut self) {
        self.editor.move_big_word_next();
        self.update_visual_selection();
    }

    fn move_big_word_prev(&mut self) {
        self.editor.move_big_word_prev();
        self.update_visual_selection();
    }

    fn move_big_word_end(&mut self) {
        self.editor.move_big_word_end();
        self.update_visual_selection();
    }

    fn move_big_word_end_prev(&mut self) {
        self.editor.move_big_word_end_prev();
        self.update_visual_selection();
    }
}

// Demo form data with interesting text for cursor demonstration
struct CursorDemoData {
    fields: Vec<(String, String)>,
}

impl CursorDemoData {
    fn new() -> Self {
        Self {
            fields: vec![
                ("👤 Name".to_string(), "John-Paul McDonald".to_string()),
                (
                    "📧 Email".to_string(),
                    "user@example-domain.com".to_string(),
                ),
                ("📱 Phone".to_string(), "+1 (555) 123-4567".to_string()),
                ("🏠 Address".to_string(), "123 Main St, Apt 4B".to_string()),
                (
                    "🏷️  Tags".to_string(),
                    "urgent,important,follow-up".to_string(),
                ),
                (
                    "📝 Notes".to_string(),
                    "Watch the cursor change! Normal=█ Insert=| Visual=blinking█".to_string(),
                ),
                (
                    "🎯 Cursor Demo".to_string(),
                    "Press 'i' for insert, 'v' for visual, 'Esc' for normal".to_string(),
                ),
            ],
        }
    }
}

impl DataProvider for CursorDemoData {
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

/// Automatic cursor management demonstration
/// Features the CursorManager directly to show it's working
fn handle_key_press(
    key: KeyCode,
    modifiers: KeyModifiers,
    editor: &mut AutoCursorTextForm<CursorDemoData>,
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
        // Mode transitions with automatic cursor management
        (AppMode::Nor, KeyCode::Char('i'), _) => {
            editor.enter_edit_mode(); // 🎯 Automatic: cursor becomes bar |
            editor.clear_command_buffer();
        }
        (AppMode::Nor, KeyCode::Char('a'), _) => {
            editor.enter_append_mode();
            editor.set_debug_message("✏️  INSERT (append) - Cursor: Steady Bar |".to_string());
            editor.clear_command_buffer();
        }
        (AppMode::Nor, KeyCode::Char('A'), _) => {
            editor.move_line_end();
            editor.enter_edit_mode(); // 🎯 Automatic: cursor becomes bar |
            editor.set_debug_message("✏️  INSERT (end of line) - Cursor: Steady Bar |".to_string());
            editor.clear_command_buffer();
        }

        (AppMode::Nor, KeyCode::Char('o'), _) => {
            if let Err(e) = editor.open_line_below() {
                editor.set_debug_message(format!("Error opening line below: {e}"));
            }
            editor.clear_command_buffer();
        }
        (AppMode::Nor, KeyCode::Char('O'), _) => {
            if let Err(e) = editor.open_line_above() {
                editor.set_debug_message(format!("Error opening line above: {e}"));
            }
            editor.clear_command_buffer();
        }

        // From Normal Mode: Enter visual modes
        (AppMode::Nor, KeyCode::Char('v'), _) => {
            editor.enter_visual_mode();
            editor.clear_command_buffer();
        }
        (AppMode::Nor, KeyCode::Char('V'), _) => {
            editor.enter_visual_line_mode();
            editor.clear_command_buffer();
        }

        // From Visual Mode: Switch between visual modes or exit
        (AppMode::Sel, KeyCode::Char('v'), _) => {
            use tui_canvas::canvas::state::SelectionState;
            match editor.editor.selection_state() {
                SelectionState::Characterwise { .. } => {
                    // Already in characterwise mode, exit visual mode (vim behavior)
                    editor.exit_visual_mode();
                    editor.set_debug_message("🔒 Exited visual mode".to_string());
                }
                _ => {
                    // Switch from linewise to characterwise mode
                    editor.editor.enter_highlight_mode();
                    editor.update_visual_selection();
                    editor.set_debug_message("🔥 Switched to VISUAL mode".to_string());
                }
            }
            editor.clear_command_buffer();
        }

        (AppMode::Sel, KeyCode::Char('V'), _) => {
            use tui_canvas::canvas::state::SelectionState;
            match editor.editor.selection_state() {
                SelectionState::Linewise { .. } => {
                    // Already in linewise mode, exit visual mode (vim behavior)
                    editor.exit_visual_mode();
                    editor.set_debug_message("🔒 Exited visual mode".to_string());
                }
                _ => {
                    // Switch from characterwise to linewise mode
                    editor.editor.enter_highlight_line_mode();
                    editor.update_visual_selection();
                    editor.set_debug_message("🔥 Switched to VISUAL LINE mode".to_string());
                }
            }
            editor.clear_command_buffer();
        }

        // Escape: Exit any mode back to normal
        (_, KeyCode::Esc, _) => {
            match mode {
                AppMode::Ins => {
                    editor.exit_edit_mode(); // Exit insert mode
                }
                AppMode::Sel => {
                    editor.exit_visual_mode(); // Exit visual mode
                }
                _ => {
                    // Already in normal mode, just clear command buffer
                    editor.clear_command_buffer();
                }
            }
        }

        // Cursor management demonstration
        (AppMode::Nor, KeyCode::F(1), _) => {
            editor.demo_manual_cursor_control()?;
        }
        (AppMode::Nor, KeyCode::F(2), _) => {
            editor.restore_automatic_cursor()?;
        }

        // Movement vim style navigation

        // Basic movement (hjkl and arrows)
        (AppMode::Nor | AppMode::Sel, KeyCode::Char('h'), _)
        | (AppMode::Nor | AppMode::Sel, KeyCode::Left, _) => {
            editor.move_left();
            editor.set_debug_message("← left".to_string());
            editor.clear_command_buffer();
        }
        (AppMode::Nor | AppMode::Sel, KeyCode::Char('l'), _)
        | (AppMode::Nor | AppMode::Sel, KeyCode::Right, _) => {
            editor.move_right();
            editor.set_debug_message("→ right".to_string());
            editor.clear_command_buffer();
        }
        (AppMode::Nor | AppMode::Sel, KeyCode::Char('j'), _)
        | (AppMode::Nor | AppMode::Sel, KeyCode::Down, _) => {
            editor.move_down();
            editor.set_debug_message("↓ next field".to_string());
            editor.clear_command_buffer();
        }
        (AppMode::Nor | AppMode::Sel, KeyCode::Char('k'), _)
        | (AppMode::Nor | AppMode::Sel, KeyCode::Up, _) => {
            editor.move_up();
            editor.set_debug_message("↑ previous field".to_string());
            editor.clear_command_buffer();
        }

        // Word movement
        (AppMode::Nor | AppMode::Sel, KeyCode::Char('w'), _) => {
            editor.move_word_next();
            editor.set_debug_message("w: next word start".to_string());
            editor.clear_command_buffer();
        }
        (AppMode::Nor | AppMode::Sel, KeyCode::Char('b'), _) => {
            editor.move_word_prev();
            editor.set_debug_message("b: previous word start".to_string());
            editor.clear_command_buffer();
        }
        (AppMode::Nor | AppMode::Sel, KeyCode::Char('e'), _) => {
            // Check if this is 'ge' command
            if editor.get_command_buffer() == "g" {
                editor.move_word_end_prev();
                editor.set_debug_message("ge: previous word end".to_string());
                editor.clear_command_buffer();
            } else {
                editor.move_word_end();
                editor.set_debug_message("e: word end".to_string());
                editor.clear_command_buffer();
            }
        }

        (AppMode::Nor | AppMode::Sel, KeyCode::Char('W'), _) => {
            editor.move_big_word_next();
            editor.set_debug_message("W: next WORD start".to_string());
            editor.clear_command_buffer();
        }
        (AppMode::Nor | AppMode::Sel, KeyCode::Char('B'), _) => {
            editor.move_big_word_prev();
            editor.set_debug_message("B: previous WORD start".to_string());
            editor.clear_command_buffer();
        }
        (AppMode::Nor | AppMode::Sel, KeyCode::Char('E'), _) => {
            // Check if this is 'gE' command
            if editor.get_command_buffer() == "g" {
                editor.move_big_word_end_prev();
                editor.set_debug_message("gE: previous WORD end".to_string());
                editor.clear_command_buffer();
            } else {
                editor.move_big_word_end();
                editor.set_debug_message("E: WORD end".to_string());
                editor.clear_command_buffer();
            }
        }

        // Line movement
        (AppMode::Nor | AppMode::Sel, KeyCode::Char('0'), _)
        | (AppMode::Nor | AppMode::Sel, KeyCode::Home, _) => {
            editor.move_line_start();
            editor.set_debug_message("0: line start".to_string());
        }
        (AppMode::Nor | AppMode::Sel, KeyCode::Char('$'), _)
        | (AppMode::Nor | AppMode::Sel, KeyCode::End, _) => {
            editor.move_line_end();
            editor.set_debug_message("$: line end".to_string());
        }

        // Field/document movement
        (AppMode::Nor | AppMode::Sel, KeyCode::Char('g'), _) => {
            if editor.get_command_buffer() == "g" {
                editor.move_first_line();
                editor.set_debug_message("gg: first field".to_string());
                editor.clear_command_buffer();
            } else {
                editor.clear_command_buffer();
                editor.add_to_command_buffer('g');
                editor.set_debug_message("g".to_string());
            }
        }
        (AppMode::Nor | AppMode::Sel, KeyCode::Char('G'), _) => {
            editor.move_last_line();
            editor.set_debug_message("G: last field".to_string());
            editor.clear_command_buffer();
        }

        // Edit mode movement
        (AppMode::Ins, KeyCode::Left, m) if m.contains(KeyModifiers::CONTROL) => {
            editor.move_word_prev();
            editor.set_debug_message("Ctrl+← word back".to_string());
        }
        (AppMode::Ins, KeyCode::Right, m) if m.contains(KeyModifiers::CONTROL) => {
            editor.move_word_next();
            editor.set_debug_message("Ctrl+→ word forward".to_string());
        }
        (AppMode::Ins, KeyCode::Left, _) => {
            editor.move_left();
        }
        (AppMode::Ins, KeyCode::Right, _) => {
            editor.move_right();
        }
        (AppMode::Ins, KeyCode::Up, _) => {
            editor.move_up();
        }
        (AppMode::Ins, KeyCode::Down, _) => {
            editor.move_down();
        }
        (AppMode::Ins, KeyCode::Home, _) => {
            editor.move_line_start();
        }
        (AppMode::Ins, KeyCode::End, _) => {
            editor.move_line_end();
        }

        // Delete operations
        (AppMode::Ins, KeyCode::Backspace, _) => {
            editor.delete_backward()?;
        }
        (AppMode::Ins, KeyCode::Delete, _) => {
            editor.delete_forward()?;
        }

        // Delete operations in normal mode (vim x)
        (AppMode::Nor, KeyCode::Char('x'), _) => {
            editor.delete_forward()?;
            editor.set_debug_message("x: deleted character".to_string());
        }
        (AppMode::Nor, KeyCode::Char('X'), _) => {
            editor.delete_backward()?;
            editor.set_debug_message("X: deleted character backward".to_string());
        }

        // Tab navigation
        (_, KeyCode::Tab, _) => {
            editor.next_field();
            editor.set_debug_message("Tab: next field".to_string());
        }
        (_, KeyCode::BackTab, _) => {
            editor.prev_field();
            editor.set_debug_message("Shift+Tab: previous field".to_string());
        }

        // Character input
        (AppMode::Ins, KeyCode::Char(c), m) if !m.contains(KeyModifiers::CONTROL) => {
            editor.insert_char(c)?;
        }

        // Debug info commands
        (AppMode::Nor, KeyCode::Char('?'), _) => {
            editor.set_debug_message(format!(
                "Field {}/{}, Pos {}, Mode: {:?} - Cursor managed automatically!",
                editor.current_field() + 1,
                editor.data_provider().field_count(),
                editor.cursor_position(),
                editor.mode()
            ));
        }

        _ => {
            if editor.has_pending_command() {
                editor.clear_command_buffer();
                editor.set_debug_message("Invalid command sequence".to_string());
            } else {
                editor.set_debug_message(format!(
                    "Unhandled: {key:?} + {modifiers:?} in {mode:?} mode"
                ));
            }
        }
    }

    Ok(true)
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut editor: AutoCursorTextForm<CursorDemoData>,
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
                    editor.set_debug_message(format!("Error: {e}"));
                }
            }
        }
    }

    Ok(())
}

fn ui(f: &mut Frame, editor: &AutoCursorTextForm<CursorDemoData>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(10)])
        .split(f.area());

    render_enhanced_canvas(f, chunks[0], editor);
    render_status_and_help(f, chunks[1], editor);
}

fn render_enhanced_canvas(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    editor: &AutoCursorTextForm<CursorDemoData>,
) {
    render_canvas_default(f, area, &editor.editor);
}

fn render_status_and_help(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    editor: &AutoCursorTextForm<CursorDemoData>,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(7)])
        .split(area);

    let mode_text = match editor.mode() {
        AppMode::Ins => "INSERT | (bar cursor)",
        AppMode::Nor => "NORMAL █ (block cursor)",
        AppMode::Sel => {
            // Use library selection state instead of editor.highlight_state()
            use tui_canvas::canvas::state::SelectionState;
            match editor.editor.selection_state() {
                SelectionState::Characterwise { .. } => "VISUAL █ (blinking block)",
                SelectionState::Linewise { .. } => "VISUAL LINE █ (blinking block)",
                _ => "VISUAL █ (blinking block)",
            }
        }
        _ => "NORMAL █ (block cursor)",
    };

    let status_text = if editor.has_pending_command() {
        format!(
            "-- {} -- {} [{}]",
            mode_text,
            editor.debug_message(),
            editor.get_command_buffer()
        )
    } else if editor.has_unsaved_changes() {
        format!("-- {} -- [Modified] {}", mode_text, editor.debug_message())
    } else {
        format!("-- {} -- {}", mode_text, editor.debug_message())
    };

    let status = Paragraph::new(Line::from(Span::raw(status_text))).block(
        Block::default()
            .borders(Borders::ALL)
            .title("🎯 Automatic Cursor Status"),
    );

    f.render_widget(status, chunks[0]);

    let help_text = match editor.mode() {
        AppMode::Nor => {
            if editor.has_pending_command() {
                match editor.get_command_buffer() {
                    "g" => "Press 'g' again for first field, or any other key to cancel",
                    _ => "Pending command... (Esc to cancel)",
                }
            } else {
                "🎯 CURSOR-STYLE DEMO: Normal █ | Insert | | Visual blinking█\n\
                Normal: hjkl/arrows=move, w/b/e=words, W/B/E=WORDS, 0/$=line, gg/G=first/last\n\
                i/a/A/o/O=insert, v/V=visual, x/X=delete, ?=info\n\
                F1=demo manual cursor, F2=restore automatic"
            }
        }
        AppMode::Ins => {
            "🎯 INSERT MODE - Cursor: | (bar)\n\
             arrows=move, Ctrl+arrows=words, Backspace/Del=delete\n\
             Esc=normal, Tab/Shift+Tab=fields"
        }
        AppMode::Sel => {
            "🎯 VISUAL MODE - Cursor: █ (blinking block)\n\
             hjkl/arrows=extend selection, w/b/e=word selection\n\
             Esc=normal"
        }
        _ => "🎯 Watch the cursor change automatically!",
    };

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("🚀 Automatic Cursor Management"),
        )
        .style(Style::default().fg(Color::Gray));

    f.render_widget(help, chunks[1]);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Print feature status
    println!("🎯 Canvas Cursor Auto Demo");
    println!("✅ cursor-style feature: ENABLED");
    println!("🚀 Automatic cursor management: ACTIVE");
    println!("📖 Watch your terminal cursor change based on mode!");
    println!();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let data = CursorDemoData::new();
    let mut editor = AutoCursorTextForm::new(data);

    // Initialize with normal mode - library automatically sets block cursor
    editor.set_mode(AppMode::Nor);

    CursorManager::update_for_mode(AppMode::Nor)?;

    let res = run_app(&mut terminal, editor);

    CursorManager::reset()?;

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

    println!("🎯 Cursor automatically reset to default!");
    Ok(())
}
