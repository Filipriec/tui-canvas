// examples/validation_2.rs
//! Advanced TUI Example demonstrating complex pattern filtering edge cases
//!
//! This example showcases the full potential of the pattern validation system
//! with creative real-world scenarios and edge cases.
//!
//! Run with: cargo run --example validation_advanced_patterns --features "validation,gui"

// REQUIRE validation and gui features
#[cfg(not(all(feature = "validation", feature = "gui")))]
compile_error!(
    "This example requires the 'validation' and 'gui' features. \
     Run with: cargo run --example validation_advanced_patterns --features \"validation,gui\""
);

use std::io;
use std::sync::Arc;
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

// Enhanced FormEditor wrapper (keeping the same structure as before)
struct AdvancedPatternFormEditor<D: DataProvider> {
    editor: FormEditor<D>,
    debug_message: String,
    command_buffer: String,
    validation_enabled: bool,
    field_switch_blocked: bool,
    block_reason: Option<String>,
}

impl<D: DataProvider> AdvancedPatternFormEditor<D> {
    fn new(data_provider: D) -> Self {
        let mut editor = FormEditor::new(data_provider);
        editor.set_validation_enabled(true);

        Self {
            editor,
            debug_message: "🚀 Advanced Pattern Validation - Showcasing edge cases and complex patterns!".to_string(),
            command_buffer: String::new(),
            validation_enabled: true,
            field_switch_blocked: false,
            block_reason: None,
        }
    }

    // ... (keeping all the same methods as before for brevity)
    // [All the previous methods: clear_command_buffer, add_to_command_buffer, etc.]
    
    fn clear_command_buffer(&mut self) { self.command_buffer.clear(); }
    fn add_to_command_buffer(&mut self, ch: char) { self.command_buffer.push(ch); }
    fn get_command_buffer(&self) -> &str { &self.command_buffer }
    fn has_pending_command(&self) -> bool { !self.command_buffer.is_empty() }
    
    fn toggle_validation(&mut self) {
        self.validation_enabled = !self.validation_enabled;
        self.editor.set_validation_enabled(self.validation_enabled);
        if self.validation_enabled {
            self.debug_message = "✅ Advanced Pattern Validation ENABLED".to_string();
        } else {
            self.debug_message = "❌ Advanced Pattern Validation DISABLED".to_string();
        }
    }

    fn move_left(&mut self) { self.editor.move_left(); self.field_switch_blocked = false; self.block_reason = None; }
    fn move_right(&mut self) { self.editor.move_right(); self.field_switch_blocked = false; self.block_reason = None; }
    
    fn move_up(&mut self) {
        match self.editor.move_up() {
            Ok(()) => { self.update_field_validation_status(); self.field_switch_blocked = false; self.block_reason = None; }
            Err(e) => { self.field_switch_blocked = true; self.block_reason = Some(e.to_string()); self.debug_message = format!("🚫 Field switch blocked: {}", e); }
        }
    }
    
    fn move_down(&mut self) {
        match self.editor.move_down() {
            Ok(()) => { self.update_field_validation_status(); self.field_switch_blocked = false; self.block_reason = None; }
            Err(e) => { self.field_switch_blocked = true; self.block_reason = Some(e.to_string()); self.debug_message = format!("🚫 Field switch blocked: {}", e); }
        }
    }

    fn move_line_start(&mut self) { self.editor.move_line_start(); }
    fn move_line_end(&mut self) { self.editor.move_line_end(); }
    
    fn enter_edit_mode(&mut self) {
        self.editor.enter_edit_mode();
        self.debug_message = "✏️  INSERT MODE - Testing advanced pattern validation".to_string();
    }
    
    fn enter_append_mode(&mut self) {
        self.editor.enter_append_mode();
        self.debug_message = "✏️  INSERT (append) - Advanced patterns active".to_string();
    }
    
    fn exit_edit_mode(&mut self) {
        self.editor.exit_edit_mode();
        self.debug_message = "🔒 NORMAL MODE".to_string();
        self.update_field_validation_status();
    }

