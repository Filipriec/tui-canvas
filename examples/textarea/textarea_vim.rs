// examples/textarea_vim.rs
//! Demonstrates automatic cursor management with the textarea widget
//!
//! This example REQUIRES the `cursor-style` and `textarea` features to compile.
//!
//! Run with:
//! cargo run --example canvas_textarea_cursor_auto --features "gui,cursor-style,textarea"

// REQUIRE cursor-style and textarea features
#[cfg(not(feature = "cursor-style"))]
compile_error!(
    "This example requires the 'cursor-style' feature. \
     Run with: cargo run --example canvas_textarea_cursor_auto --features \"gui,cursor-style,textarea\""
);

#[cfg(not(feature = "textarea"))]
compile_error!(
    "This example requires the 'textarea' feature. \
     Run with: cargo run --example canvas_textarea_cursor_auto --features \"gui,cursor-style,textarea\""
);

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    },
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
use std::time::Duration;

use canvas::{
    render_canvas_default, AppMode, CursorManager, TextArea, TextAreaState,
};

/// TextArea demo with automatic cursor management.
struct AutoCursorTextArea {
    textarea: TextAreaState,
    has_unsaved_changes: bool,
    debug_message: String,
    command_buffer: String,
}

impl AutoCursorTextArea {
    fn new() -> Self {
        let initial_text = "Automatic Cursor Management Demo\n\
Welcome!\n\
\n\
\n\
Try different modes:\n\
• Normal mode: Block cursor █\n\
• Insert mode: Bar cursor |\n\
\n\
\n\
Navigation commands:\n\
• hjkl or arrow keys: move cursor\n\
• i/a/A/o/O: enter insert mode\n\
• w/b/e/W/B/E: word movements\n\
• Esc: return to normal mode\n\
\n\
Terminal cursor changes automatically!
\n\
\n\
\n\
";

        let mut textarea = TextAreaState::from_text(initial_text);
        textarea.set_placeholder("Start typing...");
        textarea.use_wrap();

        Self {
            textarea,
            has_unsaved_changes: false,
            debug_message: "Automatic Cursor - cursor-style feature enabled".to_string(),
            command_buffer: String::new(),
        }
    }

    // Mode transitions with automatic cursor management

    fn enter_insert_mode(&mut self) -> std::io::Result<()> {
        self.textarea.enter_edit_mode(); // Direct FormEditor method call via Deref!
        CursorManager::update_for_mode(AppMode::Ins)?; // Automatic: cursor becomes bar |
        self.debug_message = "INSERT MODE - Cursor: |".to_string();
        Ok(())
    }

    fn enter_append_mode(&mut self) -> std::io::Result<()> {
        self.textarea.enter_append_mode(); // Direct FormEditor method call!
        CursorManager::update_for_mode(AppMode::Ins)?;
        self.debug_message = "INSERT (append) - Cursor: |".to_string();
        Ok(())
    }

    fn exit_to_normal_mode(&mut self) -> std::io::Result<()> {
        self.textarea.exit_edit_mode(); // Direct FormEditor method call!
        CursorManager::update_for_mode(AppMode::Nor)?; // Automatic: cursor becomes steady block
        self.debug_message = "NORMAL MODE - Cursor: █".to_string();
        Ok(())
    }

    // Manual cursor override demonstration

    fn demo_manual_cursor_control(&mut self) -> std::io::Result<()> {
        // Users can still manually control cursor if needed
        CursorManager::update_for_mode(AppMode::Command)?;
        self.debug_message = "Manual override: Command cursor _".to_string();
        Ok(())
    }

    fn restore_automatic_cursor(&mut self) -> std::io::Result<()> {
        // Restore automatic cursor based on current mode
        CursorManager::update_for_mode(self.textarea.mode())?;
        self.debug_message = "Restored automatic cursor management".to_string();
        Ok(())
    }

    // Textarea operations

    fn handle_textarea_input(&mut self, key: KeyEvent) {
        self.textarea.input(key);
        self.has_unsaved_changes = true;
    }

    // Movement operations using direct FormEditor methods

    fn move_left(&mut self) {
        self.textarea.move_left();
        self.update_debug_for_movement("← left");
    }

