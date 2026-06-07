// examples/textarea_normal.rs
//! Demonstrates automatic cursor management with the textarea widget
//!
//! This example REQUIRES the `cursor-style` and `textarea` features to compile,
//! and is adapted for `textmode-normal` (always editing, no vim modes).
//!
//! Run with:
//! cargo run --example canvas_textarea_cursor_auto_normal --features "gui,cursor-style,textarea,textmode-normal"

#[cfg(not(feature = "cursor-style"))]
compile_error!(
    "This example requires the 'cursor-style' feature. \
     Run with: cargo run --example canvas_textarea_cursor_auto_normal --features \"gui,cursor-style,textarea,textmode-normal\""
);

#[cfg(not(feature = "textarea"))]
compile_error!(
    "This example requires the 'textarea' feature. \
     Run with: cargo run --example canvas_textarea_cursor_auto_normal --features \"gui,cursor-style,textarea,textmode-normal\""
);

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};

use tui_canvas::{
    CommandLine, CommandLineCommand, CommandLineEventOutcome, CommandLineRegistry,
    CommandLineState, CommandLineSubmit,
    integration::crossterm_input::{CrosstermInputOptions, CrosstermInputSession},
    CursorManager, TextArea, TextAreaState,
};

/// TextArea demo adapted for NORMALMODE (always editing)
struct AutoCursorTextArea {
    textarea: TextAreaState,
    commandline: CommandLineState,
    commands: CommandLineRegistry,
    has_unsaved_changes: bool,
    debug_message: String,
    command_buffer: String,
}

impl AutoCursorTextArea {
    fn new() -> Self {
        let initial_text = "🎯 Automatic Cursor Management Demo (NORMALMODE)\n\
Welcome to the textarea cursor demo!\n\
\n\
This demo runs in NORMALMODE:\n\
• Always editing (no insert/normal toggle)\n\
• Cursor is always underscore _\n\
\n\
Navigation commands:\n\
• hjkl or arrow keys: move cursor\n\
• w/b/e/W/B/E: word movements\n\
• 0/$: line start/end\n\
• g/gG: first/last line\n\
\n\
Editing commands:\n\
• x/X: delete characters\n\
\n\
Press ? for help, Ctrl+Q to quit.";

        let mut textarea = TextAreaState::from_text(initial_text);
        textarea.set_placeholder("Start typing...");

        Self {
            textarea,
            commandline: CommandLineState::new(),
            commands: textarea_commands(),
            has_unsaved_changes: false,
            debug_message: "🎯 NORMALMODE Demo - always editing".to_string(),
            command_buffer: String::new(),
        }
    }

    fn handle_textarea_input(&mut self, key: KeyEvent) {
        self.textarea.input(key);
        self.has_unsaved_changes = true;
    }

    fn move_left(&mut self) {
        self.textarea.move_left();
        self.debug_message = "← left".to_string();
    }
    fn move_right(&mut self) {
        self.textarea.move_right();
        self.debug_message = "→ right".to_string();
    }
    fn move_up(&mut self) {
        self.textarea.move_up();
        self.debug_message = "↑ up".to_string();
    }
    fn move_down(&mut self) {
        self.textarea.move_down();
        self.debug_message = "↓ down".to_string();
    }

    fn move_word_next(&mut self) {
        self.textarea.move_word_next();
        self.debug_message = "w: next word".to_string();
    }
    fn move_word_prev(&mut self) {
        self.textarea.move_word_prev();
        self.debug_message = "b: previous word".to_string();
    }
    fn move_word_end(&mut self) {
        self.textarea.move_word_end();
        self.debug_message = "e: word end".to_string();
    }
    fn move_word_end_prev(&mut self) {
        self.textarea.move_word_end_prev();
        self.debug_message = "ge: previous word end".to_string();
    }

