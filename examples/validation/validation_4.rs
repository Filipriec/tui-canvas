/* examples/validation_4.rs
   Feature 4 demo: multiple custom formatters with edge cases

   Demonstrates:
   - Multiple formatter types: PSC, Phone, Credit Card, Date
   - Edge case handling: incomplete input, invalid chars, overflow
   - Real-time validation feedback and format preview
   - Advanced cursor position mapping
   - Raw vs formatted data separation
   - Error handling and fallback behavior

*/

#![allow(clippy::needless_return)]

#[cfg(not(all(feature = "validation", feature = "gui", feature = "cursor-style")))]
compile_error!(
    "This example requires the 'validation', 'gui' and 'cursor-style' features. \
     Run with: cargo run --example validation_4 --features \"gui,validation,cursor-style\""
);

use std::io;
use std::sync::Arc;

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

use tui_canvas::{
    render_canvas_default, AppMode, CursorManager, CustomFormatter, DataProvider,
    TextForm, FormattingResult, ValidationConfig, ValidationConfigBuilder,
};

/// PSC (Postal Code) Formatter: "01001" -> "010 01"
struct PSCFormatter;

impl CustomFormatter for PSCFormatter {
    fn format(&self, raw: &str) -> FormattingResult {
        if raw.is_empty() {
            return FormattingResult::success("");
        }

        // Validate: only digits allowed
        if !raw.chars().all(|c| c.is_ascii_digit()) {
            return FormattingResult::error("PSC must contain only digits");
        }

        let len = raw.chars().count();
        match len {
            0 => FormattingResult::success(""),
            1..=3 => FormattingResult::success(raw),
            4 => {
                FormattingResult::warning(format!("{} ", &raw[..3]), "PSC incomplete (4/5 digits)")
            }
            5 => {
                let formatted = format!("{} {}", &raw[..3], &raw[3..]);
                if raw == "00000" {
                    FormattingResult::warning(formatted, "Invalid PSC: 00000")
                } else {
                    FormattingResult::success(formatted)
                }
            }
            _ => FormattingResult::error("PSC too long (max 5 digits)"),
        }
    }
}

/// Phone Number Formatter: "1234567890" -> "(123) 456-7890"
struct PhoneFormatter;

impl CustomFormatter for PhoneFormatter {
    fn format(&self, raw: &str) -> FormattingResult {
        if raw.is_empty() {
            return FormattingResult::success("");
        }

        // Only digits allowed
        if !raw.chars().all(|c| c.is_ascii_digit()) {
            return FormattingResult::error("Phone must contain only digits");
        }

        let len = raw.chars().count();
        match len {
            0 => FormattingResult::success(""),
            1..=3 => FormattingResult::success(format!("({raw})")),
            4..=6 => FormattingResult::success(format!("({}) {}", &raw[..3], &raw[3..])),
            7..=10 => {
                FormattingResult::success(format!("({}) {}-{}", &raw[..3], &raw[3..6], &raw[6..]))
            }
            10 => {
                let formatted = format!("({}) {}-{}", &raw[..3], &raw[3..6], &raw[6..]);
                FormattingResult::success(formatted)
            }
            _ => FormattingResult::warning(
                format!("({}) {}-{}", &raw[..3], &raw[3..6], &raw[6..10]),
                "Phone too long (extra digits ignored)",
            ),
        }
    }
}

/// Credit Card Formatter: "1234567890123456" -> "1234 5678 9012 3456"
struct CreditCardFormatter;

impl CustomFormatter for CreditCardFormatter {
    fn format(&self, raw: &str) -> FormattingResult {
        if raw.is_empty() {
            return FormattingResult::success("");
        }

        if !raw.chars().all(|c| c.is_ascii_digit()) {
            return FormattingResult::error("Card number must contain only digits");
        }

        let mut formatted = String::new();
        for (i, ch) in raw.chars().enumerate() {
            if i > 0 && i % 4 == 0 {
                formatted.push(' ');
            }
            formatted.push(ch);
        }

        let len = raw.chars().count();
        match len {
            0..=15 => {
                FormattingResult::warning(formatted, format!("Card incomplete ({len}/16 digits)"))
            }
            16 => FormattingResult::success(formatted),
            _ => FormattingResult::warning(formatted, "Card too long (extra digits shown)"),
        }
    }
}

