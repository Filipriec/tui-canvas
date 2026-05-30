// examples/canvas_keymap.rs
//! Demonstrates the centralized keymap system for canvas interactions
//!
//! This example shows how to use the canvas-keymap feature to delegate
//! all canvas key handling to the library, supporting complex sequences
//! like "gg", "ge", etc.
//!
//! Run with:
//! cargo run --example canvas_keymap --features "gui,keymap,cursor-style"

#[cfg(not(feature = "keymap"))]
compile_error!(
    "This example requires the 'keymap' feature. \
     Run with: cargo run --example canvas_keymap --features \"gui,keymap,cursor-style\""
);

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEvent},
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
use std::collections::HashMap;
use std::io;

use canvas::{
    canvas::{gui::render_canvas_default, modes::AppMode},
    keymap::{CanvasKeyMap, KeyEventOutcome},
    DataProvider, FormEditor,
};

/// Demo application using centralized keymap system
struct KeymapDemoApp {
    editor: FormEditor<DemoData>,
    message: String,
    quit: bool,
}

impl KeymapDemoApp {
    fn new() -> Self {
        let data = DemoData::new();
        let mut editor = FormEditor::new(data);

        // Build and inject the keymap from our config
        let keymap = Self::build_demo_keymap();
        editor.set_keymap(keymap);

        Self {
            editor,
            message: "🎯 Keymap system loaded! Try: gg, ge, hjkl, w/b/e, v, i, etc.".to_string(),
            quit: false,
        }
    }

