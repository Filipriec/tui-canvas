// examples/validation_5.rs
//! Feature 5: External validation (UI-only) demo with Feature 4 (custom formatter)
//!
//! Behavior:
//! - Field 1 (PSC) uses a custom formatter (Feature 4) and external validation (Feature 5).
//!   • While editing PSC: raw text, capped at 5 digits
//!   • When not editing PSC (moved focus or Esc) and raw is 5 digits: shows formatted ("XXX XX")
//!   • After leaving PSC (or pressing 'v'), external validation kicks in:
//!      - Validating -> Valid/Invalid/Warning based on simple rules (LSP-like simulation)
//! - Field 2 (Notes) is plain text, no formatter, no external validation.
//!
//! Controls:
//! - i/a: insert/append
//! - Esc: exit edit mode (triggers PSC validation if enabled)
//! - Tab/Shift+Tab: next/prev field (triggers PSC validation if enabled)
//! - v: manually trigger validation of current field
//! - c: clear external validation state for current field
//! - r: toggle raw/format view flag in panel (visual only; canvas follows library rules)
//! - F10/Ctrl+C: quit
//!
//! Run: cargo run --example validation_5 --features "gui,validation"

#![allow(clippy::needless_return)]

#[cfg(not(all(feature = "validation", feature = "gui")))]
compile_error!(
    "This example requires the 'validation' and 'gui' features. \
     Run with: cargo run --example validation_5 --features \"gui,validation\""
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

use canvas::{
    canvas::{gui::render_canvas_default, modes::AppMode},
    DataProvider, FormEditor,
    ValidationConfigBuilder, CustomFormatter, FormattingResult,
    validation::ExternalValidationState,
};

/// PSC custom formatter for display ("XXX XX").
struct PSCFormatter;

impl CustomFormatter for PSCFormatter {
    fn format(&self, raw: &str) -> FormattingResult {
        let mut out = String::new();
        for (i, ch) in raw.chars().enumerate() {
            if i == 3 {
                out.push(' ');
            }
            out.push(ch);
        }
        FormattingResult::success(out)
    }
}

// Demo data provider: PSC + Notes
struct DemoData {
    fields: Vec<(String, String)>,
}

impl DemoData {
    fn new() -> Self {
        Self {
            fields: vec![
                ("🏁 PSC (5 digits)".to_string(), "".to_string()),
                ("📝 Notes".to_string(), "".to_string()),
            ],
        }
    }
}

impl DataProvider for DemoData {
    fn field_count(&self) -> usize { self.fields.len() }
    fn field_name(&self, index: usize) -> &str { &self.fields[index].0 }
    fn field_value(&self, index: usize) -> &str { &self.fields[index].1 }
    fn set_field_value(&mut self, index: usize, value: String) { self.fields[index].1 = value; }

    #[cfg(feature = "validation")]
    fn validation_config(&self, field_index: usize) -> Option<canvas::ValidationConfig> {
        match field_index {
            0 => {
                // PSC: Feature 4 + Feature 5 + max length cap
                Some(
                    ValidationConfigBuilder::new()
                        .with_custom_formatter(Arc::new(PSCFormatter))
                        .with_max_length(5) // Enforce raw max length during typing
                        .with_external_validation_enabled(true) // Show external validation indicator
                        .build()
                )
            }
            _ => None,
        }
    }
}

/// Simulated LSP-like validator:
/// - Receives the raw PSC and returns a state (Validating -> Valid/Warn/Invalid).
/// - This is called by our "frontend" workflow when we leave PSC (or press 'v').
///   In real apps, this would be an async backend call; we just simulate results.
fn simulate_external_psc_validation(raw_psc: &str) -> ExternalValidationState {
    // Edge cases and outcomes:
    // - Empty -> NotValidated (frontend shouldn't call on empty, but handle gracefully)
    // - Non-digit -> Invalid
    // - Length < 5 -> Warning (incomplete)
    // - Exactly 5 digits:
    //     * "00000" -> Invalid (nonsensical)
    //     * "12345" -> Warning (pretend partial region)
    //     * "01001" -> Valid(Some("Known district"))
    //     * otherwise Valid(None)
    if raw_psc.is_empty() {
        return ExternalValidationState::NotValidated;
    }
    if !raw_psc.chars().all(|c| c.is_ascii_digit()) {
        return ExternalValidationState::Invalid { message: "PSC must be digits".to_string(), suggestion: Some("Only 0-9".to_string()) };
    }
    let len = raw_psc.chars().count();
    if len < 5 {
        return ExternalValidationState::Warning { message: format!("PSC incomplete: {}/5", len) };
    }
    // len == 5
    match raw_psc {
        "00000" => ExternalValidationState::Invalid { message: "PSC cannot be 00000".to_string(), suggestion: Some("Try 01001".to_string()) },
        "12345" => ExternalValidationState::Warning { message: "Unrecognized region: verify".to_string() },
        "01001" => ExternalValidationState::Valid(Some("Known district".to_string())),
        _ => ExternalValidationState::Valid(None),
    }
}

// Editor wrapper to manage external validation workflow and show UI info
struct DemoEditor<D: DataProvider> {
    editor: FormEditor<D>,
    debug_message: String,
    show_raw_hint: bool, // panel-only toggle, does not affect canvas
}

impl<D: DataProvider> DemoEditor<D> {
    fn new(data_provider: D) -> Self {
        let mut editor = FormEditor::new(data_provider);
        editor.set_validation_enabled(true);

        Self {
            editor,
            debug_message: "🧪 External validation demo (Feature 5) + Custom formatting (Feature 4)".to_string(),
            show_raw_hint: false,
        }
    }

    fn current_field(&self) -> usize { self.editor.current_field() }
    fn mode(&self) -> AppMode { self.editor.mode() }
    fn data_provider(&self) -> &D { self.editor.data_provider() }
    fn cursor_position(&self) -> usize { self.editor.cursor_position() }
    fn ui_state(&self) -> &canvas::EditorState { self.editor.ui_state() }

    // Minimal pass-through editing controls
    fn enter_edit_mode(&mut self) {
        self.editor.enter_edit_mode();
        self.debug_message = "✏️ Edit PSC or Notes. PSC: raw while editing, formatted when not editing".to_string();
    }
    fn enter_append_mode(&mut self) {
        self.editor.enter_append_mode();
        self.debug_message = "✏️ Append mode active".to_string();
    }
    fn exit_edit_mode(&mut self) {
        self.editor.exit_edit_mode();
        self.debug_message = "🔒 Normal mode".to_string();
        // Trigger external validation upon exiting the field (Feature 5 timing)
        self.trigger_field_validation(self.editor.current_field());
    }

    fn next_field(&mut self) {
        match self.editor.next_field() {
            Ok(()) => {
                self.debug_message = "➡ Switched to next field".to_string();
                // Validate field we just left (Feature 5 timing)
                let prev = self.editor.current_field().saturating_sub(1);
                self.trigger_field_validation(prev);
            }
            Err(e) => {
                self.debug_message = format!("🚫 Cannot move to next field: {}", e);
            }
        }
    }
    fn prev_field(&mut self) {
        match self.editor.prev_field() {
            Ok(()) => {
                self.debug_message = "⬅ Switched to previous field".to_string();
                // Validate field we just left
                let prev = self.editor.current_field() + 1;
                self.trigger_field_validation(prev);
            }
            Err(e) => {
                self.debug_message = format!("🚫 Cannot move to previous field: {}", e);
            }
        }
    }

    fn insert_char(&mut self, ch: char) -> anyhow::Result<()> {
        self.editor.insert_char(ch)
    }
    fn delete_backward(&mut self) -> anyhow::Result<()> {
        self.editor.delete_backward()
    }
    fn delete_forward(&mut self) -> anyhow::Result<()> {
        self.editor.delete_forward()
    }

    // Toggle panel hint: raw vs formatted clarification (canvas obeys library rules)
    fn toggle_raw_hint(&mut self) {
        self.show_raw_hint = !self.show_raw_hint;
        self.debug_message = if self.show_raw_hint {
            "👁 Showing raw vs display hints in panel".to_string()
        } else {
            "🎭 Hints off".to_string()
        };
    }

    // Manually trigger validation for current field (press 'v')
    fn trigger_current_field_validation(&mut self) {
        let idx = self.editor.current_field();
        self.trigger_field_validation(idx);
    }

    // Core: "frontend" triggers validation on blur
    fn trigger_field_validation(&mut self, field_index: usize) {
        // Only if field exists and has external_validation enabled
        if let Some(cfg) = self.editor.validation_state().get_field_config(field_index) {
            if cfg.external_validation_enabled {
                let raw = self.editor.data_provider().field_value(field_index).to_string();
                if raw.is_empty() {
                    // Clear if empty
                    self.editor.clear_external_validation(field_index);
                    return;
                }
                // Set Validating
                self.editor.set_external_validation(field_index, ExternalValidationState::Validating);

                // "Async" backend simulation: immediate result calculation. In a real app, do this later.
                let result = simulate_external_psc_validation(&raw);
                self.editor.set_external_validation(field_index, result);
            }
        }
    }

    // Read external validation state for panel
    fn external_state(&self, field_index: usize) -> ExternalValidationState {
        self.editor.validation_state().get_external_validation(field_index)
    }

    // Clear external validation state (press 'c')
    fn clear_external_state(&mut self) {
        let idx = self.editor.current_field();
        self.editor.clear_external_validation(idx);
        self.debug_message = "🧹 Cleared external validation state".to_string();
    }
}

// UI

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut editor: DemoEditor<DemoData>,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &editor))?;

        if let Event::Key(key) = event::read()? {
            let mode = editor.mode();
            let kc = key.code;
            let km = key.modifiers;

            // Quit
            if (kc == KeyCode::Char('q') && km.contains(KeyModifiers::CONTROL))
                || (kc == KeyCode::Char('c') && km.contains(KeyModifiers::CONTROL))
                || kc == KeyCode::F(10)
            {
                break;
            }

            match (mode, kc, km) {
                // Modes
                (AppMode::ReadOnly, KeyCode::Char('i'), _) => editor.enter_edit_mode(),
                (AppMode::ReadOnly, KeyCode::Char('a'), _) => editor.enter_append_mode(),
                (_, KeyCode::Esc, _) => editor.exit_edit_mode(),

                // Movement
                (_, KeyCode::Tab, _) => editor.next_field(),
                (_, KeyCode::BackTab, _) => editor.prev_field(),

                // Edit
                (AppMode::Edit, KeyCode::Char(c), m) if !m.contains(KeyModifiers::CONTROL) => {
                    let _ = editor.insert_char(c);
                }
                (AppMode::Edit, KeyCode::Backspace, _) => { let _ = editor.delete_backward(); }
                (AppMode::Edit, KeyCode::Delete, _) => { let _ = editor.delete_forward(); }

                // External validation
                (_, KeyCode::Char('v'), _) => {
                    editor.trigger_current_field_validation();
                }
                (_, KeyCode::Char('c'), _) => {
                    editor.clear_external_state();
                }

                // Panel
                (_, KeyCode::Char('r'), _) => editor.toggle_raw_hint(),

                _ => {}
            }
        }
    }

    Ok(())
}