    fn move_right(&mut self) {
        self.textarea.move_right();
        self.update_debug_for_movement("→ right");
    }

    fn move_up(&mut self) {
        self.textarea.move_up();
        self.update_debug_for_movement("↑ up");
    }

    fn move_down(&mut self) {
        self.textarea.move_down();
        self.update_debug_for_movement("↓ down");
    }

    fn move_word_next(&mut self) {
        self.textarea.move_word_next();
        self.update_debug_for_movement("w: next word");
    }

    fn move_word_prev(&mut self) {
        self.textarea.move_word_prev();
        self.update_debug_for_movement("b: previous word");
    }

    fn move_word_end(&mut self) {
        self.textarea.move_word_end();
        self.update_debug_for_movement("e: word end");
    }

    fn move_word_end_prev(&mut self) {
        self.textarea.move_word_end_prev();
        self.update_debug_for_movement("ge: previous word end");
    }

    fn move_line_start(&mut self) {
        self.textarea.move_line_start();
        self.update_debug_for_movement("0: line start");
    }

    fn move_line_end(&mut self) {
        self.textarea.move_line_end();
        self.update_debug_for_movement("$: line end");
    }

    fn move_first_line(&mut self) {
        self.textarea.move_first_line();
        self.update_debug_for_movement("gg: first line");
    }

    fn move_last_line(&mut self) {
        self.textarea.move_last_line();
        self.update_debug_for_movement("G: last line");
    }

    // Big word movements

    fn move_big_word_next(&mut self) {
        self.textarea.move_big_word_next();
        self.update_debug_for_movement("W: next WORD");
    }

    fn move_big_word_prev(&mut self) {
        self.textarea.move_big_word_prev();
        self.update_debug_for_movement("B: previous WORD");
    }

    fn move_big_word_end(&mut self) {
        self.textarea.move_big_word_end();
        self.update_debug_for_movement("E: WORD end");
    }

    fn move_big_word_end_prev(&mut self) {
        self.textarea.move_big_word_end_prev();
        self.update_debug_for_movement("gE: previous WORD end");
    }

    fn update_debug_for_movement(&mut self, action: &str) {
        self.debug_message = action.to_string();
    }

    // Delete operations

    fn delete_char_forward(&mut self) {
        if let Ok(_) = self.textarea.delete_forward() {
            self.has_unsaved_changes = true;
            self.debug_message = "x: deleted character".to_string();
        }
    }

    fn delete_char_backward(&mut self) {
        if let Ok(_) = self.textarea.delete_backward() {
            self.has_unsaved_changes = true;
            self.debug_message = "X: deleted character backward".to_string();
        }
    }

    // Vim style editing

    fn open_line_below(&mut self) {
        self.textarea.open_line_below();
        CursorManager::update_for_mode(AppMode::Ins).ok();
        self.debug_message = "INSERT - Cursor: |".to_string();
        self.has_unsaved_changes = true;
    }

    fn open_line_above(&mut self) {
        self.textarea.open_line_above();
        CursorManager::update_for_mode(AppMode::Ins).ok();
        self.debug_message = "INSERT - Cursor: |".to_string();
        self.has_unsaved_changes = true;
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

    // Getters

    fn mode(&self) -> AppMode {
        self.textarea.mode()
    }

    fn debug_message(&self) -> &str {
        &self.debug_message
    }

    fn has_unsaved_changes(&self) -> bool {
        self.has_unsaved_changes
    }

    fn set_debug_message(&mut self, msg: String) {
        self.debug_message = msg;
    }

    fn get_cursor_info(&self) -> String {
        format!(
            "Line {}, Col {}",
            self.textarea.current_field() + 1,
            self.textarea.cursor_position() + 1
        )
    }
}

/// Handle key press with automatic cursor management
fn handle_key_press(key_event: KeyEvent, editor: &mut AutoCursorTextArea) -> anyhow::Result<bool> {
    let KeyEvent {
        code: key,
        modifiers,
        ..
    } = key_event;
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
            editor.enter_insert_mode()?;
            editor.clear_command_buffer();
        }
        (AppMode::Nor, KeyCode::Char('a'), _) => {
            editor.enter_append_mode()?;
            editor.clear_command_buffer();
        }
        (AppMode::Nor, KeyCode::Char('A'), _) => {
            editor.move_line_end();
            editor.enter_insert_mode()?;
            editor.clear_command_buffer();
        }