    /// Build a keymap configuration
    fn build_demo_keymap() -> CanvasKeyMap {
        let mut read_only = HashMap::new();
        let mut edit = HashMap::new();
        let mut highlight = HashMap::new();

        // Read only mode keybindings

        // Basic movement
        read_only.insert(
            "move_left".to_string(),
            vec!["h".to_string(), "Left".to_string()],
        );
        read_only.insert(
            "move_right".to_string(),
            vec!["l".to_string(), "Right".to_string()],
        );
        read_only.insert(
            "move_up".to_string(),
            vec!["k".to_string(), "Up".to_string()],
        );
        read_only.insert(
            "move_down".to_string(),
            vec!["j".to_string(), "Down".to_string()],
        );

        // Word movement
        read_only.insert("move_word_next".to_string(), vec!["w".to_string()]);
        read_only.insert("move_word_prev".to_string(), vec!["b".to_string()]);
        read_only.insert("move_word_end".to_string(), vec!["e".to_string()]);
        read_only.insert("move_word_end_prev".to_string(), vec!["ge".to_string()]); // Multi-key!

        // Big word movement
        read_only.insert("move_big_word_next".to_string(), vec!["W".to_string()]);
        read_only.insert("move_big_word_prev".to_string(), vec!["B".to_string()]);
        read_only.insert("move_big_word_end".to_string(), vec!["E".to_string()]);
        read_only.insert("move_big_word_end_prev".to_string(), vec!["gE".to_string()]); // Multi-key!

        // Line movement
        read_only.insert(
            "move_line_start".to_string(),
            vec!["0".to_string(), "Home".to_string()],
        );
        read_only.insert(
            "move_line_end".to_string(),
            vec!["$".to_string(), "End".to_string()],
        );

        // Field movement
        read_only.insert("move_first_line".to_string(), vec!["gg".to_string()]); // Multi-key!
        read_only.insert("move_last_line".to_string(), vec!["G".to_string()]);
        read_only.insert("next_field".to_string(), vec!["Tab".to_string()]);
        read_only.insert("prev_field".to_string(), vec!["Shift+Tab".to_string()]);

        // Mode transitions
        read_only.insert("enter_edit_mode_before".to_string(), vec!["i".to_string()]);
        read_only.insert("enter_edit_mode_after".to_string(), vec!["a".to_string()]);
        read_only.insert("enter_highlight_mode".to_string(), vec!["v".to_string()]);
        read_only.insert(
            "enter_highlight_mode_linewise".to_string(),
            vec!["V".to_string()],
        );

        // Editing actions in normal mode
        read_only.insert("delete_char_forward".to_string(), vec!["x".to_string()]);
        read_only.insert("delete_char_backward".to_string(), vec!["X".to_string()]);
        read_only.insert("open_line_below".to_string(), vec!["o".to_string()]);
        read_only.insert("open_line_above".to_string(), vec!["O".to_string()]);

        // Edit mode keybindings

        edit.insert("exit_edit_mode".to_string(), vec!["esc".to_string()]);
        edit.insert("move_left".to_string(), vec!["Left".to_string()]);
        edit.insert("move_right".to_string(), vec!["Right".to_string()]);
        edit.insert("move_up".to_string(), vec!["Up".to_string()]);
        edit.insert("move_down".to_string(), vec!["Down".to_string()]);
        edit.insert("move_line_start".to_string(), vec!["Home".to_string()]);
        edit.insert("move_line_end".to_string(), vec!["End".to_string()]);
        edit.insert("move_word_next".to_string(), vec!["Ctrl+Right".to_string()]);
        edit.insert("move_word_prev".to_string(), vec!["Ctrl+Left".to_string()]);
        edit.insert("next_field".to_string(), vec!["Tab".to_string()]);
        edit.insert("prev_field".to_string(), vec!["Shift+Tab".to_string()]);
        edit.insert(
            "delete_char_backward".to_string(),
            vec!["Backspace".to_string()],
        );
        edit.insert(
            "delete_char_forward".to_string(),
            vec!["Delete".to_string()],
        );

        // Highlight mode keybindings

        highlight.insert("exit_highlight_mode".to_string(), vec!["esc".to_string()]);
        highlight.insert(
            "enter_highlight_mode_linewise".to_string(),
            vec!["V".to_string()],
        );

        // Movement (extends selection)
        highlight.insert(
            "move_left".to_string(),
            vec!["h".to_string(), "Left".to_string()],
        );
        highlight.insert(
            "move_right".to_string(),
            vec!["l".to_string(), "Right".to_string()],
        );
        highlight.insert(
            "move_up".to_string(),
            vec!["k".to_string(), "Up".to_string()],
        );
        highlight.insert(
            "move_down".to_string(),
            vec!["j".to_string(), "Down".to_string()],
        );
        highlight.insert("move_word_next".to_string(), vec!["w".to_string()]);
        highlight.insert("move_word_prev".to_string(), vec!["b".to_string()]);
        highlight.insert("move_word_end".to_string(), vec!["e".to_string()]);
        highlight.insert("move_word_end_prev".to_string(), vec!["ge".to_string()]);
        highlight.insert("move_line_start".to_string(), vec!["0".to_string()]);
        highlight.insert("move_line_end".to_string(), vec!["$".to_string()]);
        highlight.insert("move_first_line".to_string(), vec!["gg".to_string()]);
        highlight.insert("move_last_line".to_string(), vec!["G".to_string()]);

        CanvasKeyMap::from_mode_maps(&read_only, &edit, &highlight)
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> io::Result<()> {
        // First, try canvas keymap
        match self.editor.handle_key_event(key_event) {
            KeyEventOutcome::Consumed(Some(msg)) => {
                self.message = format!("🎯 Canvas: {}", msg);
                return Ok(());
            }
            KeyEventOutcome::Consumed(None) => {
                self.message = "🎯 Canvas action executed".to_string();
                return Ok(());
            }
            KeyEventOutcome::Pending => {
                self.message = "⏳ Waiting for next key in sequence...".to_string();
                return Ok(());
            }
            KeyEventOutcome::NotMatched => {
                // Fall through to client actions
            }
            KeyEventOutcome::ExitTop | KeyEventOutcome::ExitBottom => {
                // Canvas reached a navigation boundary; a host with its own
                // focus manager would move focus here. Fall through.
            }
        }

        // Handle client-specific actions (non-canvas)
        use crossterm::event::{KeyCode, KeyModifiers};
        match (key_event.code, key_event.modifiers) {
            (KeyCode::Char('q'), KeyModifiers::CONTROL)
            | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                self.quit = true;
                self.message = "👋 Goodbye!".to_string();
            }
            (KeyCode::F(1), _) => {
                self.message =
                    "ℹ️  F1: This is a client action (not handled by canvas keymap)".to_string();
            }
            (KeyCode::F(2), _) => {
                // Demonstrate saving
                self.message = "💾 F2: Save action (client-side)".to_string();
            }
            (KeyCode::Char('?'), _) if self.editor.mode() == AppMode::ReadOnly => {
                self.show_help();
            }
            _ => {
                // Unknown key
                self.message = format!(
                    "❓ Unhandled key: {:?} (mode: {:?})",
                    key_event.code,
                    self.editor.mode()
                );
            }
        }

        Ok(())
    }

    fn show_help(&mut self) {
        self.message =
            "📖 Help: Multi-key sequences work! Try gg, ge, gE. Also: hjkl, w/b/e, v/V, i/a/o"
                .to_string();
    }

    fn should_quit(&self) -> bool {
        self.quit
    }

    fn editor(&self) -> &FormEditor<DemoData> {
        &self.editor
    }

    fn message(&self) -> &str {
        &self.message
    }
}

/// Demo form data with interesting examples for keymap testing
struct DemoData {
    fields: Vec<(String, String)>,
}

