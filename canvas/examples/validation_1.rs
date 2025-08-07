// examples/validation_1.rs
//! Demonstrates field validation with the canvas library
//!
//! This example REQUIRES the `validation` feature to compile.
//!
//! Run with:
//! cargo run --example validation_1 --features "gui,validation"
//!
//! This will fail without validation:
//! cargo run --example validation_1 --features "gui"

// REQUIRE validation feature - example won't compile without it
#[cfg(not(feature = "validation"))]
compile_error!(
    "This example requires the 'validation' feature. \
     Run with: cargo run --example validation_1 --features \"gui,validation\""
);

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
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};

use canvas::{
    canvas::{
        gui::render_canvas_default,
        modes::AppMode,
    },
    DataProvider, FormEditor,
    ValidationConfig, ValidationConfigBuilder, CharacterLimits, ValidationResult,
};

// Import CountMode from the validation module directly
use canvas::validation::limits::CountMode;

// Enhanced FormEditor that demonstrates validation functionality
struct ValidationFormEditor<D: DataProvider> {
    editor: FormEditor<D>,
    has_unsaved_changes: bool,
    debug_message: String,
    command_buffer: String,
    validation_enabled: bool,
    field_switch_blocked: bool,
    block_reason: Option<String>,
}

impl<D: DataProvider> ValidationFormEditor<D> {
    fn new(data_provider: D) -> Self {
        let mut editor = FormEditor::new(data_provider);
        
        // Enable validation by default
        editor.set_validation_enabled(true);
        
        Self {
            editor,
            has_unsaved_changes: false,
            debug_message: "🔍 Validation Demo - Try typing in different fields!".to_string(),
            command_buffer: String::new(),
            validation_enabled: true,
            field_switch_blocked: false,
            block_reason: None,
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

    // === VALIDATION CONTROL ===
    fn toggle_validation(&mut self) {
        self.validation_enabled = !self.validation_enabled;
        self.editor.set_validation_enabled(self.validation_enabled);
        
        if self.validation_enabled {
            self.debug_message = "✅ Validation ENABLED - Try exceeding limits!".to_string();
        } else {
            self.debug_message = "❌ Validation DISABLED - No limits enforced".to_string();
        }
    }

    fn check_field_switch_allowed(&self) -> (bool, Option<String>) {
        if !self.validation_enabled {
            return (true, None);
        }
        
        let can_switch = self.editor.can_switch_fields();
        let reason = if !can_switch {
            self.editor.field_switch_block_reason()
        } else {
            None
        };
        
        (can_switch, reason)
    }

    fn get_validation_status(&self) -> String {
        if !self.validation_enabled {
            return "❌ DISABLED".to_string();
        }

        if self.field_switch_blocked {
            return "🚫 SWITCH BLOCKED".to_string();
        }

        let summary = self.editor.validation_summary();
        if summary.has_errors() {
            format!("❌ {} ERRORS", summary.error_fields)
        } else if summary.has_warnings() {
            format!("⚠️  {} WARNINGS", summary.warning_fields)
        } else if summary.validated_fields > 0 {
            format!("✅ {} VALID", summary.valid_fields)
        } else {
            "🔍 READY".to_string()
        }
    }

    fn validate_current_field(&mut self) {
        let result = self.editor.validate_current_field();
        match result {
            ValidationResult::Valid => {
                self.debug_message = "✅ Current field is valid!".to_string();
            }
            ValidationResult::Warning { message } => {
                self.debug_message = format!("⚠️  Warning: {}", message);
            }
            ValidationResult::Error { message } => {
                self.debug_message = format!("❌ Error: {}", message);
            }
        }
    }

    fn validate_all_fields(&mut self) {
        let field_count = self.editor.data_provider().field_count();
        for i in 0..field_count {
            self.editor.validate_field(i);
        }
        
        let summary = self.editor.validation_summary();
        self.debug_message = format!(
            "🔍 Validated all fields: {} valid, {} warnings, {} errors",
            summary.valid_fields, summary.warning_fields, summary.error_fields
        );
    }

    fn clear_validation_results(&mut self) {
        self.editor.clear_validation_results();
        self.debug_message = "🧹 Cleared all validation results".to_string();
    }

    // === ENHANCED MOVEMENT WITH VALIDATION ===
    fn move_left(&mut self) {
        self.editor.move_left();
        self.field_switch_blocked = false;
        self.block_reason = None;
    }

    fn move_right(&mut self) {
        self.editor.move_right();
        self.field_switch_blocked = false;
        self.block_reason = None;
    }

    fn move_up(&mut self) {
        match self.editor.move_up() {
            Ok(()) => {
                self.update_field_validation_status();
                self.field_switch_blocked = false;
                self.block_reason = None;
            }
            Err(e) => {
                self.field_switch_blocked = true;
                self.block_reason = Some(e.to_string());
                self.debug_message = format!("🚫 Field switch blocked: {}", e);
            }
        }
    }

    fn move_down(&mut self) {
        match self.editor.move_down() {
            Ok(()) => {
                self.update_field_validation_status();
                self.field_switch_blocked = false;
                self.block_reason = None;
            }
            Err(e) => {
                self.field_switch_blocked = true;
                self.block_reason = Some(e.to_string());
                self.debug_message = format!("🚫 Field switch blocked: {}", e);
            }
        }
    }

    fn move_line_start(&mut self) {
        self.editor.move_line_start();
    }

    fn move_line_end(&mut self) {
        self.editor.move_line_end();
    }

    fn move_word_next(&mut self) {
        self.editor.move_word_next();
    }

    fn move_word_prev(&mut self) {
        self.editor.move_word_prev();
    }

    fn move_word_end(&mut self) {
        self.editor.move_word_end();
    }

    fn move_first_line(&mut self) {
        self.editor.move_first_line();
    }

    fn move_last_line(&mut self) {
        self.editor.move_last_line();
    }

    fn update_field_validation_status(&mut self) {
        if !self.validation_enabled {
            return;
        }

        if let Some(result) = self.editor.current_field_validation() {
            match result {
                ValidationResult::Valid => {
                    self.debug_message = format!("Field {}: ✅ Valid", self.editor.current_field() + 1);
                }
                ValidationResult::Warning { message } => {
                    self.debug_message = format!("Field {}: ⚠️  {}", self.editor.current_field() + 1, message);
                }
                ValidationResult::Error { message } => {
                    self.debug_message = format!("Field {}: ❌ {}", self.editor.current_field() + 1, message);
                }
            }
        } else {
            self.debug_message = format!("Field {}: 🔍 Not validated yet", self.editor.current_field() + 1);
        }
    }

    // === MODE TRANSITIONS ===
    fn enter_edit_mode(&mut self) {
        self.editor.enter_edit_mode();
        self.debug_message = "✏️  INSERT MODE - Type to test validation".to_string();
    }

    fn enter_append_mode(&mut self) {
        self.editor.enter_append_mode();
        self.debug_message = "✏️  INSERT (append) - Validation active".to_string();
    }

    fn exit_edit_mode(&mut self) {
        self.editor.exit_edit_mode();
        self.debug_message = "🔒 NORMAL MODE - Press 'v' to validate current field".to_string();
        self.update_field_validation_status();
    }

    fn insert_char(&mut self, ch: char) -> anyhow::Result<()> {
        let result = self.editor.insert_char(ch);
        if result.is_ok() {
            self.has_unsaved_changes = true;
            // Show real-time validation feedback
            if let Some(validation_result) = self.editor.current_field_validation() {
                match validation_result {
                    ValidationResult::Valid => {
                        // Don't spam with valid messages, just show character count if applicable
                        if let Some(limits) = self.get_current_field_limits() {
                            if let Some(status) = limits.status_text(self.editor.current_text()) {
                                self.debug_message = format!("✏️  {}", status);
                            }
                        }
                    }
                    ValidationResult::Warning { message } => {
                        self.debug_message = format!("⚠️  {}", message);
                    }
                    ValidationResult::Error { message } => {
                        self.debug_message = format!("❌ {}", message);
                    }
                }
            }
        }
        Ok(result?)
    }

    fn get_current_field_limits(&self) -> Option<&CharacterLimits> {
        let validation_state = self.editor.validation_state();
        let config = validation_state.get_field_config(self.editor.current_field())?;
        config.character_limits.as_ref()
    }

    // === DELETE OPERATIONS ===
    fn delete_backward(&mut self) -> anyhow::Result<()> {
        let result = self.editor.delete_backward();
        if result.is_ok() {
            self.has_unsaved_changes = true;
            self.debug_message = "⌫ Deleted character".to_string();
        }
        Ok(result?)
    }

    fn delete_forward(&mut self) -> anyhow::Result<()> {
        let result = self.editor.delete_forward();
        if result.is_ok() {
            self.has_unsaved_changes = true;
            self.debug_message = "⌦ Deleted character".to_string();
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
    }

    fn next_field(&mut self) {
        match self.editor.next_field() {
            Ok(()) => {
                self.update_field_validation_status();
                self.field_switch_blocked = false;
                self.block_reason = None;
            }
            Err(e) => {
                self.field_switch_blocked = true;
                self.block_reason = Some(e.to_string());
                self.debug_message = format!("🚫 Cannot move to next field: {}", e);
            }
        }
    }

    fn prev_field(&mut self) {
        match self.editor.prev_field() {
            Ok(()) => {
                self.update_field_validation_status();
                self.field_switch_blocked = false;
                self.block_reason = None;
            }
            Err(e) => {
                self.field_switch_blocked = true;
                self.block_reason = Some(e.to_string());
                self.debug_message = format!("🚫 Cannot move to previous field: {}", e);
            }
        }
    }

    // === STATUS AND DEBUG ===
    fn set_debug_message(&mut self, msg: String) {
        self.debug_message = msg;
    }

    fn debug_message(&self) -> &str {
        &self.debug_message
    }

    fn has_unsaved_changes(&self) -> bool {
        self.has_unsaved_changes
    }
}

// Demo form data with different validation rules
struct ValidationDemoData {
    fields: Vec<(String, String)>,
}

impl ValidationDemoData {
    fn new() -> Self {
        Self {
            fields: vec![
                ("👤 Name (max 20)".to_string(), "".to_string()),
                ("📧 Email (max 50, warn@40)".to_string(), "".to_string()),
                ("🔑 Password (5-20 chars)".to_string(), "".to_string()),
                ("🔢 ID (min 3, max 10)".to_string(), "".to_string()),
                ("📝 Comment (min 10, max 100)".to_string(), "".to_string()),
                ("🏷️  Tag (max 30, bytes)".to_string(), "".to_string()),
                ("🌍 Unicode (width, min 2)".to_string(), "".to_string()),
            ],
        }
    }
}

impl DataProvider for ValidationDemoData {
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

    // 🎯 NEW: Validation configuration per field
    fn validation_config(&self, field_index: usize) -> Option<ValidationConfig> {
        match field_index {
            0 => Some(ValidationConfig::with_max_length(20)), // Name: simple 20 char limit
            1 => Some(
                ValidationConfigBuilder::new()
                    .with_character_limits(
                        CharacterLimits::new(50).with_warning_threshold(40)
                    )
                    .build()
            ), // Email: 50 chars with warning at 40
            2 => Some(
                ValidationConfigBuilder::new()
                    .with_character_limits(CharacterLimits::new_range(5, 20))
                    .build()
            ), // Password: must be 5-20 characters (blocks field switching if 1-4 chars)
            3 => Some(
                ValidationConfigBuilder::new()
                    .with_character_limits(CharacterLimits::new_range(3, 10))
                    .build()
            ), // ID: must be 3-10 characters (blocks field switching if 1-2 chars)
            4 => Some(
                ValidationConfigBuilder::new()
                    .with_character_limits(CharacterLimits::new_range(10, 100))
                    .build()
            ), // Comment: must be 10-100 characters (blocks field switching if 1-9 chars)
            5 => Some(
                ValidationConfigBuilder::new()
                    .with_character_limits(
                        CharacterLimits::new(30).with_count_mode(CountMode::Bytes)
                    )
                    .build()
            ), // Tag: 30 bytes (useful for UTF-8)
            6 => Some(
                ValidationConfigBuilder::new()
                    .with_character_limits(
                        CharacterLimits::new_range(2, 20).with_count_mode(CountMode::DisplayWidth)
                    )
                    .build()
            ), // Unicode: 2-20 display width (useful for CJK characters, blocks if 1 char)
            _ => None,
        }
    }
}

/// Handle key presses with validation-focused commands
fn handle_key_press(
    key: KeyCode,
    modifiers: KeyModifiers,
    editor: &mut ValidationFormEditor<ValidationDemoData>,
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
            editor.enter_edit_mode();
            editor.clear_command_buffer();
        }
        (AppMode::ReadOnly, KeyCode::Char('a'), _) => {
            editor.enter_append_mode();
            editor.clear_command_buffer();
        }
        (AppMode::ReadOnly, KeyCode::Char('A'), _) => {
            editor.move_line_end();
            editor.enter_edit_mode();
            editor.clear_command_buffer();
        }

        // Escape: Exit edit mode
        (_, KeyCode::Esc, _) => {
            if mode == AppMode::Edit {
                editor.exit_edit_mode();
            } else {
                editor.clear_command_buffer();
            }
        }

        // === VALIDATION COMMANDS ===
        (AppMode::ReadOnly, KeyCode::Char('v'), _) => {
            editor.validate_current_field();
            editor.clear_command_buffer();
        }
        (AppMode::ReadOnly, KeyCode::Char('V'), _) => {
            editor.validate_all_fields();
            editor.clear_command_buffer();
        }
        (AppMode::ReadOnly, KeyCode::Char('c'), _) => {
            editor.clear_validation_results();
            editor.clear_command_buffer();
        }
        (AppMode::ReadOnly, KeyCode::F(1), _) => {
            editor.toggle_validation();
        }

        // === MOVEMENT ===
        (AppMode::ReadOnly, KeyCode::Char('h'), _) | (AppMode::ReadOnly, KeyCode::Left, _) => {
            editor.move_left();
            editor.clear_command_buffer();
        }
        (AppMode::ReadOnly, KeyCode::Char('l'), _) | (AppMode::ReadOnly, KeyCode::Right, _) => {
            editor.move_right();
            editor.clear_command_buffer();
        }
        (AppMode::ReadOnly, KeyCode::Char('j'), _) | (AppMode::ReadOnly, KeyCode::Down, _) => {
            editor.move_down();
            editor.clear_command_buffer();
        }
        (AppMode::ReadOnly, KeyCode::Char('k'), _) | (AppMode::ReadOnly, KeyCode::Up, _) => {
            editor.move_up();
            editor.clear_command_buffer();
        }

        // === EDIT MODE MOVEMENT ===
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

        // === DELETE OPERATIONS ===
        (AppMode::Edit, KeyCode::Backspace, _) => {
            editor.delete_backward()?;
        }
        (AppMode::Edit, KeyCode::Delete, _) => {
            editor.delete_forward()?;
        }

        // === TAB NAVIGATION ===
        (_, KeyCode::Tab, _) => {
            editor.next_field();
        }
        (_, KeyCode::BackTab, _) => {
            editor.prev_field();
        }

        // === CHARACTER INPUT ===
        (AppMode::Edit, KeyCode::Char(c), m) if !m.contains(KeyModifiers::CONTROL) => {
            editor.insert_char(c)?;
        }

        // === DEBUG/INFO COMMANDS ===
        (AppMode::ReadOnly, KeyCode::Char('?'), _) => {
            let summary = editor.editor.validation_summary();
            editor.set_debug_message(format!(
                "Field {}/{}, Pos {}, Mode: {:?}, Validation: {} fields configured, {} validated",
                editor.current_field() + 1,
                editor.data_provider().field_count(),
                editor.cursor_position(),
                editor.mode(),
                summary.total_fields,
                summary.validated_fields
            ));
        }

        _ => {
            if editor.has_pending_command() {
                editor.clear_command_buffer();
                editor.set_debug_message("Invalid command sequence".to_string());
            }
        }
    }

    Ok(true)
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut editor: ValidationFormEditor<ValidationDemoData>,
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

fn ui(f: &mut Frame, editor: &ValidationFormEditor<ValidationDemoData>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(12)])
        .split(f.area());