/// Date Formatter: "12012024" -> "12/01/2024"
struct DateFormatter;

impl CustomFormatter for DateFormatter {
    fn format(&self, raw: &str) -> FormattingResult {
        if raw.is_empty() {
            return FormattingResult::success("");
        }

        if !raw.chars().all(|c| c.is_ascii_digit()) {
            return FormattingResult::error("Date must contain only digits");
        }

        let len = raw.len();
        match len {
            0 => FormattingResult::success(""),
            1..=2 => FormattingResult::success(raw.to_string()),
            3..=4 => FormattingResult::success(format!("{}/{}", &raw[..2], &raw[2..])),
            5..=8 => {
                FormattingResult::success(format!("{}/{}/{}", &raw[..2], &raw[2..4], &raw[4..]))
            }
            8 => {
                let month = &raw[..2];
                let day = &raw[2..4];
                let year = &raw[4..];

                // Basic validation
                let m: u32 = month.parse().unwrap_or(0);
                let d: u32 = day.parse().unwrap_or(0);

                if m == 0 || m > 12 {
                    FormattingResult::warning(
                        format!("{month}/{day}/{year}"),
                        "Invalid month (01-12)",
                    )
                } else if d == 0 || d > 31 {
                    FormattingResult::warning(
                        format!("{month}/{day}/{year}"),
                        "Invalid day (01-31)",
                    )
                } else {
                    FormattingResult::success(format!("{month}/{day}/{year}"))
                }
            }
            _ => FormattingResult::error("Date too long (MMDDYYYY format)"),
        }
    }
}

// Demo data with multiple formatter types
struct MultiFormatterDemoData {
    fields: Vec<(String, String)>,
}

impl MultiFormatterDemoData {
    fn new() -> Self {
        Self {
            fields: vec![
                ("🏁 PSC (01001)".to_string(), "".to_string()),
                ("📞 Phone (1234567890)".to_string(), "".to_string()),
                ("💳 Credit Card (16 digits)".to_string(), "".to_string()),
                ("📅 Date (12012024)".to_string(), "".to_string()),
                ("📝 Plain Text".to_string(), "".to_string()),
            ],
        }
    }
}

impl DataProvider for MultiFormatterDemoData {
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

    #[cfg(feature = "validation")]
    fn validation_config(&self, field_index: usize) -> Option<ValidationConfig> {
        match field_index {
            0 => Some(
                ValidationConfigBuilder::new()
                    .with_custom_formatter(Arc::new(PSCFormatter))
                    .with_max_length(5)
                    .build(),
            ),
            1 => Some(
                ValidationConfigBuilder::new()
                    .with_custom_formatter(Arc::new(PhoneFormatter))
                    .with_max_length(12)
                    .build(),
            ),
            2 => Some(
                ValidationConfigBuilder::new()
                    .with_custom_formatter(Arc::new(CreditCardFormatter))
                    .with_max_length(20)
                    .build(),
            ),
            3 => Some(
                ValidationConfigBuilder::new()
                    .with_custom_formatter(Arc::new(DateFormatter))
                    .with_max_length(8)
                    .build(),
            ),
            4 => Some(
                ValidationConfigBuilder::new()
                    .with_custom_formatter(Arc::new(DateFormatter))
                    .with_max_length(8)
                    .build(),
            ),
            _ => None, // Plain text field - no formatter
        }
    }
}

// Demo editor with status tracking
struct EnhancedDemoEditor<D: DataProvider> {
    editor: TextForm<D>,
    debug_message: String,
    validation_enabled: bool,
    show_raw_data: bool,
    show_cursor_details: bool,
    example_mode: usize,
}

impl<D: DataProvider> EnhancedDemoEditor<D> {
    fn new(data_provider: D) -> Self {
        let mut editor = TextForm::new(data_provider);
        editor.set_validation_enabled(true);

        Self {
            editor,
            debug_message:
                "🧩 Enhanced Custom Formatter Demo - Multiple formatters with rich edge cases!"
                    .to_string(),
            validation_enabled: true,
            show_raw_data: false,
            show_cursor_details: false,
            example_mode: 0,
        }
    }