        // Vim o/O commands
        (AppMode::Nor, KeyCode::Char('o'), _) => {
            editor.open_line_below();
            editor.clear_command_buffer();
        }
        (AppMode::Nor, KeyCode::Char('O'), _) => {
            editor.open_line_above();
            editor.clear_command_buffer();
        }

        // Escape: Exit any mode back to normal
        (AppMode::Ins, KeyCode::Esc, _) => {
            editor.exit_to_normal_mode()?;
        }

        // Insert mode pass to textarea
        (AppMode::Ins, _, _) => {
            editor.handle_textarea_input(key_event);
        }

        // Cursor management demonstration
        (AppMode::Nor, KeyCode::F(1), _) => {
            editor.demo_manual_cursor_control()?;
        }
        (AppMode::Nor, KeyCode::F(2), _) => {
            editor.restore_automatic_cursor()?;
        }

        // Movement vim style navigation normal mode
        (AppMode::Nor, KeyCode::Char('h'), _) | (AppMode::Nor, KeyCode::Left, _) => {
            editor.move_left();
            editor.clear_command_buffer();
        }
        (AppMode::Nor, KeyCode::Char('l'), _) | (AppMode::Nor, KeyCode::Right, _) => {
            editor.move_right();
            editor.clear_command_buffer();
        }
        (AppMode::Nor, KeyCode::Char('j'), _) | (AppMode::Nor, KeyCode::Down, _) => {
            editor.move_down();
            editor.clear_command_buffer();
        }
        (AppMode::Nor, KeyCode::Char('k'), _) | (AppMode::Nor, KeyCode::Up, _) => {
            editor.move_up();
            editor.clear_command_buffer();
        }

        // Word movement
        (AppMode::Nor, KeyCode::Char('w'), _) => {
            editor.move_word_next();
            editor.clear_command_buffer();
        }
        (AppMode::Nor, KeyCode::Char('b'), _) => {
            editor.move_word_prev();
            editor.clear_command_buffer();
        }
        (AppMode::Nor, KeyCode::Char('e'), _) => {
            if editor.get_command_buffer() == "g" {
                editor.move_word_end_prev();
                editor.clear_command_buffer();
            } else {
                editor.move_word_end();
                editor.clear_command_buffer();
            }
        }

        // Big word movement (vim W/B/E commands)
        (AppMode::Nor, KeyCode::Char('W'), _) => {
            editor.move_big_word_next();
            editor.clear_command_buffer();
        }
        (AppMode::Nor, KeyCode::Char('B'), _) => {
            editor.move_big_word_prev();
            editor.clear_command_buffer();
        }
        (AppMode::Nor, KeyCode::Char('E'), _) => {
            if editor.get_command_buffer() == "g" {
                editor.move_big_word_end_prev();
                editor.clear_command_buffer();
            } else {
                editor.move_big_word_end();
                editor.clear_command_buffer();
            }
        }

        // Line movement
        (AppMode::Nor, KeyCode::Char('0'), _) | (AppMode::Nor, KeyCode::Home, _) => {
            editor.move_line_start();
            editor.clear_command_buffer();
        }
        (AppMode::Nor, KeyCode::Char('$'), _) | (AppMode::Nor, KeyCode::End, _) => {
            editor.move_line_end();
            editor.clear_command_buffer();
        }

        // Document movement with command buffer
        (AppMode::Nor, KeyCode::Char('g'), _) => {
            if editor.get_command_buffer() == "g" {
                editor.move_first_line();
                editor.clear_command_buffer();
            } else {
                editor.clear_command_buffer();
                editor.add_to_command_buffer('g');
                editor.set_debug_message("g".to_string());
            }
        }
        (AppMode::Nor, KeyCode::Char('G'), _) => {
            editor.move_last_line();
            editor.clear_command_buffer();
        }

        // Delete operations normal mode
        (AppMode::Nor, KeyCode::Char('x'), _) => {
            editor.delete_char_forward();
            editor.clear_command_buffer();
        }
        (AppMode::Nor, KeyCode::Char('X'), _) => {
            editor.delete_char_backward();
            editor.clear_command_buffer();
        }