    fn move_line_start(&mut self) {
        self.textarea.move_line_start();
        self.debug_message = "0: line start".to_string();
    }
    fn move_line_end(&mut self) {
        self.textarea.move_line_end();
        self.debug_message = "$: line end".to_string();
    }
    fn move_first_line(&mut self) {
        self.textarea.move_first_line();
        self.debug_message = "gg: first line".to_string();
    }
    fn move_last_line(&mut self) {
        self.textarea.move_last_line();
        self.debug_message = "G: last line".to_string();
    }

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

    fn debug_message(&self) -> &str {
        &self.debug_message
    }
    fn set_debug_message(&mut self, msg: String) {
        self.debug_message = msg;
    }
    fn has_unsaved_changes(&self) -> bool {
        self.has_unsaved_changes
    }
    fn get_cursor_info(&self) -> String {
        format!(
            "Line {}, Col {}",
            self.textarea.current_field() + 1,
            self.textarea.cursor_position() + 1
        )
    }

    // Big word movements

    fn move_big_word_next(&mut self) {
        self.textarea.move_big_word_next();
        self.debug_message = "W: next WORD".to_string();
    }

    fn move_big_word_prev(&mut self) {
        self.textarea.move_big_word_prev();
        self.debug_message = "B: previous WORD".to_string();
    }

    fn move_big_word_end(&mut self) {
        self.textarea.move_big_word_end();
        self.debug_message = "E: WORD end".to_string();
    }

    fn move_big_word_end_prev(&mut self) {
        self.textarea.move_big_word_end_prev();
        self.debug_message = "gE: previous WORD end".to_string();
    }

    fn apply_commandline_submit(&mut self, submit: CommandLineSubmit) {
        match submit {
            CommandLineSubmit::SearchForward(query) => {
                self.textarea.set_search_query(query.clone());
                let found = self.textarea.find_next();
                self.debug_message = if found {
                    format!("Search: /{query}")
                } else {
                    format!("Search: no matches for /{query}")
                };
            }
            CommandLineSubmit::SearchBackward(query) => {
                self.textarea.set_search_query(query.clone());
                let found = self.textarea.find_previous();
                self.debug_message = if found {
                    format!("Search: ?{query}")
                } else {
                    format!("Search: no matches for ?{query}")
                };
            }
            CommandLineSubmit::Command(command) => {
                self.apply_command(&command);
            }
        }
    }

    fn apply_command(&mut self, command: &str) {
        let invocation = match self.commands.dispatch(command) {
            Ok(invocation) => invocation,
            Err(err) => {
                self.debug_message = format!("Command error: {err:?}");
                return;
            }
        };

        match invocation.command.name.as_str() {
            "set-number" => {
                self.textarea.show_absolute_line_numbers();
                self.debug_message = "Command: line numbers absolute".to_string();
            }
            "set-relative-number" => {
                self.textarea.show_relative_line_numbers();
                self.debug_message = "Command: line numbers relative".to_string();
            }
            "set-no-number" => {
                self.textarea.hide_line_numbers();
                self.debug_message = "Command: line numbers none".to_string();
            }
            "no-highlight" => {
                self.textarea.clear_search();
                self.debug_message = "Command: search cleared".to_string();
            }
            other => {
                self.debug_message = format!("Command not handled: {other}");
            }
        }
    }
}

fn textarea_commands() -> CommandLineRegistry {
    let mut registry = CommandLineRegistry::new();
    registry
        .register(
            CommandLineCommand::new("set-number")
                .alias("number")
                .alias("nu")
                .pattern(["set", "number"])
                .pattern(["set", "nu"])
                .description("Show absolute line numbers"),
        )
        .unwrap()
        .register(
            CommandLineCommand::new("set-relative-number")
                .alias("relativenumber")
                .alias("rnu")
                .pattern(["set", "relativenumber"])
                .pattern(["set", "rnu"])
                .description("Show relative line numbers"),
        )
        .unwrap()
        .register(
            CommandLineCommand::new("set-no-number")
                .alias("nonumber")
                .alias("nonu")
                .pattern(["set", "nonumber"])
                .pattern(["set", "nonu"])
                .description("Hide line numbers"),
        )
        .unwrap()
        .register(
            CommandLineCommand::new("no-highlight")
                .alias("noh")
                .alias("nohlsearch")
                .description("Clear search highlights"),
        )
        .unwrap();
    registry
}

