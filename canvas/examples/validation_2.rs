// examples/validation_patterns_tui.rs
//! TUI Example demonstrating position-based pattern filtering
//!
//! Run with: cargo run --example validation_patterns_tui --features "validation,gui"

// REQUIRE validation and gui features - example won't compile without them
#[cfg(not(all(feature = "validation", feature = "gui")))]
compile_error!(
    "This example requires the 'validation' and 'gui' features. \
     Run with: cargo run --example validation_patterns_tui --features \"validation,gui\""
);

use std::io;
use canvas::ValidationResult;
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
    ValidationConfig, ValidationConfigBuilder, PatternFilters, PositionFilter, PositionRange, CharacterFilter,
};

// Enhanced FormEditor for pattern validation demonstration
struct PatternValidationFormEditor<D: DataProvider> {
    editor: FormEditor<D>,
    debug_message: String,
    command_buffer: String,
    validation_enabled: bool,
    field_switch_blocked: bool,
    block_reason: Option<String>,
}

impl<D: DataProvider> PatternValidationFormEditor<D> {
    fn new(data_provider: D) -> Self {
        let mut editor = FormEditor::new(data_provider);
        
        // Enable validation by default
        editor.set_validation_enabled(true);
        
        Self {
            editor,
            debug_message: "🔍 Pattern Validation Demo - Try typing in different fields!".to_string(),
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
            self.debug_message = "✅ Pattern Validation ENABLED".to_string();
        } else {
            self.debug_message = "❌ Pattern Validation DISABLED".to_string();
        }
    }
    
    // === MOVEMENT ===
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
    
    // === MODE TRANSITIONS ===
    fn enter_edit_mode(&mut self) {
        self.editor.enter_edit_mode();
        self.debug_message = "✏️  INSERT MODE - Type to test pattern validation".to_string();
    }
    
    fn enter_append_mode(&mut self) {
        self.editor.enter_append_mode();
        self.debug_message = "✏️  INSERT (append) - Pattern validation active".to_string();
    }
    
    fn exit_edit_mode(&mut self) {
        self.editor.exit_edit_mode();
        self.debug_message = "🔒 NORMAL MODE".to_string();
        self.update_field_validation_status();
    }
    
    fn insert_char(&mut self, ch: char) -> anyhow::Result<()> {
        let result = self.editor.insert_char(ch);
        if result.is_ok() {
            // Show real-time validation feedback
            if let Some(validation_result) = self.editor.current_field_validation() {
                match validation_result {
                    ValidationResult::Valid => {
                        self.debug_message = "✅ Valid character".to_string();
                    }
                    ValidationResult::Warning { message } => {
                        self.debug_message = format!("⚠️  Warning: {}", message);
                    }
                    ValidationResult::Error { message } => {
                        self.debug_message = format!("❌ Error: {}", message);
                    }
                }
            }
        }
        Ok(result?)
    }
    
    // === DELETE OPERATIONS ===
    fn delete_backward(&mut self) -> anyhow::Result<()> {
        let result = self.editor.delete_backward();
        if result.is_ok() {
            self.debug_message = "⌫ Deleted character".to_string();
        }
        Ok(result?)
    }
    