        // Debug info commands
        (AppMode::Nor, KeyCode::Char('?'), _) => {
            editor.set_debug_message(format!(
                "{}, Mode: {:?} - Cursor managed automatically",
                editor.get_cursor_info(),
                mode
            ));
            editor.clear_command_buffer();
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
    mut editor: AutoCursorTextArea,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut editor))?;

        if crossterm::event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                match handle_key_press(key, &mut editor) {
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
    }

    Ok(())
}

fn ui(f: &mut Frame, editor: &mut AutoCursorTextArea) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(8)])
        .split(f.area());

    render_textarea(f, chunks[0], editor);
    render_status_and_help(f, chunks[1], editor);
}

fn render_textarea(f: &mut Frame, area: ratatui::layout::Rect, editor: &mut AutoCursorTextArea) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Automatic Cursor");

    let textarea_widget = TextArea::default().block(block.clone());

    f.render_stateful_widget(textarea_widget, area, &mut editor.textarea);

    // Set cursor position for terminal cursor
    // Always show cursor - CursorManager handles the style (block/bar/blinking)
    let (cx, cy) = editor.textarea.cursor(area, Some(&block));
    f.set_cursor_position((cx, cy));
}

fn render_status_and_help(f: &mut Frame, area: ratatui::layout::Rect, editor: &AutoCursorTextArea) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(5)])
        .split(area);

    // Status bar with cursor information
    let mode_text = match editor.mode() {
        AppMode::Ins => "INSERT | (bar)",
        AppMode::Nor => "NORMAL █ (block)",
        AppMode::Sel => "VISUAL █ (blinking block)",
        _ => "NORMAL █ (block)",
    };

    let status_text = if editor.has_pending_command() {
        format!(
            "-- {} -- {} [{}]",
            mode_text,
            editor.debug_message(),
            editor.get_command_buffer()
        )
    } else if editor.has_unsaved_changes() {
        format!(
            "-- {} -- [Modified] {} | {}",
            mode_text,
            editor.debug_message(),
            editor.get_cursor_info()
        )
    } else {
        format!(
            "-- {} -- {} | {}",
            mode_text,
            editor.debug_message(),
            editor.get_cursor_info()
        )
    };

    let status = Paragraph::new(Line::from(Span::raw(status_text))).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Automatic Cursor Status"),
    );

    f.render_widget(status, chunks[0]);

    let help_text = match editor.mode() {
        AppMode::Nor => {
            if editor.has_pending_command() {
                match editor.get_command_buffer() {
                    "g" => "Press 'g' again for first line, or any other key to cancel",
                    _ => "Pending command... (Esc to cancel)",
                }
            } else {
                "CURSOR-STYLE DEMO: Normal █ | Insert | \n\
                Normal: hjkl/arrows=move, w/b/e=words, W/B/E=WORDS, 0/$=line, g/G=first/last\n\
                i/a/A/o/O=insert, x/X=delete, ?=info\n\
                F1=demo manual cursor, F2=restore automatic, Ctrl+Q=quit"
            }
        }
        AppMode::Ins => {
            "INSERT MODE - Cursor: | (bar)\n\
            Type to edit text, arrows=move, Enter=new line\n\
            Esc=normal mode"
        }
        AppMode::Sel => {
            "VISUAL MODE - Cursor: █ (blinking block)\n\
            hjkl/arrows=extend selection\n\
            Esc=normal mode"
        }
        _ => "Watch the cursor change automatically!",
    };

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Automatic Cursor"),
        )
        .style(Style::default().fg(Color::Gray));

    f.render_widget(help, chunks[1]);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Print feature status
    println!("Canvas Textarea Cursor Auto Demo");
    println!("cursor-style feature: ENABLED");
    println!("textarea feature: ENABLED");
    println!("Automatic cursor management: ACTIVE");
    println!("Watch your terminal cursor change based on mode!");
    println!();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut editor = AutoCursorTextArea::new();

    // Initialize with normal mode - library automatically sets block cursor
    editor.exit_to_normal_mode()?;

    let res = run_app(&mut terminal, editor);

    // Reset cursor on exit
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

    println!("Cursor automatically reset to default!");
    Ok(())
}