/// Handle key press in NORMALMODE (always editing, casual editor style)
fn handle_key_press(key_event: KeyEvent, editor: &mut AutoCursorTextArea) -> anyhow::Result<bool> {
    match editor.commandline.input_key(key_event) {
        CommandLineEventOutcome::Ignored => {}
        CommandLineEventOutcome::Handled => return Ok(true),
        CommandLineEventOutcome::Cancelled => {
            editor.set_debug_message("Command line cancelled".to_string());
            return Ok(true);
        }
        CommandLineEventOutcome::Submitted(submit) => {
            editor.apply_commandline_submit(submit);
            return Ok(true);
        }
    }

    let KeyEvent {
        code: key,
        modifiers,
        ..
    } = key_event;

    // Quit
    if (key == KeyCode::Char('q') && modifiers.contains(KeyModifiers::CONTROL))
        || (key == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL))
        || key == KeyCode::F(10)
    {
        return Ok(false);
    }

    match (key, modifiers) {
        // Movement
        (KeyCode::Left, _) => editor.move_left(),
        (KeyCode::Right, _) => editor.move_right(),
        (KeyCode::Up, _) => editor.move_up(),
        (KeyCode::Down, _) => editor.move_down(),

        // Word movement (Ctrl+Arrows)
        (KeyCode::Left, m) if m.contains(KeyModifiers::CONTROL) => editor.move_word_prev(),
        (KeyCode::Right, m) if m.contains(KeyModifiers::CONTROL) => editor.move_word_next(),
        (KeyCode::Right, m) if m.contains(KeyModifiers::CONTROL | KeyModifiers::SHIFT) => {
            editor.move_word_end()
        }

        // Line/document movement
        (KeyCode::Home, _) => editor.move_line_start(),
        (KeyCode::End, _) => editor.move_line_end(),
        (KeyCode::Home, m) if m.contains(KeyModifiers::CONTROL) => editor.move_first_line(),
        (KeyCode::End, m) if m.contains(KeyModifiers::CONTROL) => editor.move_last_line(),

        // Delete
        (KeyCode::Delete, _) => editor.delete_char_forward(),
        (KeyCode::Backspace, _) => editor.delete_char_backward(),

        (KeyCode::F(1), _) => {
            // Switch to indicator mode
            editor.textarea.use_overflow_indicator('$');
            editor.set_debug_message("Overflow: indicator '$' (wrap OFF)".to_string());
        }
        (KeyCode::F(2), _) => {
            // Switch to wrap mode
            editor.textarea.use_wrap();
            editor.set_debug_message("Overflow: wrap ON".to_string());
        }

        (KeyCode::F(3), _) => {
            editor.textarea.set_wrap_indent_cols(3);
            editor.set_debug_message("Wrap indent: 3 columns".to_string());
        }
        (KeyCode::F(4), _) => {
            editor.textarea.set_wrap_indent_cols(0);
            editor.set_debug_message("Wrap indent: 0 columns".to_string());
        }
        (KeyCode::F(5), _) => {
            editor.textarea.hide_line_numbers();
            editor.set_debug_message("Line numbers: none".to_string());
        }
        (KeyCode::F(6), _) => {
            editor.textarea.show_absolute_line_numbers();
            editor.set_debug_message("Line numbers: absolute".to_string());
        }
        (KeyCode::F(7), _) => {
            editor.textarea.show_relative_line_numbers();
            editor.set_debug_message("Line numbers: relative".to_string());
        }
        (KeyCode::F(8), _) => {
            editor.textarea.set_search_query("textarea");
            let found = editor.textarea.find_next();
            editor.set_debug_message(if found {
                "Search: textarea".to_string()
            } else {
                "Search: no matches for textarea".to_string()
            });
        }
        (KeyCode::F(9), _) => {
            editor.textarea.clear_search();
            editor.set_debug_message("Search cleared".to_string());
        }
        (KeyCode::Char('n'), _) => {
            let found = editor.textarea.find_next();
            editor.set_debug_message(if found {
                "Search: next match".to_string()
            } else {
                "Search: no next match".to_string()
            });
        }
        (KeyCode::Char('N'), _) => {
            let found = editor.textarea.find_previous();
            editor.set_debug_message(if found {
                "Search: previous match".to_string()
            } else {
                "Search: no previous match".to_string()
            });
        }

        // Default: treat as text input
        _ => editor.handle_textarea_input(key_event),
    }

    Ok(true)
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    session: &CrosstermInputSession,
    mut editor: AutoCursorTextArea,
) -> std::io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut editor))?;

        match session.read_event()? {
            Event::Key(key) => match handle_key_press(key, &mut editor) {
                Ok(should_continue) => {
                    if !should_continue {
                        break;
                    }
                }
                Err(e) => {
                    editor.set_debug_message(format!("Error: {e}"));
                }
            },
            other => {
                editor.textarea.handle_event(other);
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

    if editor.commandline.is_active() {
        let (cx, cy) = editor.commandline.cursor(f.area());
        f.set_cursor_position((cx, cy));
    }
}

fn render_textarea(f: &mut Frame, area: ratatui::layout::Rect, editor: &mut AutoCursorTextArea) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("🎯 Textarea with NORMALMODE (always editing)");

    let textarea_widget = TextArea::default().block(block.clone());
    f.render_stateful_widget(textarea_widget, area, &mut editor.textarea);

    let (cx, cy) = editor.textarea.cursor(area, Some(&block));
    f.set_cursor_position((cx, cy));
}