    fn insert_char(&mut self, ch: char) -> anyhow::Result<()> {
        let result = self.editor.insert_char(ch);
        if result.is_ok() {
            if let Some(validation_result) = self.editor.current_field_validation() {
                match validation_result {
                    ValidationResult::Valid => { self.debug_message = "✅ Character accepted".to_string(); }
                    ValidationResult::Warning { message } => { self.debug_message = format!("⚠️  Warning: {}", message); }
                    ValidationResult::Error { message } => { self.debug_message = format!("❌ Pattern violation: {}", message); }
                }
            }
        }
        Ok(result?)
    }

    fn delete_backward(&mut self) -> anyhow::Result<()> {
        let result = self.editor.delete_backward();
        if result.is_ok() { self.debug_message = "⌫ Character deleted".to_string(); }
        Ok(result?)
    }

    fn delete_forward(&mut self) -> anyhow::Result<()> {
        let result = self.editor.delete_forward();
        if result.is_ok() { self.debug_message = "⌦ Character deleted".to_string(); }
        Ok(result?)
    }

    // Delegate methods
    fn current_field(&self) -> usize { self.editor.current_field() }
    fn cursor_position(&self) -> usize { self.editor.cursor_position() }
    fn mode(&self) -> AppMode { self.editor.mode() }
    fn current_text(&self) -> &str { self.editor.current_text() }
    fn data_provider(&self) -> &D { self.editor.data_provider() }
    fn ui_state(&self) -> &canvas::EditorState { self.editor.ui_state() }
    fn set_mode(&mut self, mode: AppMode) { self.editor.set_mode(mode); }

    fn next_field(&mut self) {
        match self.editor.next_field() {
            Ok(()) => { self.update_field_validation_status(); self.field_switch_blocked = false; self.block_reason = None; }
            Err(e) => { self.field_switch_blocked = true; self.block_reason = Some(e.to_string()); self.debug_message = format!("🚫 Cannot move to next field: {}", e); }
        }
    }

    fn prev_field(&mut self) {
        match self.editor.prev_field() {
            Ok(()) => { self.update_field_validation_status(); self.field_switch_blocked = false; self.block_reason = None; }
            Err(e) => { self.field_switch_blocked = true; self.block_reason = Some(e.to_string()); self.debug_message = format!("🚫 Cannot move to previous field: {}", e); }
        }
    }

    fn set_debug_message(&mut self, msg: String) { self.debug_message = msg; }
    fn debug_message(&self) -> &str { &self.debug_message }

    fn update_field_validation_status(&mut self) {
        if !self.validation_enabled { return; }
        if let Some(result) = self.editor.current_field_validation() {
            match result {
                ValidationResult::Valid => { self.debug_message = format!("Field {}: ✅ Pattern valid", self.editor.current_field() + 1); }
                ValidationResult::Warning { message } => { self.debug_message = format!("Field {}: ⚠️  {}", self.editor.current_field() + 1, message); }
                ValidationResult::Error { message } => { self.debug_message = format!("Field {}: ❌ {}", self.editor.current_field() + 1, message); }
            }
        }
    }

    fn get_validation_status(&self) -> String {
        if !self.validation_enabled { return "❌ DISABLED".to_string(); }
        if self.field_switch_blocked { return "🚫 SWITCH BLOCKED".to_string(); }
        let summary = self.editor.validation_summary();
        if summary.has_errors() { format!("❌ {} ERRORS", summary.error_fields) }
        else if summary.has_warnings() { format!("⚠️  {} WARNINGS", summary.warning_fields) }
        else if summary.validated_fields > 0 { format!("✅ {} VALID", summary.valid_fields) }
        else { "🔍 READY".to_string() }
    }
}

// Advanced demo form with creative and edge-case-heavy validation patterns
struct AdvancedPatternData {
    fields: Vec<(String, String)>,
}