impl DemoData {
    fn new() -> Self {
        Self {
            fields: vec![
                (
                    "🎯 Name".to_string(),
                    "John-Paul McDonald-Smith".to_string(),
                ),
                (
                    "📧 Email".to_string(),
                    "user@long-domain-name.example.com".to_string(),
                ),
                (
                    "📱 Phone".to_string(),
                    "+1 (555) 123-4567 ext. 890".to_string(),
                ),
                (
                    "🏠 Address".to_string(),
                    "123 Main Street, Apartment 4B, Suite 100".to_string(),
                ),
                (
                    "🏷️  Tags".to_string(),
                    "urgent,important,follow-up,high-priority".to_string(),
                ),
                (
                    "📝 Notes".to_string(),
                    "Test word movements: w=next-word, b=prev-word, e=word-end, ge=prev-word-end"
                        .to_string(),
                ),
                (
                    "🔥 Multi-key".to_string(),
                    "Try multi-key sequences: gg=first-field, ge=prev-word-end, gE=prev-WORD-end"
                        .to_string(),
                ),
                (
                    "⚡ Vim Actions".to_string(),
                    "Normal mode: x=delete-char, o=open-line-below, v=visual, i=insert".to_string(),
                ),
            ],
        }
    }
}

impl DataProvider for DemoData {
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
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: KeymapDemoApp) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &app))?;

        if let Event::Key(key) = event::read()? {
            app.handle_key_event(key)?;
            if app.should_quit() {
                break;
            }
        }
    }
    Ok(())
}

fn ui(f: &mut Frame, app: &KeymapDemoApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(12)])
        .split(f.area());

    // Render the canvas
    render_canvas_default(f, chunks[0], app.editor());

    // Render status and help
    render_status_and_help(f, chunks[1], app);
}

fn render_status_and_help(f: &mut Frame, area: ratatui::layout::Rect, app: &KeymapDemoApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(9)])
        .split(area);

    // Status message
    let status_text = format!(
        "Mode: {:?} | Field: {}/{} | Pos: {} | {}",
        app.editor().mode(),
        app.editor().current_field() + 1,
        app.editor().data_provider().field_count(),
        app.editor().cursor_position(),
        app.message()
    );

    let status = Paragraph::new(Line::from(Span::raw(status_text))).block(
        Block::default()
            .borders(Borders::ALL)
            .title("🎯 Keymap Demo Status"),
    );

    f.render_widget(status, chunks[0]);

    // Help text based on current mode
    let help_text = match app.editor().mode() {
        AppMode::ReadOnly => {
            "🎯 KEYMAP DEMO - All keys handled by centralized keymap system!\n\
             \n\
             📍 MOVEMENT: hjkl(basic) | w/b/e(words) | W/B/E(WORDS) | 0/$(line) | gg/G(fields)\n\
             🔥 MULTI-KEY: gg=first-field, ge=prev-word-end, gE=prev-WORD-end\n\
             ✏️  MODES: i/a(insert) | v/V(visual) | o/O(open-line)\n\
             🗑️  DELETE: x/X(delete-char)\n\
             📂 FIELDS: Tab/Shift+Tab\n\
             \n\
             💡 Try multi-key sequences like 'gg' or 'ge' - watch the status for 'Waiting...'\n\
             🚪 Ctrl+C=quit | ?=help | F1/F2=client actions (not canvas)"
        }
        AppMode::Edit => {
            "✏️  INSERT MODE - Keys handled by keymap system\n\
             \n\
             🔄 NAVIGATION: arrows | Ctrl+arrows(words) | Home/End(line) | Tab/Shift+Tab(fields)\n\
             🗑️  DELETE: Backspace/Delete\n\
             🚪 EXIT: Esc=normal\n\
             \n\
             💡 Type text normally - the keymap handles navigation!"
        }
        AppMode::Highlight => {
            "🎯 VISUAL MODE - Selection extended by keymap movements\n\
             \n\
             📍 EXTEND: hjkl(basic) | w/b/e(words) | 0/$(line) | gg/G(fields)\n\
             🔄 SWITCH: V=toggle-line-mode\n\
             🚪 EXIT: Esc=normal\n\
             \n\
             💡 All movements extend the selection automatically!"
        }
        _ => "🎯 Keymap system active!",
    };

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("🚀 Centralized Keymap System"),
        )
        .style(Style::default().fg(Color::Gray));

    f.render_widget(help, chunks[1]);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Canvas Keymap Demo");
    println!("✅ canvas-keymap feature: ENABLED");
    println!("🚀 Centralized key handling: ACTIVE");
    println!("📖 Multi-key sequences: SUPPORTED (gg, ge, gE, etc.)");
    println!();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = KeymapDemoApp::new();
    let res = run_app(&mut terminal, app);

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

    println!("🎯 Keymap demo completed!");
    Ok(())
}