    // Field type detection
    fn current_field_type(&self) -> &'static str {
        match self.editor.current_field() {
            0 => "PSC",
            1 => "Phone",
            2 => "Credit Card",
            3 => "Date",
            _ => "Plain Text",
        }
    }

    fn has_formatter(&self) -> bool {
        self.editor.current_field() < 5 // First 5 fields have formatters
    }

    fn get_input_rules(&self) -> &'static str {
        match self.editor.current_field() {
            0 => "5 digits only (PSC format)",
            1 => "10+ digits (US phone format)",
            2 => "16+ digits (credit card)",
            3 => "Digits as cents (12345 = $123.45)",
            4 => "8 digits MMDDYYYY (date format)",
            _ => "Any text (no formatting)",
        }
    }

    fn cycle_example_data(&mut self) {
        let examples = [
            // PSC examples
            vec![
                "01001",
                "1234567890",
                "1234567890123456",
                "12345",
                "12012024",
                "Plain text here",
            ],
            // Incomplete examples
            vec!["010", "123", "1234", "123", "1201", "More text"],
            // Invalid examples (will show error handling)
            vec![
                "0abc1",
                "12a45",
                "123abc",
                "abc",
                "ab01cd",
                "Special chars!",
            ],
            // Edge cases
            vec![
                "00000",
                "0000000000",
                "0000000000000000",
                "99",
                "13012024",
                "",
            ],
        ];

        self.example_mode = (self.example_mode + 1) % examples.len();
        let current_examples = &examples[self.example_mode];

        for (i, example) in current_examples.iter().enumerate() {
            if i < self.editor.data_provider().field_count() {
                self.editor
                    .data_provider_mut()
                    .set_field_value(i, example.to_string());
            }
        }

        let mode_names = [
            "Valid Examples",
            "Incomplete Input",
            "Invalid Characters",
            "Edge Cases",
        ];
        self.debug_message = format!("📋 Loaded: {}", mode_names[self.example_mode]);
    }

    // Status methods
    fn toggle_validation(&mut self) {
        self.validation_enabled = !self.validation_enabled;
        self.editor.set_validation_enabled(self.validation_enabled);
        self.debug_message = if self.validation_enabled {
            "✅ Custom Formatters ENABLED".to_string()
        } else {
            "❌ Custom Formatters DISABLED".to_string()
        };
    }

    fn toggle_raw_data_view(&mut self) {
        self.show_raw_data = !self.show_raw_data;
        self.debug_message = if self.show_raw_data {
            "👁️ Showing RAW data focus".to_string()
        } else {
            "✨ Showing FORMATTED display focus".to_string()
        };
    }

    fn toggle_cursor_details(&mut self) {
        self.show_cursor_details = !self.show_cursor_details;
        self.debug_message = if self.show_cursor_details {
            "📍 Detailed cursor mapping info ON".to_string()
        } else {
            "📍 Detailed cursor mapping info OFF".to_string()
        };
    }

    fn get_current_field_analysis(&self) -> (String, String, String, Option<String>) {
        let field_index = self.editor.current_field();
        let raw = self.editor.data_provider().field_value(field_index);
        let display = self.editor.current_display_text();

        let status = if raw == display {
            if self.has_formatter() {
                if self.mode() == AppMode::Ins {
                    "Raw (editing)".to_string()
                } else {
                    "No formatting needed".to_string()
                }
            } else {
                "No formatter".to_string()
            }
        } else {
            "Custom formatted".to_string()
        };

        let warning = if self.validation_enabled && self.has_formatter() {
            // Check if there are any formatting warnings
            if !raw.is_empty() {
                match self.editor.current_field() {
                    0 if raw.len() < 5 => Some(format!("PSC incomplete: {}/5", raw.len())),
                    1 if raw.len() < 10 => Some(format!("Phone incomplete: {}/10", raw.len())),
                    2 if raw.len() < 16 => Some(format!("Card incomplete: {}/16", raw.len())),
                    4 if raw.len() < 8 => Some(format!("Date incomplete: {}/8", raw.len())),
                    _ => None,
                }
            } else {
                None
            }
        } else {
            None
        };

        (raw.to_string(), display, status, warning)
    }

    // Delegate methods with enhanced feedback
    fn enter_edit_mode(&mut self) {
        self.editor.enter_edit_mode();
        let field_type = self.current_field_type();
        let rules = self.get_input_rules();
        self.debug_message =
            format!("✏️ INSERT MODE - Cursor: Steady Bar | - {field_type} - {rules}");
    }

    fn exit_edit_mode(&mut self) {
        self.editor.exit_edit_mode();
        let (raw, display, _, warning) = self.get_current_field_analysis();
        if let Some(warn) = warning {
            self.debug_message = format!(
                "🔒 NORMAL - Cursor: Steady Block █ - {} | ⚠️ {}",
                self.current_field_type(),
                warn
            );
        } else if raw != display {
            self.debug_message = format!(
                "🔒 NORMAL - Cursor: Steady Block █ - {} formatted successfully",
                self.current_field_type()
            );
        } else {
            self.debug_message = "🔒 NORMAL MODE - Cursor: Steady Block █".to_string();
        }
    }

    fn insert_char(&mut self, ch: char) -> anyhow::Result<()> {
        let result = self.editor.insert_char(ch);
        if result.is_ok() {
            let (raw, display, _, _) = self.get_current_field_analysis();
            if raw != display && self.validation_enabled {
                self.debug_message = format!("✏️ '{ch}' added - Real-time formatting active");
            } else {
                self.debug_message = format!("✏️ '{ch}' added");
            }
        }
        result
    }

    // Position mapping demo
    fn show_position_mapping(&mut self) {
        if !self.has_formatter() {
            self.debug_message = "📍 No position mapping (plain text field)".to_string();
            return;
        }

        let raw_pos = self.editor.cursor_position();
        let display_pos = self.editor.display_cursor_position();
        let field_index = self.editor.current_field();
        let raw = self.editor.data_provider().field_value(field_index);
        let display = self.editor.current_display_text();

        if raw_pos != display_pos {
            self.debug_message = format!(
                "🗺️ Position mapping: Raw[{}]='{}' ↔ Display[{}]='{}'",
                raw_pos,
                raw.chars().nth(raw_pos).unwrap_or('∅'),
                display_pos,
                display.chars().nth(display_pos).unwrap_or('∅')
            );
        } else {
            self.debug_message = format!("📍 Cursor at position {raw_pos} (no mapping needed)");
        }
    }

    // Delegate remaining methods
    fn mode(&self) -> AppMode {
        self.editor.mode()
    }
    fn current_field(&self) -> usize {
        self.editor.current_field()
    }
    fn cursor_position(&self) -> usize {
        self.editor.cursor_position()
    }
    fn data_provider(&self) -> &D {
        self.editor.data_provider()
    }
    fn data_provider_mut(&mut self) -> &mut D {
        self.editor.data_provider_mut()
    }
    fn ui_state(&self) -> &tui_canvas::EditorState {
        self.editor.ui_state()
    }

    fn move_up(&mut self) {
        let _ = self.editor.move_up();
    }
    fn move_down(&mut self) {
        let _ = self.editor.move_down();
    }
    fn move_left(&mut self) {
        let _ = self.editor.move_left();
    }
    fn move_right(&mut self) {
        let _ = self.editor.move_right();
    }
    fn delete_backward(&mut self) -> anyhow::Result<()> {
        self.editor.delete_backward()
    }
    fn delete_forward(&mut self) -> anyhow::Result<()> {
        self.editor.delete_forward()
    }
    fn next_field(&mut self) {
        let _ = self.editor.next_field();
    }
    fn prev_field(&mut self) {
        let _ = self.editor.prev_field();
    }
}