impl AdvancedPatternData {
    fn new() -> Self {
        Self {
            fields: vec![
                ("🕐 Time (HH:MM) - 24hr format".to_string(), "".to_string()),
                ("🎨 Hex Color (#RRGGBB) - Web colors".to_string(), "".to_string()),
                ("🌐 IPv4 (XXX.XXX.XXX.XXX) - Network address".to_string(), "".to_string()),
                ("🏷️  Product Code (ABC-123-XYZ) - Mixed format".to_string(), "".to_string()),
                ("📅 Date Code (2024W15) - Year + Week".to_string(), "".to_string()),
                ("🔢 Binary (101010) - Only 0s and 1s".to_string(), "".to_string()),
                ("🎯 Complex ID (A1-B2C-3D4E) - Multi-rule".to_string(), "".to_string()),
                ("🚀 Custom Pattern - Advanced logic".to_string(), "".to_string()),
            ],
        }
    }
}

impl DataProvider for AdvancedPatternData {
    fn field_count(&self) -> usize { self.fields.len() }
    fn field_name(&self, index: usize) -> &str { &self.fields[index].0 }
    fn field_value(&self, index: usize) -> &str { &self.fields[index].1 }
    fn set_field_value(&mut self, index: usize, value: String) { self.fields[index].1 = value; }

