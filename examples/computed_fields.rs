// examples/computed_fields.rs - COMPLETE WORKING VERSION
//! Demonstrates computed fields with the canvas library - Invoice Calculator Example
//!
//! This example REQUIRES the `computed` feature to compile.
//!
//! Run with:
//! cargo run --example computed_fields --features "gui,computed"

#[cfg(not(feature = "computed"))]
compile_error!(
    "This example requires the 'computed' feature. \
     Run with: cargo run --example computed_fields --features \"gui,computed\""
);

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;

use tui_canvas::{
    render_canvas_default, AppMode,
    computed::{ComputedContext, ComputedProvider},
    DataProvider, TextForm,
};

/// Invoice data with computed fields
struct InvoiceData {
    fields: Vec<(String, String)>,
    computed_indices: std::collections::HashSet<usize>,
}

impl InvoiceData {
    fn new() -> Self {
        let mut computed_indices = std::collections::HashSet::new();

        // Mark computed fields (read-only, calculated)
        computed_indices.insert(4); // Subtotal
        computed_indices.insert(5); // Tax Amount
        computed_indices.insert(6); // Total

        Self {
            fields: vec![
                ("📦 Product Name".to_string(), "".to_string()),
                ("🔢 Quantity".to_string(), "".to_string()),
                ("💰 Unit Price ($)".to_string(), "".to_string()),
                ("📊 Tax Rate (%)".to_string(), "".to_string()),
                ("➕ Subtotal ($)".to_string(), "".to_string()), // COMPUTED
                ("🧾 Tax Amount ($)".to_string(), "".to_string()), // COMPUTED
                ("💳 Total ($)".to_string(), "".to_string()),    // COMPUTED
                ("📝 Notes".to_string(), "".to_string()),
            ],
            computed_indices,
        }
    }
}

impl DataProvider for InvoiceData {
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

    /// Mark computed fields
    fn is_computed_field(&self, field_index: usize) -> bool {
        self.computed_indices.contains(&field_index)
    }

    /// Get computed field values
    fn computed_field_value(&self, field_index: usize) -> Option<String> {
        if self.computed_indices.contains(&field_index) {
            Some(self.fields[field_index].1.clone())
        } else {
            None
        }
    }
}

/// Invoice calculator - computes totals based on input fields
struct InvoiceCalculator;

impl ComputedProvider for InvoiceCalculator {
    fn compute_field(&mut self, context: ComputedContext) -> String {
        // Helper to parse field values safely
        let parse_field = |index: usize| -> f64 {
            let value = context.field_values[index].trim();
            if value.is_empty() {
                0.0
            } else {
                value.parse().unwrap_or(0.0)
            }
        };

        match context.target_field {
            4 => {
                // Subtotal = Quantity × Unit Price
                let qty = parse_field(1);
                let price = parse_field(2);
                let subtotal = qty * price;

                if qty == 0.0 || price == 0.0 {
                    "".to_string() // Show empty if no meaningful calculation
                } else {
                    format!("{subtotal:.2}")
                }
            }
            5 => {
                // Tax Amount = Subtotal × (Tax Rate / 100)
                let qty = parse_field(1);
                let price = parse_field(2);
                let tax_rate = parse_field(3);
                let subtotal = qty * price;
                let tax_amount = subtotal * (tax_rate / 100.0);

                if subtotal == 0.0 || tax_rate == 0.0 {
                    "".to_string()
                } else {
                    format!("{tax_amount:.2}")
                }
            }
            6 => {
                // Total = Subtotal + Tax Amount
                let qty = parse_field(1);
                let price = parse_field(2);
                let tax_rate = parse_field(3);
                let subtotal = qty * price;

                if subtotal == 0.0 {
                    "".to_string()
                } else {
                    let tax_amount = subtotal * (tax_rate / 100.0);
                    let total = subtotal + tax_amount;
                    format!("{total:.2}")
                }
            }
            _ => "".to_string(),
        }
    }

    fn handles_field(&self, field_index: usize) -> bool {
        matches!(field_index, 4..=6) // Subtotal, Tax Amount, Total
    }

