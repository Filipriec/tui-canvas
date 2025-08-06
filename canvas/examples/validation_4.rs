/* examples/validation_4.rs
   Demonstrates Feature 4: Custom parsing/formatting provided by the app,
   displayed by the library while keeping raw input authoritative.

   Use-case: PSC (postal code) typed as "01001" should display as "010 01".
   - Raw input: "01001"
   - Display:   "010 01"
   - Cursor mapping is handled by the library via PositionMapper
   - Validation still applies to raw text (if configured)
   - Formatting is optional and only active when feature "validation" is enabled

   Run with:
     cargo run --example validation_4 --features "gui,validation"
*/

#![allow(clippy::needless_return)]

#[cfg(not(all(feature = "validation", feature = "gui")))]
compile_error!(
    "This example requires the 'validation' and 'gui' features. \
     Run with: cargo run --example validation_4 --features \"gui,validation\""
);

use std::io;
use std::sync::Arc;

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers,
    },
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

// Bring library types
use canvas::{
    canvas::{gui::render_canvas_default, modes::AppMode},
    DataProvider, FormEditor,
    ValidationConfig, ValidationConfigBuilder,
    // Feature 4 exports
    CustomFormatter, FormattingResult,
};

/// PSC custom formatter
///
/// Formats a raw 5-digit PSC as "XXX XX".
/// Examples:
/// - ""        -> ""
/// - "0"       -> "0"
/// - "01"      -> "01"
/// - "010"     -> "010"
/// - "0100"    -> "010 0"
/// - "01001"   -> "010 01"
/// Any extra chars are appended after the space (simple behavior).
struct PSCFormatter;

impl CustomFormatter for PSCFormatter {
    fn format(&self, raw: &str) -> FormattingResult {
        let mut out = String::new();

        for (i, ch) in raw.chars().enumerate() {
            // Insert space after 3rd character for PSC visual grouping
            if i == 3 {
                out.push(' ');
            }
            out.push(ch);
        }

        // Use default position mapper which treats non-alphanumeric as separators
        FormattingResult::success(out)
    }
}

// Demo editor wrapper for custom formatter demonstration (mirror UX from validation_3)
struct PscDemoFormEditor<D: DataProvider> {
    editor: FormEditor<D>,
    debug_message: String,
    command_buffer: String,
    validation_enabled: bool,
    show_raw_data: bool,
}

impl<D: DataProvider> PscDemoFormEditor<D> {
    fn new(data_provider: D) -> Self {
        let mut editor = FormEditor::new(data_provider);
        editor.set_validation_enabled(true);

        Self {
            editor,
            debug_message:
                "🧩 Custom Formatter Demo - App-defined parsing with library-managed display!".to_string(),
            command_buffer: String::new(),
            validation_enabled: true,
            show_raw_data: false,
        }
    }