    fn validation_config(&self, field_index: usize) -> Option<ValidationConfig> {
        match field_index {
            0 => {
                // 🕐 Time (HH:MM) - Hours 00-23, Minutes 00-59
                // This showcases: Multiple position ranges, exact character matching, custom validation
                let time_pattern = PatternFilters::new()
                    .add_filter(PositionFilter::new(
                        PositionRange::Multiple(vec![0, 1, 3, 4]), // Hours and minutes positions
                        CharacterFilter::Numeric,
                    ))
                    .add_filter(PositionFilter::new(
                        PositionRange::Single(2), // Colon separator
                        CharacterFilter::Exact(':'),
                    ));

                Some(ValidationConfigBuilder::new()
                    .with_pattern_filters(time_pattern)
                    .with_max_length(5) // HH:MM = 5 characters
                    .build())
            }
            1 => {
                // 🎨 Hex Color (#RRGGBB) - Web color format
                // This showcases: OneOf filter with hex digits, exact character at start
                let hex_digits = vec!['0','1','2','3','4','5','6','7','8','9','A','B','C','D','E','F','a','b','c','d','e','f'];
                let hex_color_pattern = PatternFilters::new()
                    .add_filter(PositionFilter::new(
                        PositionRange::Single(0), // Hash symbol
                        CharacterFilter::Exact('#'),
                    ))
                    .add_filter(PositionFilter::new(
                        PositionRange::Range(1, 6), // 6 hex digits for RGB
                        CharacterFilter::OneOf(hex_digits),
                    ));

                Some(ValidationConfigBuilder::new()
                    .with_pattern_filters(hex_color_pattern)
                    .with_max_length(7) // #RRGGBB = 7 characters
                    .build())
            }
            2 => {
                // 🌐 IPv4 Address (XXX.XXX.XXX.XXX) - Network address
                // This showcases: Complex pattern with dots at specific positions
                let ipv4_pattern = PatternFilters::new()
                    .add_filter(PositionFilter::new(
                        PositionRange::Multiple(vec![3, 7, 11]), // Dots at specific positions
                        CharacterFilter::Exact('.'),
                    ))
                    .add_filter(PositionFilter::new(
                        PositionRange::Multiple(vec![0,1,2,4,5,6,8,9,10,12,13,14]), // Number positions
                        CharacterFilter::Numeric,
                    ));

                Some(ValidationConfigBuilder::new()
                    .with_pattern_filters(ipv4_pattern)
                    .with_max_length(15) // XXX.XXX.XXX.XXX = up to 15 chars
                    .build())
            }
            3 => {
                // 🏷️ Product Code (ABC-123-XYZ) - Mixed format sections
                // This showcases: Different rules for different sections
                let product_code_pattern = PatternFilters::new()
                    .add_filter(PositionFilter::new(
                        PositionRange::Range(0, 2), // First 3 positions: letters
                        CharacterFilter::Alphabetic,
                    ))
                    .add_filter(PositionFilter::new(
                        PositionRange::Multiple(vec![3, 7]), // Dashes
                        CharacterFilter::Exact('-'),
                    ))
                    .add_filter(PositionFilter::new(
                        PositionRange::Range(4, 6), // Middle 3 positions: numbers
                        CharacterFilter::Numeric,
                    ))
                    .add_filter(PositionFilter::new(
                        PositionRange::Range(8, 10), // Last 3 positions: letters
                        CharacterFilter::Alphabetic,
                    ));

                Some(ValidationConfigBuilder::new()
                    .with_pattern_filters(product_code_pattern)
                    .with_max_length(11) // ABC-123-XYZ = 11 characters
                    .build())
            }
            4 => {
                // 📅 Date Code (2024W15) - Year + Week format
                // This showcases: From position filtering and mixed patterns
                let date_code_pattern = PatternFilters::new()
                    .add_filter(PositionFilter::new(
                        PositionRange::Range(0, 3), // Year: 4 digits
                        CharacterFilter::Numeric,
                    ))
                    .add_filter(PositionFilter::new(
                        PositionRange::Single(4), // Week indicator
                        CharacterFilter::Exact('W'),
                    ))
                    .add_filter(PositionFilter::new(
                        PositionRange::From(5), // Week number: rest are digits
                        CharacterFilter::Numeric,
                    ));

                Some(ValidationConfigBuilder::new()
                    .with_pattern_filters(date_code_pattern)
                    .with_max_length(7) // 2024W15 = 7 characters
                    .build())
            }
            5 => {
                // 🔢 Binary (101010) - Only 0s and 1s
                // This showcases: OneOf filter with limited character set
                let binary_pattern = PatternFilters::new()
                    .add_filter(PositionFilter::new(
                        PositionRange::From(0), // All positions
                        CharacterFilter::OneOf(vec!['0', '1']),
                    ));

                Some(ValidationConfigBuilder::new()
                    .with_pattern_filters(binary_pattern)
                    .with_max_length(16) // Allow up to 16 binary digits
                    .build())
            }
            6 => {
                // 🎯 Complex ID (A1-B2C-3D4E) - Multiple overlapping rules
                // This showcases: Complex overlapping patterns and edge cases
                let complex_id_pattern = PatternFilters::new()
                    .add_filter(PositionFilter::new(
                        PositionRange::Multiple(vec![0, 3, 6, 8]), // Letter positions
                        CharacterFilter::Alphabetic,
                    ))
                    .add_filter(PositionFilter::new(
                        PositionRange::Multiple(vec![1, 4, 7, 9]), // Number positions
                        CharacterFilter::Numeric,
                    ))
                    .add_filter(PositionFilter::new(
                        PositionRange::Multiple(vec![2, 5]), // Dashes
                        CharacterFilter::Exact('-'),
                    ))
                    .add_filter(PositionFilter::new(
                        PositionRange::Single(5), // Special case: override dash with letter C
                        CharacterFilter::Alphabetic, // This creates an interesting edge case
                    ));

                Some(ValidationConfigBuilder::new()
                    .with_pattern_filters(complex_id_pattern)
                    .with_max_length(10) // A1-B2C-3D4E = 10 characters
                    .build())
            }
            7 => {
                // 🚀 Custom Pattern - Advanced logic with custom function
                // This showcases: Custom validation function for complex rules
                let custom_pattern = PatternFilters::new()
                    .add_filter(PositionFilter::new(
                        PositionRange::From(0),
                        CharacterFilter::Custom(Arc::new(|c| {
                            // Advanced rule: Alternating vowels and consonants!
                            // Even positions (0,2,4...): vowels (a,e,i,o,u)
                            // Odd positions (1,3,5...): consonants
                            let vowels = ['a', 'e', 'i', 'o', 'u', 'A', 'E', 'I', 'O', 'U'];
                            
                            // For demo purposes, we'll just accept alphabetic characters
                            // In real usage, you'd implement the alternating logic based on position
                            c.is_alphabetic()
                        })),
                    ));

                Some(ValidationConfigBuilder::new()
                    .with_pattern_filters(custom_pattern)
                    .with_max_length(12) // Allow up to 12 characters
                    .build())
            }
            _ => None,
        }
    }
}