    fn field_dependencies(&self, field_index: usize) -> Vec<usize> {
        match field_index {
            4 => vec![1, 2],    // Subtotal depends on Quantity, Unit Price
            5 => vec![1, 2, 3], // Tax Amount depends on Quantity, Unit Price, Tax Rate
            6 => vec![1, 2, 3], // Total depends on Quantity, Unit Price, Tax Rate
            _ => vec![],
        }
    }
}

/// Editor with computed fields
struct ComputedFieldsEditor<D: DataProvider> {
    editor: TextForm<D>,
    calculator: InvoiceCalculator,
    debug_message: String,
    last_computed_values: Vec<String>,
}

impl<D: DataProvider> ComputedFieldsEditor<D> {
    fn new(data_provider: D) -> Self {
        let mut editor = TextForm::new(data_provider);
        editor.set_computed_provider(InvoiceCalculator);

        let calculator = InvoiceCalculator;
        let last_computed_values = vec!["".to_string(); 8];

        Self {
            editor,
            calculator,
            debug_message: "💰 Invoice Calculator - Start typing in fields to see calculations!"
                .to_string(),
            last_computed_values,
        }
    }

    fn is_computed_field(&self, field_index: usize) -> bool {
        self.editor.ui_state().is_computed_field(field_index)
    }

    fn update_computed_fields(&mut self) {
        self.editor.recompute_all_fields(&mut self.calculator);

        for i in [4, 5, 6] {
            // Computed field indices
            let computed_value = self.editor.effective_field_value(i);
            self.editor
                .data_provider_mut()
                .set_field_value(i, computed_value.clone());
        }

        let mut changed = false;
        let mut has_calculations = false;

        for i in [4, 5, 6] {
            let new_value = self.editor.effective_field_value(i);
            if new_value != self.last_computed_values[i] {
                changed = true;
                self.last_computed_values[i] = new_value.clone();
            }
            if !new_value.is_empty() {
                has_calculations = true;
            }
        }

        if changed {
            if has_calculations {
                let subtotal = &self.last_computed_values[4];
                let tax = &self.last_computed_values[5];
                let total = &self.last_computed_values[6];

                let mut parts = Vec::new();
                if !subtotal.is_empty() {
                    parts.push(format!("Subtotal=${subtotal}"));
                }
                if !tax.is_empty() {
                    parts.push(format!("Tax=${tax}"));
                }
                if !total.is_empty() {
                    parts.push(format!("Total=${total}"));
                }

                if !parts.is_empty() {
                    self.debug_message = format!("🧮 Calculated: {}", parts.join(", "));
                } else {
                    self.debug_message =
                        "💰 Enter Quantity and Unit Price to see calculations".to_string();
                }
            } else {
                self.debug_message =
                    "💰 Enter Quantity and Unit Price to see calculations".to_string();
            }
        }
    }

    fn insert_char(&mut self, ch: char) -> anyhow::Result<()> {
        let current_field = self.editor.current_field();
        let result = self.editor.insert_char(ch);

        if result.is_ok() && matches!(current_field, 1..=3) {
            self.editor
                .on_field_changed(&mut self.calculator, current_field);
            self.update_computed_fields();
        }

        result
    }

    fn delete_backward(&mut self) -> anyhow::Result<()> {
        let current_field = self.editor.current_field();
        let result = self.editor.delete_backward();

        if result.is_ok() && matches!(current_field, 1..=3) {
            self.editor
                .on_field_changed(&mut self.calculator, current_field);
            self.update_computed_fields();
        }

        result
    }

    fn delete_forward(&mut self) -> anyhow::Result<()> {
        let current_field = self.editor.current_field();
        let result = self.editor.delete_forward();

        if result.is_ok() && matches!(current_field, 1..=3) {
            self.editor
                .on_field_changed(&mut self.calculator, current_field);
            self.update_computed_fields();
        }

        result
    }

    fn next_field(&mut self) {
        let old_field = self.editor.current_field();
        let _ = self.editor.next_field();
        let new_field = self.editor.current_field();

        if old_field != new_field {
            let field_name = self.editor.data_provider().field_name(new_field);
            let field_type = if self.is_computed_field(new_field) {
                "computed (read-only)"
            } else {
                "editable"
            };
            self.debug_message = format!("→ {field_name} - {field_type} field");
        }
    }