    render_enhanced_canvas(f, chunks[0], editor);
    render_validation_status(f, chunks[1], editor);
}

fn render_enhanced_canvas(
    f: &mut Frame,
    area: Rect,
    editor: &ValidationFormEditor<ValidationDemoData>,
) {
    render_canvas_default(f, area, &editor.editor);
}

fn render_validation_status(
    f: &mut Frame,
    area: Rect,
    editor: &ValidationFormEditor<ValidationDemoData>,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Status bar
            Constraint::Length(4), // Validation summary
            Constraint::Length(5), // Help
        ])
        .split(area);

    // Status bar with validation information
    let mode_text = match editor.mode() {
        AppMode::Edit => "INSERT",
        AppMode::ReadOnly => "NORMAL",
        _ => "OTHER",
    };

    let validation_status = editor.get_validation_status();
    let status_text = if editor.has_pending_command() {
        format!("-- {} -- {} [{}] | Validation: {}", 
                mode_text, editor.debug_message(), editor.get_command_buffer(), validation_status)
    } else if editor.has_unsaved_changes() {
        format!("-- {} -- [Modified] {} | Validation: {}", 
                mode_text, editor.debug_message(), validation_status)
    } else {
        format!("-- {} -- {} | Validation: {}", 
                mode_text, editor.debug_message(), validation_status)
    };

    let status = Paragraph::new(Line::from(Span::raw(status_text)))
        .block(Block::default().borders(Borders::ALL).title("🔍 Validation Status"));

    f.render_widget(status, chunks[0]);

    // Validation summary with field switching info
    let summary = editor.editor.validation_summary();
    let summary_text = if editor.validation_enabled {
        let switch_info = if editor.field_switch_blocked {
            format!("\n🚫 Field switching blocked: {}", 
                   editor.block_reason.as_deref().unwrap_or("Unknown reason"))
        } else {
            let (can_switch, reason) = editor.check_field_switch_allowed();
            if !can_switch {
                format!("\n⚠️  Field switching will be blocked: {}",
                       reason.as_deref().unwrap_or("Unknown reason"))
            } else {
                "\n✅ Field switching allowed".to_string()
            }
        };
        
        format!(
            "📊 Validation Summary: {} fields configured, {} validated{}\n\
             ✅ Valid: {}  ⚠️  Warnings: {}  ❌ Errors: {}  📈 Progress: {:.0}%",
            summary.total_fields,
            summary.validated_fields,
            switch_info,
            summary.valid_fields,
            summary.warning_fields,
            summary.error_fields,
            summary.completion_percentage() * 100.0
        )
    } else {
        "❌ Validation is currently DISABLED\nPress F1 to enable validation".to_string()
    };

    let summary_style = if summary.has_errors() {
        Style::default().fg(Color::Red)
    } else if summary.has_warnings() {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Green)
    };

    let validation_summary = Paragraph::new(summary_text)
        .block(Block::default().borders(Borders::ALL).title("📈 Validation Overview"))
        .style(summary_style)
        .wrap(Wrap { trim: true });

    f.render_widget(validation_summary, chunks[1]);

    // Enhanced help text
    let help_text = match editor.mode() {
        AppMode::ReadOnly => {
            "🔍 VALIDATION DEMO: Different fields have different limits!\n\
             Fields with MINIMUM requirements will block field switching if too short!\n\
             Movement: hjkl/arrows=move, Tab/Shift+Tab=fields\n\
             Edit: i/a/A=insert modes, Esc=normal\n\
             Validation: v=validate current, V=validate all, c=clear results, F1=toggle\n\
             ?=info, Ctrl+C/Ctrl+Q=quit"
        }
        AppMode::Edit => {
            "✏️  INSERT MODE - Type to test validation limits!\n\
             Some fields have MINIMUM character requirements!\n\
             Try typing 1-2 chars in Password/ID/Comment fields, then try to switch!\n\
             arrows=move, Backspace/Del=delete, Esc=normal, Tab=next field\n\
             Field switching may be BLOCKED if minimum requirements not met!"
        }
        _ => "🔍 Validation Demo Active!"
    };

    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("🚀 Validation Commands"))
        .style(Style::default().fg(Color::Gray))
        .wrap(Wrap { trim: true });

    f.render_widget(help, chunks[2]);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Print feature status
    println!("🔍 Canvas Validation Demo");
    println!("✅ validation feature: ENABLED");
    println!("🚀 Field validation: ACTIVE");
    println!("🚫 Field switching validation: ACTIVE");
    println!("📊 Try typing in fields with minimum requirements!");
    println!("   - Password (min 5): Type 1-4 chars, then try to switch fields");
    println!("   - ID (min 3): Type 1-2 chars, then try to switch fields");
    println!("   - Comment (min 10): Type 1-9 chars, then try to switch fields");
    println!("   - Unicode (min 2): Type 1 char, then try to switch fields");
    println!();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let data = ValidationDemoData::new();
    let editor = ValidationFormEditor::new(data);

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

    println!("🔍 Validation demo completed!");
    Ok(())
}