// Key handling
fn handle_key_press(
    key: KeyCode,
    modifiers: KeyModifiers,
    editor: &mut EnhancedDemoEditor<MultiFormatterDemoData>,
) -> anyhow::Result<bool> {
    let mode = editor.mode();

    // Quit
    if matches!(key, KeyCode::F(10))
        || (key == KeyCode::Char('q') && modifiers.contains(KeyModifiers::CONTROL))
        || (key == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL))
    {
        return Ok(false);
    }

    match (mode, key, modifiers) {
        // Mode transitions
        (AppMode::Nor, KeyCode::Char('i'), _) => editor.enter_edit_mode(),
        (AppMode::Nor, KeyCode::Char('a'), _) => {
            editor.editor.enter_append_mode();
            editor.debug_message = format!(
                "✏️ APPEND {} - {}",
                editor.current_field_type(),
                editor.get_input_rules()
            );
        }
        (_, KeyCode::Esc, _) => editor.exit_edit_mode(),

        // Demo features
        (AppMode::Nor, KeyCode::Char('e'), _) => editor.cycle_example_data(),
        (AppMode::Nor, KeyCode::Char('r'), _) => editor.toggle_raw_data_view(),
        (AppMode::Nor, KeyCode::Char('c'), _) => editor.toggle_cursor_details(),
        (AppMode::Nor, KeyCode::Char('m'), _) => editor.show_position_mapping(),
        (AppMode::Nor, KeyCode::F(1), _) => editor.toggle_validation(),

        // Movement
        (_, KeyCode::Up, _) | (AppMode::Nor, KeyCode::Char('k'), _) => editor.move_up(),
        (_, KeyCode::Down, _) | (AppMode::Nor, KeyCode::Char('j'), _) => editor.move_down(),
        (_, KeyCode::Left, _) | (AppMode::Nor, KeyCode::Char('h'), _) => editor.move_left(),
        (_, KeyCode::Right, _) | (AppMode::Nor, KeyCode::Char('l'), _) => editor.move_right(),
        (_, KeyCode::Tab, _) => editor.next_field(),
        (_, KeyCode::BackTab, _) => editor.prev_field(),

        // Editing
        (AppMode::Ins, KeyCode::Char(c), m) if !m.contains(KeyModifiers::CONTROL) => {
            editor.insert_char(c)?;
        }
        (AppMode::Ins, KeyCode::Backspace, _) => {
            editor.delete_backward()?;
        }
        (AppMode::Ins, KeyCode::Delete, _) => {
            editor.delete_forward()?;
        }

        // Field analysis
        (AppMode::Nor, KeyCode::Char('?'), _) => {
            let (raw, display, status, warning) = editor.get_current_field_analysis();
            let warning_text = warning.map(|w| format!(" ⚠️ {w}")).unwrap_or_default();
            editor.debug_message = format!(
                "🔍 Field {}: {} | Raw: '{}' | Display: '{}'{}",
                editor.current_field() + 1,
                status,
                raw,
                display,
                warning_text
            );
        }

        _ => {}
    }

    Ok(true)
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut editor: EnhancedDemoEditor<MultiFormatterDemoData>,
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
                    editor.debug_message = format!("❌ Error: {e}");
                }
            }
        }
    }
    Ok(())
}