    fn prev_field(&mut self) {
        let old_field = self.editor.current_field();
        let _ = self.editor.prev_field();
        let new_field = self.editor.current_field();

        if old_field != new_field {
            let field_name = self.editor.data_provider().field_name(new_field);
            let field_type = if self.is_computed_field(new_field) {
                "computed (read-only)"
            } else {
                "editable"
            };
            self.debug_message = format!("← {field_name} - {field_type} field");
        }
    }

    fn enter_edit_mode(&mut self) {
        let current = self.editor.current_field();

        // Double protection: check both ways
        if self.editor.data_provider().is_computed_field(current) || self.is_computed_field(current)
        {
            let field_name = self.editor.data_provider().field_name(current);
            self.debug_message = format!(
                "🚫 {field_name} is computed (read-only) - Press Tab to move to editable fields"
            );
            return;
        }

        self.editor.enter_edit_mode();
        let field_name = self.editor.data_provider().field_name(current);
        self.debug_message = format!("✏️ Editing {field_name} - Type to see calculations update");
    }

    fn enter_append_mode(&mut self) {
        let current = self.editor.current_field();

        if self.editor.data_provider().is_computed_field(current) || self.is_computed_field(current)
        {
            let field_name = self.editor.data_provider().field_name(current);
            self.debug_message = format!(
                "🚫 {field_name} is computed (read-only) - Press Tab to move to editable fields"
            );
            return;
        }

        self.editor.enter_append_mode();
        let field_name = self.editor.data_provider().field_name(current);
        self.debug_message = format!("✏️ Appending to {field_name} - Type to see calculations");
    }

    fn exit_edit_mode(&mut self) {
        let current_field = self.editor.current_field();
        self.editor.exit_edit_mode();

        if matches!(current_field, 1..=3) {
            self.editor
                .on_field_changed(&mut self.calculator, current_field);
            self.update_computed_fields();
        }

        self.debug_message = "🔒 Normal mode - Press 'i' to edit fields".to_string();
    }

    // Delegate methods
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
    fn move_left(&mut self) {
        self.editor.move_left();
    }
    fn move_right(&mut self) {
        self.editor.move_right();
    }
    fn move_up(&mut self) {
        let _ = self.editor.move_up();
    }
    fn move_down(&mut self) {
        let _ = self.editor.move_down();
    }
}

fn handle_key_press(
    key: KeyCode,
    modifiers: KeyModifiers,
    editor: &mut ComputedFieldsEditor<InvoiceData>,
) -> anyhow::Result<bool> {
    let mode = editor.mode();

    if (key == KeyCode::Char('q') && modifiers.contains(KeyModifiers::CONTROL))
        || (key == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL))
        || key == KeyCode::F(10)
    {
        return Ok(false);
    }

    match (mode, key, modifiers) {
        (AppMode::Nor, KeyCode::Char('i'), _) => {
            editor.enter_edit_mode();
        }
        (AppMode::Nor, KeyCode::Char('a'), _) => {
            editor.enter_append_mode();
        }
        (AppMode::Nor, KeyCode::Char('A'), _) => {
            editor.editor.move_line_end();
            editor.enter_edit_mode();
        }
        (_, KeyCode::Esc, _) => {
            if mode == AppMode::Ins {
                editor.exit_edit_mode();
            }
        }

        // Movement
        (AppMode::Nor, KeyCode::Char('h'), _) | (AppMode::Nor, KeyCode::Left, _) => {
            editor.move_left();
        }
        (AppMode::Nor, KeyCode::Char('l'), _) | (AppMode::Nor, KeyCode::Right, _) => {
            editor.move_right();
        }
        (AppMode::Nor, KeyCode::Char('j'), _) | (AppMode::Nor, KeyCode::Down, _) => {
            editor.move_down();
        }
        (AppMode::Nor, KeyCode::Char('k'), _) | (AppMode::Nor, KeyCode::Up, _) => {
            editor.move_up();
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

        // Navigation
        (_, KeyCode::Tab, _) => {
            editor.next_field();
        }
        (_, KeyCode::BackTab, _) => {
            editor.prev_field();
        }

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

        // Debug info
        (AppMode::Nor, KeyCode::Char('?'), _) => {
            let current = editor.current_field();
            let field_name = editor.data_provider().field_name(current);
            let field_type = if editor.is_computed_field(current) {
                "COMPUTED (read-only)"
            } else {
                "EDITABLE"
            };
            editor.debug_message = format!(
                "{} - {} - Position {} - Mode: {:?}",
                field_name,
                field_type,
                editor.cursor_position(),
                mode
            );
        }

        _ => {}
    }

    Ok(true)
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut editor: ComputedFieldsEditor<InvoiceData>,
) -> io::Result<()> {
    editor.update_computed_fields(); // Initial computation

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
                    editor.debug_message = format!("Error: {e}");
                }
            }
        }
    }

    Ok(())
}