fn ui(f: &mut Frame, editor: &DemoEditor<DemoData>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(14)])
        .split(f.area());

    render_canvas_default(f, chunks[0], &editor.editor);

    render_status_panel(f, chunks[1], editor);
}

fn render_status_panel(f: &mut Frame, area: Rect, editor: &DemoEditor<DemoData>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Length(6),
        ])
        .split(area);

    // Bar
    let mode_text = match editor.mode() {
        AppMode::Edit => "INSERT",
        AppMode::ReadOnly => "NORMAL",
        _ => "OTHER",
    };

    let bar = Paragraph::new(Line::from(Span::raw(format!(
        "-- {} -- {}",
        mode_text,
        editor.debug_message
    ))))
    .block(Block::default().borders(Borders::ALL).title("🧪 External Validation Demo"));

    f.render_widget(bar, chunks[0]);

    // External validation snapshot
    let mut lines = Vec::new();
    let field_count = editor.data_provider().field_count();
    for i in 0..field_count {
        let name = editor.data_provider().field_name(i);
        let raw = editor.editor.data_provider().field_value(i);
        let display = editor.editor.display_text_for_field(i);
        let state = editor.external_state(i);
        let (label, color) = match state {
            ExternalValidationState::NotValidated => ("Not validated", Color::Gray),
            ExternalValidationState::Validating => ("Validating…", Color::Blue),
            ExternalValidationState::Valid(Some(_)) => ("Valid ✓", Color::Green),
            ExternalValidationState::Valid(None) => ("Valid ✓", Color::Green),
            ExternalValidationState::Invalid { .. } => ("Invalid ✖", Color::Red),
            ExternalValidationState::Warning { .. } => ("Warning ⚠", Color::Yellow),
        };

        let mut text = format!("{}: ", name);
        if editor.show_raw_hint {
            text.push_str(&format!("raw='{}' display='{}' | ", raw, display));
        }
        text.push_str(label);

        lines.push(Span::styled(text, Style::default().fg(color)));
        lines.push(Span::raw("\n"));
    }

    let snapshot = Paragraph::new(Line::from(lines))
        .block(Block::default().borders(Borders::ALL).title("📡 External validation states"));
    f.render_widget(snapshot, chunks[1]);

    // Help
    let help = Paragraph::new(
        "Controls:\n\
         • i/a: insert/append, Esc: exit edit (validates PSC)\n\
         • Tab/Shift+Tab: switch fields (validates PSC)\n\
         • v: validate current field now, c: clear validation state\n\
         • r: toggle raw/display hints (panel only)\n\
         • Ctrl+C/F10: quit\n\
         \n\
         PSC rules:\n\
         • Raw typing only while editing (Feature 4) and max 5 digits\n\
         • On blur/exit: external validation occurs (Feature 5)\n\
         • Known Valid: 01001; Invalid: 00000 or non-digits; Warning: incomplete or 12345",
    )
    .style(Style::default().fg(Color::Gray))
    .block(Block::default().borders(Borders::ALL).title("ℹ Help"))
    .wrap(Wrap { trim: true });
    f.render_widget(help, chunks[2]);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Feature 5: External validation (UI-only) + Feature 4: Custom formatter");
    println!("✅ validation feature: ENABLED");
    println!("✅ gui feature: ENABLED");

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let data = DemoData::new();
    let editor = DemoEditor::new(data);

    let res = run_app(&mut terminal, editor);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    println!("🎉 Example finished");
    Ok(())
}