    fn delete_forward(&mut self) -> anyhow::Result<()> {
        let result = self.editor.delete_forward();
        if result.is_ok() {
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
}

// Demo form with pattern-based validation
struct PatternValidationData {
    fields: Vec<(String, String)>,
}

impl PatternValidationData {
    fn new() -> Self {
        Self {
            fields: vec![
                ("🚗 License Plate (AB123)".to_string(), "".to_string()),
                ("📞 Phone (123-456-7890)".to_string(), "".to_string()),
                ("💳 Credit Card (1234-5678-9012-3456)".to_string(), "".to_string()),
                ("🆔 Custom ID (AB123def)".to_string(), "".to_string()),
            ],
        }
    }
}

impl DataProvider for PatternValidationData {
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
    
    // Pattern validation configuration per field
    fn validation_config(&self, field_index: usize) -> Option<ValidationConfig> {
        match field_index {
            0 => {
                // License plate: AB123 (2 letters, 3 numbers)
                let license_plate_pattern = PatternFilters::new()
                    .add_filter(PositionFilter::new(
                        PositionRange::Range(0, 1),
                        CharacterFilter::Alphabetic,
                    ))
                    .add_filter(PositionFilter::new(
                        PositionRange::Range(2, 4),
                        CharacterFilter::Numeric,
                    ));
                
                Some(ValidationConfigBuilder::new()
                    .with_pattern_filters(license_plate_pattern)
                    .build())
            }
            1 => {
                // Phone number: 123-456-7890
                let phone_pattern = PatternFilters::new()
                    .add_filter(PositionFilter::new(
                        PositionRange::Multiple(vec![0,1,2,4,5,6,8,9,10,11]),
                        CharacterFilter::Numeric,
                    ))
                    .add_filter(PositionFilter::new(
                        PositionRange::Multiple(vec![3, 7]),
                        CharacterFilter::Exact('-'),
                    ));
                
                Some(ValidationConfigBuilder::new()
                    .with_pattern_filters(phone_pattern)
                    .build())
            }
            2 => {
                // Credit card: 1234-5678-9012-3456
                let credit_card_pattern = PatternFilters::new()
                    .add_filter(PositionFilter::new(
                        PositionRange::Multiple(vec![0,1,2,3,5,6,7,8,10,11,12,13,15,16,17,18]),
                        CharacterFilter::Numeric,
                    ))
                    .add_filter(PositionFilter::new(
                        PositionRange::Multiple(vec![4, 9, 14]),
                        CharacterFilter::Exact('-'),
                    ));
                
                Some(ValidationConfigBuilder::new()
                    .with_pattern_filters(credit_card_pattern)
                    .build())
            }
            3 => {
                // Custom ID: First 2 letters, rest alphanumeric
                let custom_id_pattern = PatternFilters::new()
                    .add_filter(PositionFilter::new(
                        PositionRange::Range(0, 1),
                        CharacterFilter::Alphabetic,
                    ))
                    .add_filter(PositionFilter::new(
                        PositionRange::From(2),
                        CharacterFilter::Alphanumeric,
                    ));
                
                Some(ValidationConfigBuilder::new()
                    .with_pattern_filters(custom_id_pattern)
                    .build())
            }
            _ => None,
        }
    }
}

/// Handle key presses with pattern validation commands
fn handle_key_press(
    key: KeyCode,
    modifiers: KeyModifiers,
    editor: &mut PatternValidationFormEditor<PatternValidationData>,
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
    mut editor: PatternValidationFormEditor<PatternValidationData>,
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

fn ui(f: &mut Frame, editor: &PatternValidationFormEditor<PatternValidationData>) {
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
    editor: &PatternValidationFormEditor<PatternValidationData>,
) {
    render_canvas_default(f, area, &editor.editor);
}

fn render_validation_status(
    f: &mut Frame,
    area: Rect,
    editor: &PatternValidationFormEditor<PatternValidationData>,
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
        format!("-- {} -- {} [{}] | Pattern Validation: {}",
                mode_text, editor.debug_message(), editor.get_command_buffer(), validation_status)
    } else {
        format!("-- {} -- {} | Pattern Validation: {}",
                mode_text, editor.debug_message(), validation_status)
    };
    
    let status = Paragraph::new(Line::from(Span::raw(status_text)))
        .block(Block::default().borders(Borders::ALL).title("🔍 Pattern Validation Status"));
    
    f.render_widget(status, chunks[0]);
    
    // Validation summary with field switching info
    let summary = editor.editor.validation_summary();
    let summary_text = if editor.validation_enabled {
        let switch_info = if editor.field_switch_blocked {
            format!("\n🚫 Field switching blocked: {}",
                   editor.block_reason.as_deref().unwrap_or("Unknown reason"))
        } else {
            "\n✅ Field switching allowed".to_string()
        };
        
        format!(
            "📊 Pattern Validation Summary: {} fields configured, {} validated{}\n\
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
        "❌ Pattern validation is currently DISABLED\nPress F1 to enable validation".to_string()
    };
    
    let summary_style = if summary.has_errors() {
        Style::default().fg(Color::Red)
    } else if summary.has_warnings() {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Green)
    };
    
    let validation_summary = Paragraph::new(summary_text)
        .block(Block::default().borders(Borders::ALL).title("📈 Pattern Validation Overview"))
        .style(summary_style)
        .wrap(Wrap { trim: true });
    
    f.render_widget(validation_summary, chunks[1]);
    
    // Pattern-specific help text
    let help_text = match editor.mode() {
        AppMode::ReadOnly => {
            "🔍 PATTERN VALIDATION DEMO: Each field has specific character patterns!\n\
             License Plate: 2 letters + 3 numbers (AB123)\n\
             Phone: Numbers with dashes at positions 3 and 7 (123-456-7890)\n\
             Credit Card: Number groups separated by dashes (1234-5678-9012-3456)\n\
             Custom ID: 2 letters + alphanumeric (AB123def)\n\
             Movement: hjkl/arrows=move, Tab/Shift+Tab=fields, i/a=insert, F1=toggle validation"
        }
        AppMode::Edit => {
            "✏️  INSERT MODE - Type to test pattern validation!\n\
             Pattern validation will reject characters that don't match the expected pattern\n\
             arrows=move, Backspace/Del=delete, Esc=normal, Tab=next field"
        }
        _ => "🔍 Pattern Validation Demo Active!"
    };
    
    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("🚀 Pattern Validation Commands"))
        .style(Style::default().fg(Color::Gray))
        .wrap(Wrap { trim: true });
    
    f.render_widget(help, chunks[2]);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Print feature status
    println!("🔍 Canvas Pattern Validation TUI Demo");
    println!("✅ validation feature: ENABLED");
    println!("✅ gui feature: ENABLED");
    println!("🚀 Pattern-based validation: ACTIVE");
    println!("📊 Try typing in fields with different patterns!");
    println!();
    
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    let data = PatternValidationData::new();
    let editor = PatternValidationFormEditor::new(data);
    
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
    
    println!("🔍 Pattern validation demo completed!");
    Ok(())
}