fn ui(f: &mut Frame, editor: &ComputedFieldsEditor<InvoiceData>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(10)])
        .split(f.area());

    render_enhanced_canvas(f, chunks[0], editor);
    render_computed_status(f, chunks[1], editor);
}

fn render_enhanced_canvas(f: &mut Frame, area: Rect, editor: &ComputedFieldsEditor<InvoiceData>) {
    render_canvas_default(f, area, &editor.editor);
}

fn render_computed_status(f: &mut Frame, area: Rect, editor: &ComputedFieldsEditor<InvoiceData>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(7)])
        .split(area);

    let mode_text = match editor.mode() {
        AppMode::Ins => "INSERT",
        AppMode::Nor => "NORMAL",
        _ => "OTHER",
    };

    let current = editor.current_field();
    let field_status = if editor.is_computed_field(current) {
        "📊 COMPUTED FIELD (read-only)"
    } else {
        "✏️ EDITABLE FIELD"
    };

    let status_text = format!(
        "-- {} -- {} | {}",
        mode_text, field_status, editor.debug_message
    );

    let status = Paragraph::new(Line::from(Span::raw(status_text))).block(
        Block::default()
            .borders(Borders::ALL)
            .title("💰 Invoice Calculator"),
    );

    f.render_widget(status, chunks[0]);

    let help_text = match editor.mode() {
        AppMode::Nor => {
            "💰 COMPUTED FIELDS DEMO: Real-time invoice calculations!\n\
             🔢 EDITABLE: Product, Quantity, Unit Price, Tax Rate, Notes\n\
             📊 COMPUTED: Subtotal, Tax Amount, Total (calculated automatically)\n\
             \n\
             🚀 START: Press 'i' to edit Quantity, type '5', Tab to Unit Price, type '19.99'\n\
             Watch Subtotal and Total appear! Add Tax Rate to see tax calculations.\n\
             Navigation: Tab/Shift+Tab skips computed fields automatically"
        }
        AppMode::Ins => {
            "✏️ EDIT MODE: Type numbers to see calculations appear!\n\
             \n\
             💡 EXAMPLE: Type '5' in Quantity, then Tab to Unit Price and type '19.99'\n\
             • Subtotal appears: $99.95\n\
             • Total appears: $99.95\n\
             • Add Tax Rate (like '10') to see tax: $9.99, Total: $109.94\n\
             \n\
             Esc=normal, Tab=next field (auto-skips computed fields)"
        }
        _ => "💰 Invoice Calculator with Computed Fields",
    };

    let help_style = if editor.is_computed_field(editor.current_field()) {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::ITALIC)
    } else {
        Style::default().fg(Color::Gray)
    };

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("🚀 Try It Now!"),
        )
        .style(help_style)
        .wrap(Wrap { trim: true });

    f.render_widget(help, chunks[1]);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("💰 Canvas Computed Fields Demo - Invoice Calculator");
    println!("✅ computed feature: ENABLED");
    println!("🚀 QUICK TEST:");
    println!("   1. Press 'i' to edit Quantity");
    println!("   2. Type '5' and press Tab");
    println!("   3. Type '19.99' in Unit Price");
    println!("   4. Watch Subtotal ($99.95) and Total ($99.95) appear!");
    println!();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let data = InvoiceData::new();
    let editor = ComputedFieldsEditor::new(data);

    let res = run_app(&mut terminal, editor);

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

    println!("💰 Demo completed! Computed fields should have updated in real-time!");
    Ok(())
}