fn ui(f: &mut Frame, editor: &EnhancedDemoEditor<MultiFormatterDemoData>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(18)])
        .split(f.area());

    render_canvas_default(f, chunks[0], &editor.editor);
    render_enhanced_status(f, chunks[1], editor);
}

fn render_enhanced_status(
    f: &mut Frame,
    area: Rect,
    editor: &EnhancedDemoEditor<MultiFormatterDemoData>,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Status bar
            Constraint::Length(6), // Current field analysis
            Constraint::Length(9), // Help
        ])
        .split(area);

    // Status bar
    let mode_text = match editor.mode() {
        AppMode::Ins => "INSERT | (bar cursor)",
        AppMode::Nor => "NORMAL █ (block cursor)",
        _ => "NORMAL █ (block cursor)",
    };

    let formatter_count = (0..editor.data_provider().field_count())
        .filter(|&i| editor.data_provider().validation_config(i).is_some())
        .count();

    let status_text = format!(
        "-- {} -- {} | Formatters: {}/{} active | View: {}{}",
        mode_text,
        editor.debug_message,
        formatter_count,
        editor.data_provider().field_count(),
        if editor.show_raw_data {
            "RAW"
        } else {
            "DISPLAY"
        },
        if editor.show_cursor_details {
            " | CURSOR+"
        } else {
            ""
        }
    );

    let status = Paragraph::new(Line::from(Span::raw(status_text))).block(
        Block::default()
            .borders(Borders::ALL)
            .title("🧩 Enhanced Custom Formatter Demo"),
    );

    f.render_widget(status, chunks[0]);

    // Current field analysis
    let (raw, display, status, warning) = editor.get_current_field_analysis();
    let field_name = editor.data_provider().field_name(editor.current_field());
    let field_type = editor.current_field_type();

    let mut analysis_lines = vec![
        format!("📝 Current: {} ({})", field_name, field_type),
        format!("🔧 Status: {}", status),
    ];

    if editor.show_raw_data || editor.mode() == AppMode::Ins {
        analysis_lines.push(format!("💾 Raw Data: '{raw}'"));
        analysis_lines.push(format!("✨ Display: '{display}'"));
    } else {
        analysis_lines.push(format!("✨ User Sees: '{display}'"));
        analysis_lines.push(format!("💾 Stored As: '{raw}'"));
    }

    if editor.show_cursor_details {
        analysis_lines.push(format!(
            "📍 Cursor: Raw[{}] → Display[{}]",
            editor.cursor_position(),
            editor.editor.display_cursor_position()
        ));
    }

    if let Some(ref warn) = warning {
        analysis_lines.push(format!("⚠️ Warning: {warn}"));
    }

    let analysis_color = if warning.is_some() {
        Color::Yellow
    } else if raw != display && editor.validation_enabled {
        Color::Green
    } else {
        Color::Gray
    };

    let analysis = Paragraph::new(analysis_lines.join("\n"))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("🔍 Field Analysis"),
        )
        .style(Style::default().fg(analysis_color))
        .wrap(Wrap { trim: true });

    f.render_widget(analysis, chunks[1]);

    // Help
    let help_text = match editor.mode() {
        AppMode::Nor => {
            "🎯 CURSOR-STYLE: Normal █ | Insert |\n\
             🧩 ENHANCED CUSTOM FORMATTER DEMO\n\
             \n\
             Try these formatters:
             • PSC: 01001 → 010 01 | Phone: 1234567890 → (123) 456-7890 | Card: 1234567890123456 → 1234 5678 9012 3456
             • Date: 12012024 → 12/01/2024 | Plain: no formatting
             \n\
             Commands: i=insert, e=cycle examples, r=toggle raw/display, c=cursor details, m=position mapping\n\
             Movement: hjkl/arrows, Tab=next field, ?=analyze current field, F1=toggle formatters\n\
             Ctrl+C/F10=quit"
        }
        AppMode::Ins => {
            "🎯 INSERT MODE - Cursor: | (bar)\n\
             ✏️ Real-time formatting as you type!\n\
             \n\
             Current field rules: {}\n\
             • Raw input is authoritative (what gets stored)\n\
             • Display formatting updates in real-time (what users see)\n\
             • Cursor position is mapped between raw and display\n\
             \n\
             Esc=normal mode, arrows=navigate, Backspace/Del=delete"
        }
        _ => "🧩 Enhanced Custom Formatter Demo"
    };

    let formatted_help = if editor.mode() == AppMode::Ins {
        help_text.replace("{}", editor.get_input_rules())
    } else {
        help_text.to_string()
    };

    let help = Paragraph::new(formatted_help)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("🚀 Enhanced Features & Commands"),
        )
        .style(Style::default().fg(Color::Gray))
        .wrap(Wrap { trim: true });

    f.render_widget(help, chunks[2]);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧩 Enhanced Canvas Custom Formatter Demo (Feature 4)");
    println!("✅ validation feature: ENABLED");
    println!("✅ gui feature: ENABLED");
    println!("✅ cursor-style feature: ENABLED");
    println!("🧩 Enhanced features:");
    println!("   • 5 different custom formatters with edge cases");
    println!("   • Real-time format preview and validation");
    println!("   • Advanced cursor position mapping");
    println!("   • Comprehensive error handling and warnings");
    println!("   • Raw vs formatted data separation demos");
    println!();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let data = MultiFormatterDemoData::new();
    let mut editor = EnhancedDemoEditor::new(data);

    // Initialize with normal mode - library automatically sets block cursor
    editor.editor.set_mode(AppMode::Nor);

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

    println!("🧩 Enhanced custom formatter demo completed!");
    println!("🏆 You experienced comprehensive custom formatting with:");
    println!("   • Multiple formatter types (PSC, Phone, Credit Card, Date)");
    println!("   • Edge case handling (incomplete, invalid, overflow)");
    println!("   • Real-time format preview and cursor mapping");
    println!("   • Clear separation between raw business data and display formatting");
    Ok(())
}