fn render_status_and_help(f: &mut Frame, area: ratatui::layout::Rect, editor: &mut AutoCursorTextArea) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(4),
        ])
        .split(area);

    f.render_stateful_widget(CommandLine::default(), f.area(), &mut editor.commandline);

    let status_text = if editor.has_pending_command() {
        format!(
            "-- NORMALMODE (underscore cursor) -- {} [{}]",
            editor.debug_message(),
            editor.get_command_buffer()
        )
    } else if editor.has_unsaved_changes() {
        format!(
            "-- NORMALMODE (underscore cursor) -- [Modified] {} | {}",
            editor.debug_message(),
            editor.get_cursor_info()
        )
    } else {
        format!(
            "-- NORMALMODE (underscore cursor) -- {} | {}",
            editor.debug_message(),
            editor.get_cursor_info()
        )
    };

    let status = Paragraph::new(Line::from(Span::raw(status_text))).block(
        Block::default()
            .borders(Borders::ALL)
            .title("🎯 Cursor Status"),
    );

    f.render_widget(status, chunks[1]);

    let help_text = "🎯 NORMALMODE (always editing)\n\
hjkl/arrows=move, w/b/e=words, W/B/E=WORDS, 0/$=line, g/G=first/last\n\
x/X=delete, typing inserts text\n\
:/?=command/search, n/N=next/previous, :noh clears search, Ctrl+Q=quit";

    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("🚀 Help"))
        .style(Style::default().fg(Color::Gray));

    f.render_widget(help, chunks[2]);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Canvas Textarea Cursor Auto Demo (NORMALMODE)");
    println!("✅ cursor-style feature: ENABLED");
    println!("✅ textarea feature: ENABLED");
    println!("✅ textmode-normal feature: ENABLED");
    println!("🚀 Always editing, underscore cursor active");
    println!();

    let mut session =
        CrosstermInputSession::install_with_options(CrosstermInputOptions::tui_defaults())?;
    let backend = CrosstermBackend::new(std::io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let editor = AutoCursorTextArea::new();

    let res = run_app(&mut terminal, &session, editor);

    CursorManager::reset()?;
    let _ = session.uninstall();
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    println!("🎯 Cursor automatically reset to default!");
    Ok(())
}
