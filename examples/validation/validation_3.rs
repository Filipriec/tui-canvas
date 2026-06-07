// examples/validation_3.rs
//! Display mask demo.
//!
//! Shows dynamic and template display modes, custom patterns, custom input
//! characters, placeholder characters, and cursor movement through formatted
//! displays.
//!
//! Mask input positions must match the configured character limit.
//!
//! Run with: cargo run --example validation_3 --features "gui,validation,cursor-style"

// REQUIRE validation, gui and cursor-style features for mask functionality
#[cfg(not(all(feature = "validation", feature = "gui", feature = "cursor-style")))]
compile_error!(
    "This example requires the 'validation', 'gui' and 'cursor-style' features. \
     Run with: cargo run --example validation_3 --features \"gui,validation,cursor-style\""
);

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;

use tui_canvas::{
    render_canvas_default, AppMode, CursorManager, DataProvider, DisplayMask,
    FormEditor, ValidationConfig, ValidationConfigBuilder,
};

// FormEditor wrapper for mask demo
struct MaskDemoFormEditor<D: DataProvider> {
    editor: FormEditor<D>,
    debug_message: String,
    command_buffer: String,
    validation_enabled: bool,
    show_raw_data: bool,
}

impl<D: DataProvider> MaskDemoFormEditor<D> {
    fn new(data_provider: D) -> Self {
        let mut editor = FormEditor::new(data_provider);
        editor.set_validation_enabled(true);

        Self {
            editor,
            debug_message: "🎭 Display Mask Demo - Visual formatting with clean business logic!"
                .to_string(),
            command_buffer: String::new(),
            validation_enabled: true,
            show_raw_data: false,
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

    // Mask control
    fn toggle_validation(&mut self) {
        self.validation_enabled = !self.validation_enabled;
        self.editor.set_validation_enabled(self.validation_enabled);
        if self.validation_enabled {
            self.debug_message =
                "✅ Display Masks ENABLED - See visual formatting in action!".to_string();
        } else {
            self.debug_message = "❌ Display Masks DISABLED - Raw text only".to_string();
        }
    }

    fn toggle_raw_data_view(&mut self) {
        self.show_raw_data = !self.show_raw_data;
        if self.show_raw_data {
            self.debug_message =
                "👁️ Showing RAW business data (what's actually stored)".to_string();
        } else {
            self.debug_message = "🎭 Showing FORMATTED display (what users see)".to_string();
        }
    }

    fn get_current_field_info(&self) -> (String, String, String) {
        let field_index = self.editor.current_field();
        let raw_data = self.editor.data_provider().field_value(field_index);
        let display_data = if self.validation_enabled {
            self.editor.current_display_text()
        } else {
            raw_data.to_string()
        };

        let mask_info =
            if let Some(config) = self.editor.validation_state().get_field_config(field_index) {
                if let Some(mask) = &config.display_mask {
                    format!(
                        "Pattern: '{}', Mode: {:?}",
                        mask.pattern(),
                        mask.display_mode()
                    )
                } else {
                    "No mask configured".to_string()
                }
            } else {
                "No validation config".to_string()
            };

        (raw_data.to_string(), display_data, mask_info)
    }

    // Movement with mask awareness
    fn move_left(&mut self) {
        self.editor.move_left();
        self.update_cursor_info();
    }

    fn move_right(&mut self) {
        self.editor.move_right();
        self.update_cursor_info();
    }

    fn move_up(&mut self) {
        if self.editor.move_up() {
            self.update_field_info();
        } else if let Some(reason) = self.editor.field_switch_block_reason() {
            self.debug_message = format!("🚫 Field switch blocked: {reason}");
        }
    }

    fn move_down(&mut self) {
        if self.editor.move_down() {
            self.update_field_info();
        } else if let Some(reason) = self.editor.field_switch_block_reason() {
            self.debug_message = format!("🚫 Field switch blocked: {reason}");
        }
    }

    fn move_line_start(&mut self) {
        self.editor.move_line_start();
        self.update_cursor_info();
    }

    fn move_line_end(&mut self) {
        self.editor.move_line_end();
        self.update_cursor_info();
    }

    fn update_cursor_info(&mut self) {
        if self.validation_enabled {
            let raw_pos = self.editor.cursor_position();
            let display_pos = self.editor.display_cursor_position();
            if raw_pos != display_pos {
                self.debug_message = format!(
                    "📍 Cursor: Raw pos {raw_pos} → Display pos {display_pos} (mask active)"
                );
            } else {
                self.debug_message = format!("📍 Cursor at position {raw_pos} (no mask offset)");
            }
        }
    }

    fn update_field_info(&mut self) {
        let field_name = self
            .editor
            .data_provider()
            .field_name(self.editor.current_field());
        self.debug_message = format!("📝 Switched to: {field_name}");
    }

    // Mode transitions
    fn enter_edit_mode(&mut self) {
        self.editor.enter_edit_mode();
        self.debug_message =
            "✏️ INSERT MODE - Cursor: Steady Bar | - Type to see mask formatting in real-time"
                .to_string();
    }

    fn enter_append_mode(&mut self) {
        self.editor.enter_append_mode();
        self.debug_message =
            "✏️ INSERT (append) - Cursor: Steady Bar | - Mask formatting active".to_string();
    }

    fn exit_edit_mode(&mut self) {
        self.editor.exit_edit_mode();
        self.debug_message = "🔒 NORMAL MODE - Cursor: Steady Block █ - Press 'r' to see raw data, 'm' for mask info".to_string();
    }

    fn insert_char(&mut self, ch: char) -> anyhow::Result<()> {
        let result = self.editor.insert_char(ch);
        if result.is_ok() {
            let (raw, display, _) = self.get_current_field_info();
            if raw != display {
                self.debug_message = format!("✏️ Added '{ch}': Raw='{raw}' Display='{display}'");
            } else {
                self.debug_message = format!("✏️ Added '{ch}': '{raw}'");
            }
        }
        result
    }

    // Delete operations
    fn delete_backward(&mut self) -> anyhow::Result<()> {
        let result = self.editor.delete_backward();
        if result.is_ok() {
            self.debug_message = "⌫ Character deleted".to_string();
            self.update_cursor_info();
        }
        result
    }

    fn delete_forward(&mut self) -> anyhow::Result<()> {
        let result = self.editor.delete_forward();
        if result.is_ok() {
            self.debug_message = "⌦ Character deleted".to_string();
            self.update_cursor_info();
        }
        result
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
        // Library automatically updates cursor for the mode
        self.editor.set_mode(mode);
    }

    fn next_field(&mut self) {
        if self.editor.next_field() {
            self.update_field_info();
        } else if let Some(reason) = self.editor.field_switch_block_reason() {
            self.debug_message = format!("🚫 Cannot move to next field: {reason}");
        }
    }

    fn prev_field(&mut self) {
        if self.editor.prev_field() {
            self.update_field_info();
        } else if let Some(reason) = self.editor.field_switch_block_reason() {
            self.debug_message = format!("🚫 Cannot move to previous field: {reason}");
        }
    }

    // Status and debug
    fn set_debug_message(&mut self, msg: String) {
        self.debug_message = msg;
    }
    fn debug_message(&self) -> &str {
        &self.debug_message
    }

    fn show_mask_details(&mut self) {
        let (raw, display, mask_info) = self.get_current_field_info();
        self.debug_message = format!(
            "🔍 Field {}: {} | Raw: '{}' Display: '{}'",
            self.current_field() + 1,
            mask_info,
            raw,
            display
        );
    }

    fn get_mask_status(&self) -> String {
        if !self.validation_enabled {
            return "❌ DISABLED".to_string();
        }

        let field_count = self.editor.data_provider().field_count();
        let mut mask_count = 0;
        for i in 0..field_count {
            if let Some(config) = self.editor.validation_state().get_field_config(i) {
                if config.display_mask.is_some() {
                    mask_count += 1;
                }
            }
        }

        format!("🎭 {mask_count} MASKS")
    }
}

// Demo data with mask examples
struct MaskDemoData {
    fields: Vec<(String, String)>,
}

impl MaskDemoData {
    fn new() -> Self {
        Self {
            fields: vec![
                ("📞 Phone (Dynamic)".to_string(), "".to_string()),
                ("📞 Phone (Template)".to_string(), "".to_string()),
                ("📅 Date US (MM/DD/YYYY)".to_string(), "".to_string()),
                ("📅 Date EU (DD.MM.YYYY)".to_string(), "".to_string()),
                ("📅 Date ISO (YYYY-MM-DD)".to_string(), "".to_string()),
                ("🏛️ SSN (XXX-XX-XXXX)".to_string(), "".to_string()),
                ("💳 Credit Card".to_string(), "".to_string()),
                ("🏢 Employee ID (EMP-####)".to_string(), "".to_string()),
                ("📦 Product Code (ABC###XYZ)".to_string(), "".to_string()),
                ("🌈 Custom Separators".to_string(), "".to_string()),
                ("⭐ Custom Placeholders".to_string(), "".to_string()),
                ("🎯 Mixed Input Chars".to_string(), "".to_string()),
            ],
        }
    }
}

impl DataProvider for MaskDemoData {
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

    fn validation_config(&self, field_index: usize) -> Option<ValidationConfig> {
        match field_index {
            0 => {
                // Phone dynamic
                let phone_mask = DisplayMask::new("(###) ###-####", '#');
                Some(
                    ValidationConfigBuilder::new()
                        .with_display_mask(phone_mask)
                        .with_max_length(10) // Matches 10 input positions
                        .build(),
                )
            }
            1 => {
                // Phone template
                let phone_template = DisplayMask::new("(###) ###-####", '#').with_template('_');
                Some(
                    ValidationConfigBuilder::new()
                        .with_display_mask(phone_template)
                        .with_max_length(10) // Matches 10 input positions
                        .build(),
                )
            }
            2 => {
                // 📅 Date US (MM/DD/YYYY) - American date format
                let us_date = DisplayMask::new("##/##/####", '#');
                Some(ValidationConfig::with_mask(us_date))
            }
            3 => {
                // 📅 Date EU (DD.MM.YYYY) - European date format with dots
                let eu_date = DisplayMask::new("##.##.####", '#').with_template('•');
                Some(ValidationConfig::with_mask(eu_date))
            }
            4 => {
                // 📅 Date ISO (YYYY-MM-DD) - ISO date format
                let iso_date = DisplayMask::new("####-##-##", '#').with_template('-');
                Some(ValidationConfig::with_mask(iso_date))
            }
            5 => {
                // SSN with custom input character
                let ssn_mask = DisplayMask::new("XXX-XX-XXXX", 'X');
                Some(
                    ValidationConfigBuilder::new()
                        .with_display_mask(ssn_mask)
                        .with_max_length(9) // Matches 9 input positions
                        .build(),
                )
            }
            6 => {
                // Credit card
                let cc_mask = DisplayMask::new("#### #### #### ####", '#').with_template('•');
                Some(
                    ValidationConfigBuilder::new()
                        .with_display_mask(cc_mask)
                        .with_max_length(16) // Matches 16 input positions
                        .build(),
                )
            }
            7 => {
                // 🏢 Employee ID with business prefix
                let emp_id = DisplayMask::new("EMP-####", '#');
                Some(ValidationConfig::with_mask(emp_id))
            }
            8 => {
                // 📦 Product Code with mixed letters and numbers
                let product_code = DisplayMask::new("ABC###XYZ", '#');
                Some(ValidationConfig::with_mask(product_code))
            }
            9 => {
                // 🌈 Custom Separators - Using | and ~ as separators
                let custom_sep = DisplayMask::new("##|##~####", '#').with_template('?');
                Some(ValidationConfig::with_mask(custom_sep))
            }
            10 => {
                // ⭐ Custom Placeholders - Using different placeholder characters
                let custom_placeholder = DisplayMask::new("##-##-##", '#').with_template('★');
                Some(ValidationConfig::with_mask(custom_placeholder))
            }
            11 => {
                // 🎯 Mixed Input Characters - Using 'N' for numbers
                let mixed_input = DisplayMask::new("ID:NNN-NNN", 'N');
                Some(ValidationConfig::with_mask(mixed_input))
            }
            _ => None,
        }
    }
}

// Key handling with mask specific commands
fn handle_key_press(
    key: KeyCode,
    modifiers: KeyModifiers,
    editor: &mut MaskDemoFormEditor<MaskDemoData>,
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
        (AppMode::Nor, KeyCode::Char('i'), _) => {
            editor.enter_edit_mode();
            editor.clear_command_buffer();
        }
        (AppMode::Nor, KeyCode::Char('a'), _) => {
            editor.enter_append_mode();
            editor.clear_command_buffer();
        }
        (AppMode::Nor, KeyCode::Char('A'), _) => {
            editor.move_line_end();
            editor.enter_edit_mode();
            editor.clear_command_buffer();
        }

        // Escape: Exit edit mode
        (_, KeyCode::Esc, _) => {
            if mode == AppMode::Ins {
                editor.exit_edit_mode();
            } else {
                editor.clear_command_buffer();
            }
        }

        // Mask specific commands
        (AppMode::Nor, KeyCode::Char('m'), _) => {
            editor.show_mask_details();
            editor.clear_command_buffer();
        }
        (AppMode::Nor, KeyCode::Char('r'), _) => {
            editor.toggle_raw_data_view();
            editor.clear_command_buffer();
        }
        (AppMode::Nor, KeyCode::F(1), _) => {
            editor.toggle_validation();
        }

        // Movement
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

        // Line movement
        (AppMode::Nor, KeyCode::Char('0'), _) => {
            editor.move_line_start();
            editor.clear_command_buffer();
        }
        (AppMode::Nor, KeyCode::Char('$'), _) => {
            editor.move_line_end();
            editor.clear_command_buffer();
        }

        // Edit mode movement
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

        // Delete operations
        (AppMode::Ins, KeyCode::Backspace, _) => {
            editor.delete_backward()?;
        }
        (AppMode::Ins, KeyCode::Delete, _) => {
            editor.delete_forward()?;
        }

        // Tab navigation
        (_, KeyCode::Tab, _) => {
            editor.next_field();
        }
        (_, KeyCode::BackTab, _) => {
            editor.prev_field();
        }

        // Character input
        (AppMode::Ins, KeyCode::Char(c), m) if !m.contains(KeyModifiers::CONTROL) => {
            editor.insert_char(c)?;
        }

        // Debug info commands
        (AppMode::Nor, KeyCode::Char('?'), _) => {
            let (raw, display, mask_info) = editor.get_current_field_info();
            editor.set_debug_message(format!(
                "Field {}/{}, Cursor {}, {}, Raw: '{}', Display: '{}'",
                editor.current_field() + 1,
                editor.data_provider().field_count(),
                editor.cursor_position(),
                mask_info,
                raw,
                display
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
    mut editor: MaskDemoFormEditor<MaskDemoData>,
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

fn ui(f: &mut Frame, editor: &MaskDemoFormEditor<MaskDemoData>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(16)])
        .split(f.area());

    render_enhanced_canvas(f, chunks[0], editor);
    render_mask_status(f, chunks[1], editor);
}

fn render_enhanced_canvas(f: &mut Frame, area: Rect, editor: &MaskDemoFormEditor<MaskDemoData>) {
    render_canvas_default(f, area, &editor.editor);
}

fn render_mask_status(f: &mut Frame, area: Rect, editor: &MaskDemoFormEditor<MaskDemoData>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Status bar
            Constraint::Length(6), // Data comparison
            Constraint::Length(7), // Help
        ])
        .split(area);

    // Status bar with mask information
    let mode_text = match editor.mode() {
        AppMode::Ins => "INSERT | (bar cursor)",
        AppMode::Nor => "NORMAL █ (block cursor)",
        _ => "NORMAL █ (block cursor)",
    };

    let mask_status = editor.get_mask_status();
    let status_text = format!(
        "-- {} -- {} | Masks: {} | View: {}",
        mode_text,
        editor.debug_message(),
        mask_status,
        if editor.show_raw_data {
            "RAW"
        } else {
            "FORMATTED"
        }
    );

    let status = Paragraph::new(Line::from(Span::raw(status_text))).block(
        Block::default()
            .borders(Borders::ALL)
            .title("🎭 Display Mask Demo"),
    );

    f.render_widget(status, chunks[0]);

    // Data comparison showing raw vs display
    let (raw_data, display_data, mask_info) = editor.get_current_field_info();
    let field_name = editor.data_provider().field_name(editor.current_field());

    let comparison_text = format!(
        "📝 Current Field: {}\n\
         🔧 Mask Config: {}\n\
         \n\
         💾 Raw Business Data: '{}' ← What's actually stored in your database\n\
         🎭 Formatted Display: '{}' ← What users see in the interface\n\
         📍 Cursor: Raw pos {} → Display pos {}",
        field_name,
        mask_info,
        raw_data,
        display_data,
        editor.cursor_position(),
        editor.editor.display_cursor_position()
    );

    let comparison_style = if raw_data != display_data {
        Style::default().fg(Color::Green) // Green when mask is active
    } else {
        Style::default().fg(Color::Gray) // Gray when no formatting
    };

    let data_comparison = Paragraph::new(comparison_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("📊 Raw Data vs Display Formatting"),
        )
        .style(comparison_style)
        .wrap(Wrap { trim: true });

    f.render_widget(data_comparison, chunks[1]);

    // Help text
    let help_text = match editor.mode() {
        AppMode::Nor => {
            "🎯 CURSOR-STYLE: Normal █ | Insert |\n\
             🎭 MASK DEMO: Visual formatting keeps business logic clean!\n\
             \n\
             📱 Try different fields to see various mask patterns:\n\
             • Dynamic vs Template modes  • Custom separators  • Different input chars\n\
             \n\
             Commands: i/a=insert, m=mask details, r=toggle raw/display view\n\
             Movement: hjkl/arrows=move, 0/$=line start/end, Tab=next field, F1=toggle masks\n\
             ?=detailed info, Ctrl+C=quit"
        }
        AppMode::Ins => {
            "🎯 INSERT MODE - Cursor: | (bar)\n\
             ✏️ Type to see real-time mask formatting!\n\
             \n\
             🔥 Key Features in Action:\n\
             • Separators auto-appear as you type  • Cursor skips over separators\n\
             • Template fields show placeholders  • Raw data stays clean for business logic\n\
             \n\
             arrows=move through mask, Backspace/Del=delete, Esc=normal, Tab=next field\n\
             Notice how cursor position maps between raw data and display!"
        }
        _ => "🎭 Display Mask Demo Active!",
    };

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("🚀 Mask Features & Commands"),
        )
        .style(Style::default().fg(Color::Gray))
        .wrap(Wrap { trim: true });

    f.render_widget(help, chunks[2]);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Print feature status
    println!("🎭 Canvas Display Mask Demo (Feature 3)");
    println!("✅ validation feature: ENABLED");
    println!("✅ gui feature: ENABLED");
    println!("🎭 Display masks: ACTIVE");
    println!("✅ cursor-style feature: ENABLED");
    println!("🔥 Key Benefits Demonstrated:");
    println!("   • Clean separation: Visual formatting ≠ Business logic");
    println!("   • User-friendly: Pretty displays with automatic cursor handling");
    println!("   • Flexible: Custom patterns, separators, and placeholders");
    println!("   • Transparent: Library handles all complexity, API stays simple");
    println!();
    println!("💡 Try typing in different fields to see mask magic!");
    println!("   📞 Phone fields show dynamic vs template modes");
    println!("   📅 Date fields show different regional formats");
    println!("   💳 Credit card shows spaced formatting");
    println!("   ⭐ Custom fields show advanced separator/placeholder options");
    println!();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let data = MaskDemoData::new();
    let mut editor = MaskDemoFormEditor::new(data);

    // Initialize with normal mode - library automatically sets block cursor
    editor.set_mode(AppMode::Nor);

    CursorManager::update_for_mode(AppMode::Nor)?;

    let res = run_app(&mut terminal, editor);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    CursorManager::reset()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    println!("🎭 Display mask demo completed!");
    println!("🏆 You've seen how masks provide beautiful UX while keeping business logic clean!");
    Ok(())
}