// Key handling (same structure as before)
fn handle_key_press(
    key: KeyCode,
    modifiers: KeyModifiers,
    editor: &mut AdvancedPatternFormEditor<AdvancedPatternData>,
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
        // Mode transitions
        (AppMode::ReadOnly, KeyCode::Char('i'), _) => { editor.enter_edit_mode(); editor.clear_command_buffer(); }
        (AppMode::ReadOnly, KeyCode::Char('a'), _) => { editor.enter_append_mode(); editor.clear_command_buffer(); }
        (AppMode::ReadOnly, KeyCode::Char('A'), _) => { editor.move_line_end(); editor.enter_edit_mode(); editor.clear_command_buffer(); }
        (_, KeyCode::Esc, _) => { if mode == AppMode::Edit { editor.exit_edit_mode(); } else { editor.clear_command_buffer(); } }

        // Validation commands
        (AppMode::ReadOnly, KeyCode::F(1), _) => { editor.toggle_validation(); }

        // Movement in ReadOnly mode
        (AppMode::ReadOnly, KeyCode::Char('h'), _) | (AppMode::ReadOnly, KeyCode::Left, _) => { editor.move_left(); editor.clear_command_buffer(); }
        (AppMode::ReadOnly, KeyCode::Char('l'), _) | (AppMode::ReadOnly, KeyCode::Right, _) => { editor.move_right(); editor.clear_command_buffer(); }
        (AppMode::ReadOnly, KeyCode::Char('j'), _) | (AppMode::ReadOnly, KeyCode::Down, _) => { editor.move_down(); editor.clear_command_buffer(); }
        (AppMode::ReadOnly, KeyCode::Char('k'), _) | (AppMode::ReadOnly, KeyCode::Up, _) => { editor.move_up(); editor.clear_command_buffer(); }

        // Movement in Edit mode
        (AppMode::Edit, KeyCode::Left, _) => { editor.move_left(); }
        (AppMode::Edit, KeyCode::Right, _) => { editor.move_right(); }
        (AppMode::Edit, KeyCode::Up, _) => { editor.move_up(); }
        (AppMode::Edit, KeyCode::Down, _) => { editor.move_down(); }

        // Delete operations
        (AppMode::Edit, KeyCode::Backspace, _) => { editor.delete_backward()?; }
        (AppMode::Edit, KeyCode::Delete, _) => { editor.delete_forward()?; }

        // Tab navigation
        (_, KeyCode::Tab, _) => { editor.next_field(); }
        (_, KeyCode::BackTab, _) => { editor.prev_field(); }

        // Character input
        (AppMode::Edit, KeyCode::Char(c), m) if !m.contains(KeyModifiers::CONTROL) => {
            editor.insert_char(c)?;
        }

        // Debug info
        (AppMode::ReadOnly, KeyCode::Char('?'), _) => {
            let summary = editor.editor.validation_summary();
            editor.set_debug_message(format!(
                "Field {}/{}, Pos {}, Mode: {:?}, Advanced patterns: {} configured",
                editor.current_field() + 1,
                editor.data_provider().field_count(),
                editor.cursor_position(),
                editor.mode(),
                summary.total_fields
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
    mut editor: AdvancedPatternFormEditor<AdvancedPatternData>,
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

fn ui(f: &mut Frame, editor: &AdvancedPatternFormEditor<AdvancedPatternData>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(15)])
        .split(f.area());

    render_canvas_default(f, chunks[0], &editor.editor);
    render_advanced_validation_status(f, chunks[1], editor);
}

fn render_advanced_validation_status(
    f: &mut Frame,
    area: Rect,
    editor: &AdvancedPatternFormEditor<AdvancedPatternData>,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Status bar
            Constraint::Length(5), // Validation summary
            Constraint::Length(7), // Help
        ])
        .split(area);

    // Status bar
    let mode_text = match editor.mode() {
        AppMode::Edit => "INSERT",
        AppMode::ReadOnly => "NORMAL",
        _ => "OTHER",
    };

    let validation_status = editor.get_validation_status();
    let status_text = format!("-- {} -- {} | Advanced Patterns: {}", mode_text, editor.debug_message(), validation_status);

    let status = Paragraph::new(Line::from(Span::raw(status_text)))
        .block(Block::default().borders(Borders::ALL).title("🚀 Advanced Pattern Validation"));

    f.render_widget(status, chunks[0]);

    // Enhanced validation summary
    let summary = editor.editor.validation_summary();
    let field_info = match editor.current_field() {
        0 => "Time format (HH:MM) - Tests exact chars + numeric ranges",
        1 => "Hex color (#RRGGBB) - Tests OneOf filter with case insensitive",
        2 => "IPv4 address - Tests complex dot positioning",
        3 => "Product code (ABC-123-XYZ) - Tests section-based patterns",
        4 => "Date code (2024W15) - Tests From position filtering",
        5 => "Binary input - Tests limited character set (0,1 only)",
        6 => "Complex ID - Tests overlapping/conflicting rules",
        7 => "Custom pattern - Tests advanced custom validation logic",
        _ => "Unknown field",
    };

    let summary_text = if editor.validation_enabled {
        format!(
            "📊 Advanced Pattern Summary: {} fields with complex rules\n\
             Current Field: {}\n\
             ✅ Valid: {}  ⚠️  Warnings: {}  ❌ Errors: {}  📈 Progress: {:.0}%\n\
             🎯 Pattern Focus: {}",
            summary.total_fields,
            editor.current_field() + 1,
            summary.valid_fields,
            summary.warning_fields,
            summary.error_fields,
            summary.completion_percentage() * 100.0,
            field_info
        )
    } else {
        "❌ Advanced pattern validation is DISABLED\nPress F1 to enable and see the magic!".to_string()
    };

    let summary_style = if summary.has_errors() {
        Style::default().fg(Color::Red)
    } else if summary.has_warnings() {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Green)
    };

    let validation_summary = Paragraph::new(summary_text)
        .block(Block::default().borders(Borders::ALL).title("🎯 Advanced Pattern Analysis"))
        .style(summary_style)
        .wrap(Wrap { trim: true });

    f.render_widget(validation_summary, chunks[1]);

    // Enhanced help text
    let help_text = match editor.mode() {
        AppMode::ReadOnly => {
            "🚀 ADVANCED PATTERN SHOWCASE - Each field demonstrates different edge cases!\n\
             🕐 Time: Numeric+exact chars  🎨 Hex: OneOf with case-insensitive  🌐 IPv4: Complex positioning\n\
             🏷️  Product: Multi-section rules  📅 Date: From-position filtering  🔢 Binary: Limited charset\n\
             🎯 Complex: Overlapping rules  🚀 Custom: Advanced logic functions\n\
             \n\
             Movement: hjkl/arrows=move, Tab/Shift+Tab=fields, i/a=insert, F1=toggle, ?=info"
        }
        AppMode::Edit => {
            "✏️  INSERT MODE - Testing advanced pattern validation!\n\
             Each character is validated against complex rules in real-time\n\
             Try entering invalid characters to see detailed error messages\n\
             arrows=move, Backspace/Del=delete, Esc=normal, Tab=next field"
        }
        _ => "🚀 Advanced Pattern Validation Active!"
    };

    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("🎯 Advanced Pattern Commands & Info"))
        .style(Style::default().fg(Color::Gray))
        .wrap(Wrap { trim: true });

    f.render_widget(help, chunks[2]);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Canvas Advanced Pattern Validation Demo");
    println!("✅ validation feature: ENABLED");
    println!("✅ gui feature: ENABLED");
    println!("🎯 Advanced pattern filtering: ACTIVE");
    println!("🧪 Edge cases and complex patterns: READY");
    println!("💡 Each field showcases different validation capabilities!");
    println!();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let data = AdvancedPatternData::new();
    let editor = AdvancedPatternFormEditor::new(data);

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

    println!("🚀 Advanced pattern validation demo completed!");
    println!("🎯 Hope you enjoyed seeing all the edge cases in action!");
    Ok(())
}