    // === PSC HELPERS (conditional formatting policy) ===
    fn is_psc_field(&self) -> bool {
        self.editor.current_field() == 0
    }
    fn psc_raw(&self) -> &str {
        if self.is_psc_field() { self.editor.current_text() } else { "" }
    }
    fn psc_is_valid(&self) -> bool {
        let raw = self.psc_raw();
        raw.chars().count() == 5 && raw.chars().all(|c| c.is_ascii_digit())
    }
    fn psc_should_format_for_display(&self) -> bool {
        // Apply formatting only when NOT editing, on PSC field, and valid 5 digits
        self.mode() != AppMode::Edit && self.is_psc_field() && self.psc_is_valid()
    }
    fn psc_filter_input(&self, ch: char) -> bool {
        if !self.is_psc_field() {
            return true;
        }
        // Only allow digits, enforce max 5
        if !ch.is_ascii_digit() {
            return false;
        }
        self.psc_raw().chars().count() < 5
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

    // === FORMATTER CONTROL ===
    fn toggle_validation(&mut self) {
        self.validation_enabled = !self.validation_enabled;
        self.editor.set_validation_enabled(self.validation_enabled);
        if self.validation_enabled {
            self.debug_message = "✅ Custom Formatter ENABLED - Library displays app-formatted output!".to_string();
        } else {
            self.debug_message = "❌ Custom Formatter DISABLED - Raw text only".to_string();
        }
    }

    fn toggle_raw_data_view(&mut self) {
        self.show_raw_data = !self.show_raw_data;
        if self.show_raw_data {
            self.debug_message =
                "👁️ Showing RAW business data (what's actually stored)".to_string();
        } else {
            self.debug_message =
                "✨ Showing FORMATTED display (provided by your app, rendered by library)".to_string();
        }
    }

    fn get_current_field_info(&self) -> (String, String, String) {
        let field_index = self.editor.current_field();
        let raw_data = self.editor.current_text();

        // Conditional display policy:
        // - If editing PSC: show raw (no formatting)
        // - Else if PSC valid and PSC field: show formatted
        // - Else: show raw
        let display_data = if self.is_psc_field() {
            if self.mode() == AppMode::Edit {
                raw_data.to_string()
            } else if self.psc_is_valid() {
                self.editor.current_display_text()
            } else {
                raw_data.to_string()
            }
        } else {
            // Non-PSC field: show raw in this demo
            raw_data.to_string()
        };

        let fmt_info = if self.is_psc_field() {
            if self.psc_is_valid() {
                "CustomFormatter: PSC ‘XXX XX’ (active)".to_string()
            } else {
                "CustomFormatter: PSC ‘XXX XX’ (waiting for 5 digits)".to_string()
            }
        } else {
            "No formatter".to_string()
        };

        (raw_data.to_string(), display_data, fmt_info)
    }

    // === ENHANCED MOVEMENT ===
    fn move_left(&mut self) {
        self.editor.move_left();
        self.update_cursor_info();
    }

    fn move_right(&mut self) {
        self.editor.move_right();
        self.update_cursor_info();
    }

    fn move_up(&mut self) {
        match self.editor.move_up() {
            Ok(()) => {
                self.update_field_info();
            }
            Err(e) => {
                self.debug_message = format!("🚫 Field switch blocked: {}", e);
            }
        }
    }

    fn move_down(&mut self) {
        match self.editor.move_down() {
            Ok(()) => {
                self.update_field_info();
            }
            Err(e) => {
                self.debug_message = format!("🚫 Field switch blocked: {}", e);
            }
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
                    "📍 Cursor: Raw pos {} → Display pos {} (custom formatting active)",
                    raw_pos, display_pos
                );
            } else {
                self.debug_message = format!("📍 Cursor at position {} (no display offset)", raw_pos);
            }
        }
    }

    fn update_field_info(&mut self) {
        let field_name = self
            .editor
            .data_provider()
            .field_name(self.editor.current_field());
        self.debug_message = format!("📝 Switched to: {}", field_name);
    }

    // === MODE TRANSITIONS ===
    fn enter_edit_mode(&mut self) {
        self.editor.enter_edit_mode();
        self.debug_message =
            "✏️ INSERT MODE - Type to see custom formatting applied in real-time".to_string();
    }

    fn enter_append_mode(&mut self) {
        self.editor.enter_append_mode();
        self.debug_message = "✏️ INSERT (append) - Custom formatting active".to_string();
    }

    fn exit_edit_mode(&mut self) {
        self.editor.exit_edit_mode();
        self.debug_message = "🔒 NORMAL MODE - Press 'r' to see raw data".to_string();
    }

    fn insert_char(&mut self, ch: char) -> anyhow::Result<()> {
        // Enforce PSC typing rules on PSC field:
        // - Only digits
        // - Max 5 characters
        if self.is_psc_field() && !self.psc_filter_input(ch) {
            self.debug_message = "🚦 PSC: only digits, max 5".to_string();
            return Ok(());
        }

        let result = self.editor.insert_char(ch);
        if result.is_ok() {
            // In edit mode we always show raw
            let raw = self.editor.current_text().to_string();
            let display = if self.psc_should_format_for_display() {
                self.editor.current_display_text()
            } else {
                raw.clone()
            };
            if raw != display {
                self.debug_message =
                    format!("✏️ Added '{}': Raw='{}' Display='{}'", ch, raw, display);
            } else {
                self.debug_message = format!("✏️ Added '{}': '{}'", ch, raw);
            }
        }
        Ok(result?)
    }

    // === DELETE OPERATIONS ===
    fn delete_backward(&mut self) -> anyhow::Result<()> {
        let result = self.editor.delete_backward();
        if result.is_ok() {
            // In edit mode, we revert to raw view; debug info reflects that
            self.debug_message = "⌫ Character deleted".to_string();
            self.update_cursor_info();
        }
        Ok(result?)
    }

    fn delete_forward(&mut self) -> anyhow::Result<()> {
        let result = self.editor.delete_forward();
        if result.is_ok() {
            // In edit mode, we revert to raw view; debug info reflects that
            self.debug_message = "⌦ Character deleted".to_string();
            self.update_cursor_info();
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
                self.update_field_info();
            }
            Err(e) => {
                self.debug_message = format!("🚫 Cannot move to next field: {}", e);
            }
        }
    }

    fn prev_field(&mut self) {
        match self.editor.prev_field() {
            Ok(()) => {
                self.update_field_info();
            }
            Err(e) => {
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

    fn show_formatter_details(&mut self) {
        let (raw, display, fmt_info) = self.get_current_field_info();
        self.debug_message = format!(
            "🔍 Field {}: {} | Raw: '{}' Display: '{}'",
            self.current_field() + 1,
            fmt_info,
            raw,
            display
        );
    }

    fn get_formatter_status(&self) -> String {
        if !self.validation_enabled {
            return "❌ DISABLED".to_string();
        }

        // Count fields with validation config (for demo parity)
        let field_count = self.editor.data_provider().field_count();
        let mut cfg_count = 0;
        for i in 0..field_count {
            if self.editor.validation_state().get_field_config(i).is_some() {
                cfg_count += 1;
            }
        }

        format!("🧩 {} FORMATTERS", cfg_count)
    }
}

// Demo data with a PSC field configured with a custom formatter
struct PscDemoData {
    fields: Vec<(String, String)>,
}

impl PscDemoData {
    fn new() -> Self {
        Self {
            fields: vec![
                ("🏁 PSC (type 01001)".to_string(), "".to_string()),
                ("📝 Notes (raw)".to_string(), "".to_string()),
            ],
        }
    }
}

impl DataProvider for PscDemoData {
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

    // Provide validation config with custom formatter for field 0
    #[cfg(feature = "validation")]
    fn validation_config(&self, field_index: usize) -> Option<ValidationConfig> {
        match field_index {
            0 => {
                // PSC 5 digits displayed as "XXX XX". Raw value remains unmodified.
                let cfg = ValidationConfigBuilder::new()
                    .with_custom_formatter(Arc::new(PSCFormatter))
                    // Optional: add character limits or patterns for raw value
                    // .with_max_length(5)
                    .build();
                Some(cfg)
            }
            _ => None,
        }
    }
}

// Enhanced key handling with custom formatter specific commands
fn handle_key_press(
    key: KeyCode,
    modifiers: KeyModifiers,
    editor: &mut PscDemoFormEditor<PscDemoData>,
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

        // === FORMATTER-SPECIFIC COMMANDS ===
        (AppMode::ReadOnly, KeyCode::Char('m'), _) => {
            editor.show_formatter_details();
            editor.clear_command_buffer();
        }
        (AppMode::ReadOnly, KeyCode::Char('r'), _) => {
            editor.toggle_raw_data_view();
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

        // Line movement
        (AppMode::ReadOnly, KeyCode::Char('0'), _) => {
            editor.move_line_start();
            editor.clear_command_buffer();
        }
        (AppMode::ReadOnly, KeyCode::Char('$'), _) => {
            editor.move_line_end();
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
            let (raw, display, fmt_info) = editor.get_current_field_info();
            editor.set_debug_message(format!(
                "Field {}/{}, Cursor {}, {}, Raw: '{}', Display: '{}'",
                editor.current_field() + 1,
                editor.data_provider().field_count(),
                editor.cursor_position(),
                fmt_info,
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
    mut editor: PscDemoFormEditor<PscDemoData>,
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

fn ui(f: &mut Frame, editor: &PscDemoFormEditor<PscDemoData>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(16)])
        .split(f.area());

    render_enhanced_canvas(f, chunks[0], editor);
    render_formatter_status(f, chunks[1], editor);
}

fn render_enhanced_canvas(f: &mut Frame, area: Rect, editor: &PscDemoFormEditor<PscDemoData>) {
    render_canvas_default(f, area, &editor.editor);
}

fn render_formatter_status(
    f: &mut Frame,
    area: Rect,
    editor: &PscDemoFormEditor<PscDemoData>,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Status bar
            Constraint::Length(6), // Data comparison
            Constraint::Length(7), // Help
        ])
        .split(area);

    // Status bar with formatter information
    let mode_text = match editor.mode() {
        AppMode::Edit => "INSERT",
        AppMode::ReadOnly => "NORMAL",
        _ => "OTHER",
    };

    let fmt_status = editor.get_formatter_status();
    let status_text = format!(
        "-- {} -- {} | Formatters: {} | View: {}",
        mode_text,
        editor.debug_message(),
        fmt_status,
        if editor.show_raw_data { "RAW" } else { "FORMATTED" }
    );

    let status =
        Paragraph::new(Line::from(Span::raw(status_text)))
            .block(Block::default().borders(Borders::ALL).title("🧩 Custom Formatter Demo"));

    f.render_widget(status, chunks[0]);

    // Data comparison showing raw vs display
    let (raw_data, display_data, fmt_info) = editor.get_current_field_info();
    let field_name = editor.data_provider().field_name(editor.current_field());

    let comparison_text = format!(
        "📝 Current Field: {}\n\
         🔧 Formatter Config: {}\n\
         \n\
         💾 Raw Business Data: '{}' ← What's actually stored in your database\n\
         ✨ Formatted Display: '{}' ← What users see in the interface\n\
         📍 Cursor: Raw pos {} → Display pos {}",
        field_name,
        fmt_info,
        raw_data,
        display_data,
        editor.cursor_position(),
        editor.editor.display_cursor_position()
    );

    let comparison_style = if raw_data != display_data {
        Style::default().fg(Color::Green) // Green when formatting is active
    } else {
        Style::default().fg(Color::Gray) // Gray when no formatting
    };

    let data_comparison = Paragraph::new(comparison_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("📊 Raw Data vs App-Provided Formatting"),
        )
        .style(comparison_style)
        .wrap(Wrap { trim: true });

    f.render_widget(data_comparison, chunks[1]);

    // Help text
    let help_text = match editor.mode() {
        AppMode::ReadOnly => {
            "🧩 CUSTOM FORMATTER DEMO: App provides parsing/formatting; library displays and maps cursor!\n\
             \n\
             Try the PSC field:\n\
             • Type: 01001  → Display: 010 01\n\
             • Raw data stays unmodified: '01001'\n\
             \n\
             Commands: i/a=insert, m=formatter details, r=toggle raw/display view\n\
             Movement: hjkl/arrows=move, 0/$ line start/end, Tab=next field, F1=toggle formatting\n\
             ?=detailed info, Ctrl+C=quit"
        }
        AppMode::Edit => {
            "✏️ INSERT MODE - Type to see real-time custom formatter output!\n\
             \n\
             Key Points:\n\
             • Your app formats; library displays and maps cursor\n\
             • Raw input is authoritative for validation and storage\n\
             \n\
             arrows=move, Backspace/Del=delete, Esc=normal, Tab=next field"
        }
        _ => "🧩 Custom Formatter Demo Active!"
    };

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("🚀 Formatter Features & Commands"),
        )
        .style(Style::default().fg(Color::Gray))
        .wrap(Wrap { trim: true });

    f.render_widget(help, chunks[2]);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Print feature status
    println!("🧩 Canvas Custom Formatter Demo (Feature 4)");
    println!("✅ validation feature: ENABLED");
    println!("✅ gui feature: ENABLED");
    println!("🧩 Custom formatting: ACTIVE");
    println!("🔥 Key Benefits Demonstrated:");
    println!("   • App decides how to display values (e.g., PSC '01001' → '010 01')");
    println!("   • Library handles display + cursor mapping automatically");
    println!("   • Raw input remains authoritative for validation/storage");
    println!();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let data = PscDemoData::new();
    let editor = PscDemoFormEditor::new(data);

    let res = run_app(&mut terminal, editor);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    println!("🧩 Custom formatter demo completed!");
    println!("🏆 You saw how app-defined formatting integrates seamlessly with the library!");
    Ok(())
}