// examples/suggestions2.rs
//! Demonstrates automatic cursor management + MULTIPLE SUGGESTION FIELDS
//!
//! This example REQUIRES the `cursor-style` feature to compile.
//!
//! Run with:
//! cargo run --example suggestions2 --features "gui,cursor-style,suggestions"

// REQUIRE cursor-style feature - example won't compile without it
#[cfg(not(feature = "cursor-style"))]
compile_error!(
    "This example requires the 'cursor-style' feature. \
     Run with: cargo run --example suggestions2 --features \"gui,cursor-style,suggestions\""
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
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};

use canvas::{
    canvas::{
        gui::render_canvas_default,
        modes::{AppMode, ModeManager, HighlightState},
        CursorManager, // This import only exists when cursor-style feature is enabled
    },
    suggestions::gui::render_suggestions_dropdown,
    DataProvider, FormEditor, SuggestionsProvider, SuggestionItem,
};

use async_trait::async_trait;
use anyhow::Result;

// Enhanced FormEditor that demonstrates automatic cursor management + SUGGESTIONS
struct AutoCursorFormEditor<D: DataProvider> {
    editor: FormEditor<D>,
    has_unsaved_changes: bool,
    debug_message: String,
    command_buffer: String, // For multi-key vim commands like "gg"
}

impl<D: DataProvider> AutoCursorFormEditor<D> {
    fn new(data_provider: D) -> Self {
        Self {
            editor: FormEditor::new(data_provider),
            has_unsaved_changes: false,
            debug_message: "🎯 Multi-Field Suggestions Demo - 5 fields with different suggestions!".to_string(),
            command_buffer: String::new(),
        }
    }

    fn close_suggestions(&mut self) {
        self.editor.close_suggestions();
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

    // === VISUAL/HIGHLIGHT MODE SUPPORT ===

    fn enter_visual_mode(&mut self) {
        // Use the library method instead of manual state setting
        self.editor.enter_highlight_mode();
        self.debug_message = "🔥 VISUAL MODE - Cursor: Blinking Block █".to_string();
    }

    fn enter_visual_line_mode(&mut self) {
        // Use the library method instead of manual state setting
        self.editor.enter_highlight_line_mode();
        self.debug_message = "🔥 VISUAL LINE MODE - Cursor: Blinking Block █".to_string();
    }

    fn exit_visual_mode(&mut self) {
        // Use the library method
        self.editor.exit_highlight_mode();
        self.debug_message = "🔒 NORMAL MODE - Cursor: Steady Block █".to_string();
    }

    fn update_visual_selection(&mut self) {
        if self.editor.is_highlight_mode() {
            use canvas::canvas::state::SelectionState;
            match self.editor.selection_state() {
                SelectionState::Characterwise { anchor } => {
                    self.debug_message = format!(
                        "🎯 Visual selection: anchor=({},{}) current=({},{}) - Cursor: Blinking Block █",
                        anchor.0, anchor.1,
                        self.editor.current_field(),
                        self.editor.cursor_position()
                    );
                }
                SelectionState::Linewise { anchor_field } => {
                    self.debug_message = format!(
                        "🎯 Visual LINE selection: anchor={} current={} - Cursor: Blinking Block █",
                        anchor_field,
                        self.editor.current_field()
                    );
                }
                _ => {}
            }
        }
    }

    // === ENHANCED MOVEMENT WITH VISUAL UPDATES ===

    fn move_left(&mut self) {
        self.editor.move_left();
        self.update_visual_selection();
    }

    fn move_right(&mut self) {
        self.editor.move_right();
        self.update_visual_selection();
    }

    fn move_up(&mut self) {
        let _ = self.editor.move_up();
        self.update_visual_selection();
    }

    fn move_down(&mut self) {
        let _ = self.editor.move_down();
        self.update_visual_selection();
    }

    fn move_word_next(&mut self) {
        self.editor.move_word_next();
        self.update_visual_selection();
    }

    fn move_word_prev(&mut self) {
        self.editor.move_word_prev();
        self.update_visual_selection();
    }

    fn move_word_end(&mut self) {
        self.editor.move_word_end();
        self.update_visual_selection();
    }

    fn move_word_end_prev(&mut self) {
        self.editor.move_word_end_prev();
        self.update_visual_selection();
    }

    fn move_line_start(&mut self) {
        self.editor.move_line_start();
        self.update_visual_selection();
    }

    fn move_line_end(&mut self) {
        self.editor.move_line_end();
        self.update_visual_selection();
    }

    fn move_first_line(&mut self) {
        self.editor.move_first_line();
        self.update_visual_selection();
    }

    fn move_last_line(&mut self) {
        self.editor.move_last_line();
        self.update_visual_selection();
    }

    fn prev_field(&mut self) {
        let _ = self.editor.prev_field();
        self.update_visual_selection();
    }

    fn next_field(&mut self) {
        let _ = self.editor.next_field();
        self.update_visual_selection();
    }

    // === DELETE OPERATIONS ===

    fn delete_backward(&mut self) -> anyhow::Result<()> {
        let result = self.editor.delete_backward();
        if result.is_ok() {
            self.has_unsaved_changes = true;
            self.debug_message = "⌫ Deleted character backward".to_string();
        }
        Ok(result?)
    }

    fn delete_forward(&mut self) -> anyhow::Result<()> {
        let result = self.editor.delete_forward();
        if result.is_ok() {
            self.has_unsaved_changes = true;
            self.debug_message = "⌦ Deleted character forward".to_string();
        }
        Ok(result?)
    }

    // === SUGGESTIONS CONTROL WRAPPERS ===

    fn open_suggestions(&mut self, field_index: usize) {
        self.editor.open_suggestions(field_index);
    }

    // === MODE TRANSITIONS WITH AUTOMATIC CURSOR MANAGEMENT ===

    fn enter_edit_mode(&mut self) {
        self.editor.enter_edit_mode(); // 🎯 Library automatically sets cursor to bar |
        self.debug_message = "✏️  INSERT MODE - Cursor: Steady Bar |".to_string();
    }

    fn enter_append_mode(&mut self) {
        self.editor.enter_append_mode(); // 🎯 Library automatically positions cursor and sets mode
        self.debug_message = "✏️  INSERT (append) - Cursor: Steady Bar |".to_string();
    }

    fn exit_edit_mode(&mut self) {
        let _ = self.editor.exit_edit_mode(); // 🎯 Library automatically sets cursor to block █
        self.exit_visual_mode();
        self.debug_message = "🔒 NORMAL MODE - Cursor: Steady Block █".to_string();
    }

    fn insert_char(&mut self, ch: char) -> anyhow::Result<()> {
        let result = self.editor.insert_char(ch);
        if result.is_ok() {
            self.has_unsaved_changes = true;
        }
        Ok(result?)
    }

    // === SUGGESTIONS SUPPORT ===

    async fn trigger_suggestions<A>(&mut self, provider: &mut A) -> anyhow::Result<()>
    where
        A: SuggestionsProvider,
    {
        self.editor.trigger_suggestions(provider).await
    }

    fn suggestions_next(&mut self) {
        self.editor.suggestions_next();
    }

    fn apply_suggestion(&mut self) -> Option<String> {
        self.editor.apply_suggestion()
    }

    fn is_suggestions_active(&self) -> bool {
        self.editor.is_suggestions_active()
    }

    fn suggestions(&self) -> &[SuggestionItem] {
        self.editor.suggestions()
    }

    pub fn update_inline_completion(&mut self) {
        self.editor.update_inline_completion();
    }

    // === MANUAL CURSOR OVERRIDE DEMONSTRATION ===

    /// Demonstrate manual cursor control (for advanced users)
    fn demo_manual_cursor_control(&mut self) -> std::io::Result<()> {
        // Users can still manually control cursor if needed
        CursorManager::update_for_mode(AppMode::Command)?;
        self.debug_message = "🔧 Manual override: Command cursor _".to_string();
        Ok(())
    }

    fn restore_automatic_cursor(&mut self) -> std::io::Result<()> {
        // Restore automatic cursor based on current mode
        CursorManager::update_for_mode(self.editor.mode())?;
        self.debug_message = "🎯 Restored automatic cursor management".to_string();
        Ok(())
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
        let field_index = self.editor.current_field();
        self.editor.data_provider().field_value(field_index)
    }

    fn data_provider(&self) -> &D {
        self.editor.data_provider()
    }

    fn ui_state(&self) -> &canvas::EditorState {
        self.editor.ui_state()
    }

    fn set_mode(&mut self, mode: AppMode) {
        self.editor.set_mode(mode); // 🎯 Library automatically updates cursor
        if mode != AppMode::Highlight {
            self.exit_visual_mode();
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

// ===================================================================
// MULTI-FIELD DEMO DATA - 5 different types of suggestion fields
// ===================================================================

struct MultiFieldDemoData {
    fields: Vec<(String, String)>,
}

impl MultiFieldDemoData {
    fn new() -> Self {
        Self {
            fields: vec![
                ("🍎 Favorite Fruit".to_string(), "".to_string()),           // Field 0: Fruits
                ("💼 Job Role".to_string(), "".to_string()),                 // Field 1: Jobs
                ("💻 Programming Language".to_string(), "".to_string()),     // Field 2: Languages
                ("🌍 Country".to_string(), "".to_string()),                  // Field 3: Countries
                ("🎨 Favorite Color".to_string(), "".to_string()),           // Field 4: Colors
            ],
        }
    }
}

impl DataProvider for MultiFieldDemoData {
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

    fn supports_suggestions(&self, field_index: usize) -> bool {
        // All 5 fields support suggestions - perfect for testing!
        field_index < 5
    }

    fn display_value(&self, _index: usize) -> Option<&str> {
        None
    }
}

// ===================================================================
// COMPREHENSIVE SUGGESTIONS PROVIDER - 5 different suggestion types!
// ===================================================================

struct ComprehensiveSuggestionsProvider;

impl ComprehensiveSuggestionsProvider {
    fn new() -> Self {
        Self
    }

    /// Get fruit suggestions (field 0)
    fn get_fruit_suggestions(&self, query: &str) -> Vec<SuggestionItem> {
        let fruits = vec![
            ("Apple", "🍎 Crisp and sweet"),
            ("Banana", "🍌 Rich in potassium"),
            ("Cherry", "🍒 Small and tart"),
            ("Date", "📅 Sweet and chewy"),
            ("Elderberry", "🫐 Dark purple berry"),
            ("Fig", "🍇 Sweet Mediterranean fruit"),
            ("Grape", "🍇 Perfect for wine"),
            ("Honeydew", "🍈 Sweet melon"),
            ("Ananas", "🍎 Crisp and sweet"),
            ("Avocado", "🍈 Sweet melon"),
        ];

        self.filter_suggestions(fruits, query, "fruit")
    }

    /// Get job role suggestions (field 1)
    fn get_job_suggestions(&self, query: &str) -> Vec<SuggestionItem> {
        let jobs = vec![
            ("Software Engineer", "👨‍💻 Build applications"),
            ("Product Manager", "📋 Manage product roadmap"),
            ("Data Scientist", "📊 Analyze data patterns"),
            ("UX Designer", "🎨 Design user experiences"),
            ("DevOps Engineer", "⚙️ Manage infrastructure"),
            ("Marketing Manager", "📢 Drive growth"),
            ("Sales Representative", "💰 Generate revenue"),
            ("Accountant", "💼 Manage finances"),
        ];

        self.filter_suggestions(jobs, query, "role")
    }

    /// Get programming language suggestions (field 2)
    fn get_language_suggestions(&self, query: &str) -> Vec<SuggestionItem> {
        let languages = vec![
            ("Rust", "🦀 Systems programming"),
            ("Python", "🐍 Versatile and popular"),
            ("JavaScript", "⚡ Web development"),
            ("TypeScript", "🔷 Typed JavaScript"),
            ("Go", "🏃 Fast and simple"),
            ("Java", "☕ Enterprise favorite"),
            ("C++", "⚡ High performance"),
            ("Swift", "🍎 iOS development"),
        ];

        self.filter_suggestions(languages, query, "language")
    }

    /// Get country suggestions (field 3)
    fn get_country_suggestions(&self, query: &str) -> Vec<SuggestionItem> {
        let countries = vec![
            ("United States", "🇺🇸 North America"),
            ("Canada", "🇨🇦 Great neighbors"),
            ("United Kingdom", "🇬🇧 Tea and crumpets"),
            ("Germany", "🇩🇪 Engineering excellence"),
            ("France", "🇫🇷 Art and cuisine"),
            ("Japan", "🇯🇵 Technology hub"),
            ("Australia", "🇦🇺 Down under"),
            ("Brazil", "🇧🇷 Carnival country"),
        ];

        self.filter_suggestions(countries, query, "country")
    }

    /// Get color suggestions (field 4)
    fn get_color_suggestions(&self, query: &str) -> Vec<SuggestionItem> {
        let colors = vec![
            ("Red", "🔴 Bold and energetic"),
            ("Blue", "🔵 Calm and trustworthy"),
            ("Green", "🟢 Natural and fresh"),
            ("Yellow", "🟡 Bright and cheerful"),
            ("Purple", "🟣 Royal and mysterious"),
            ("Orange", "🟠 Warm and vibrant"),
            ("Pink", "🩷 Soft and gentle"),
            ("Black", "⚫ Classic and elegant"),
        ];

        self.filter_suggestions(colors, query, "color")
    }

    /// Generic filtering helper
    fn filter_suggestions(&self, items: Vec<(&str, &str)>, query: &str, _category: &str) -> Vec<SuggestionItem> {
        let query_lower = query.to_lowercase();

        items.iter()
            .filter(|(item, _)| {
                query.is_empty() || item.to_lowercase().starts_with(&query_lower)
            })
            .map(|(item, description)| SuggestionItem {
                display_text: format!("{} - {}", item, description),
                value_to_store: item.to_string(),
            })
            .collect()
    }
}

#[async_trait]
impl SuggestionsProvider for ComprehensiveSuggestionsProvider {
    async fn fetch_suggestions(&mut self, field_index: usize, query: &str) -> Result<Vec<SuggestionItem>> {
        // Simulate different network delays for different fields (realistic!)
        let delay_ms = match field_index {
            0 => 100, // Fruits: local data
            1 => 200, // Jobs: medium API call
            2 => 150, // Languages: cached data
            3 => 300, // Countries: slow geographic API
            4 => 80,  // Colors: instant local
            _ => 100,
        };
        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;

        let suggestions = match field_index {
            0 => self.get_fruit_suggestions(query),
            1 => self.get_job_suggestions(query),
            2 => self.get_language_suggestions(query),
            3 => self.get_country_suggestions(query),
            4 => self.get_color_suggestions(query),
            _ => Vec::new(),
        };

        Ok(suggestions)
    }
}

/// Multi-field suggestions demonstration + automatic cursor management
async fn handle_key_press(
    key: KeyCode,
    modifiers: KeyModifiers,
    editor: &mut AutoCursorFormEditor<MultiFieldDemoData>,
    suggestions_provider: &mut ComprehensiveSuggestionsProvider,
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
        // === SUGGESTIONS HANDLING ===
        (_, KeyCode::Tab, _) => {
            if editor.is_suggestions_active() {
                // Cycle through suggestions
                editor.suggestions_next();
                editor.set_debug_message("📍 Next suggestion".to_string());
            } else if editor.data_provider().supports_suggestions(editor.current_field()) {
                // Open suggestions explicitly
                editor.open_suggestions(editor.current_field());
                match editor.trigger_suggestions(suggestions_provider).await {
                    Ok(_) => {
                        editor.update_inline_completion();
                        editor.set_debug_message(format!(
                            "✨ {} suggestions loaded",
                            editor.suggestions().len()
                        ));
                    }
                    Err(e) => {
                        editor.set_debug_message(format!("❌ Suggestion error: {}", e));
                    }
                }
            } else {
                editor.next_field();
                editor.set_debug_message("Tab: next field".to_string());
            }
        }

        // Enter: Apply suggestion or move to next field
        (_, KeyCode::Enter, _) => {
            if editor.is_suggestions_active() {
                if let Some(applied) = editor.apply_suggestion() {
                    editor.set_debug_message(format!("✅ Selected: {}", applied));
                } else {
                    editor.set_debug_message("❌ No suggestion selected".to_string());
                }
            } else {
                editor.next_field();
                let field_names = ["Fruit", "Job", "Language", "Country", "Color"];
                let field_name = field_names.get(editor.current_field()).unwrap_or(&"Field");
                editor.set_debug_message(format!("Enter: moved to {} field", field_name));
            }
        }

        // Escape: Close suggestions or exit mode
        (_, KeyCode::Esc, _) => {
            if editor.is_suggestions_active() {
                editor.close_suggestions();
                editor.set_debug_message("❌ Suggestions closed".to_string());
            } else {
                match mode {
                    AppMode::Edit => {
                        editor.exit_edit_mode();
                    }
                    AppMode::Highlight => {
                        editor.exit_visual_mode();
                    }
                    _ => {
                        editor.clear_command_buffer();
                    }
                }
            }
        }

        // === MODE TRANSITIONS WITH AUTOMATIC CURSOR MANAGEMENT ===
        (AppMode::ReadOnly, KeyCode::Char('i'), _) => {
            editor.enter_edit_mode(); // 🎯 Automatic: cursor becomes bar |
            editor.clear_command_buffer();

            // Auto-show suggestions on entering insert mode
            if editor.data_provider().supports_suggestions(editor.current_field()) {
                let _ = editor.trigger_suggestions(suggestions_provider).await;
                editor.update_inline_completion();
            }
        }
        (AppMode::ReadOnly, KeyCode::Char('a'), _) => {
            editor.enter_append_mode();
            editor.set_debug_message("✏️  INSERT (append) - Cursor: Steady Bar |".to_string());
            editor.clear_command_buffer();
        }
        (AppMode::ReadOnly, KeyCode::Char('A'), _) => {
            editor.move_line_end();
            editor.enter_edit_mode(); // 🎯 Automatic: cursor becomes bar |
            editor.set_debug_message("✏️  INSERT (end of line) - Cursor: Steady Bar |".to_string());
            editor.clear_command_buffer();
        }

        // From Normal Mode: Enter visual modes
        (AppMode::ReadOnly, KeyCode::Char('v'), _) => {
            editor.enter_visual_mode();
            editor.clear_command_buffer();
        }
        (AppMode::ReadOnly, KeyCode::Char('V'), _) => {
            editor.enter_visual_line_mode();
            editor.clear_command_buffer();
        }

        // === CURSOR MANAGEMENT DEMONSTRATION ===
        (AppMode::ReadOnly, KeyCode::F(1), _) => {
            editor.demo_manual_cursor_control()?;
        }
        (AppMode::ReadOnly, KeyCode::F(2), _) => {
            editor.restore_automatic_cursor()?;
        }

        // === MOVEMENT: VIM-STYLE NAVIGATION ===

        // Basic movement (hjkl and arrows)
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('h'), _)
        | (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Left, _) => {
            editor.move_left();
            editor.set_debug_message("← left".to_string());
            editor.clear_command_buffer();
        }
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('l'), _)
        | (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Right, _) => {
            editor.move_right();
            editor.set_debug_message("→ right".to_string());
            editor.clear_command_buffer();
        }
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('j'), _)
        | (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Down, _) => {
            editor.move_down();
            let field_names = ["Fruit", "Job", "Language", "Country", "Color"];
            let field_name = field_names.get(editor.current_field()).unwrap_or(&"Field");
            editor.set_debug_message(format!("↓ moved to {} field", field_name));
            editor.clear_command_buffer();
        }
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('k'), _)
        | (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Up, _) => {
            editor.move_up();
            let field_names = ["Fruit", "Job", "Language", "Country", "Color"];
            let field_name = field_names.get(editor.current_field()).unwrap_or(&"Field");
            editor.set_debug_message(format!("↑ moved to {} field", field_name));
            editor.clear_command_buffer();
        }

        // Word movement
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('w'), _) => {
            editor.move_word_next();
            editor.set_debug_message("w: next word start".to_string());
            editor.clear_command_buffer();
        }
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('b'), _) => {
            editor.move_word_prev();
            editor.set_debug_message("b: previous word start".to_string());
            editor.clear_command_buffer();
        }
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('e'), _) => {
            editor.move_word_end();
            editor.set_debug_message("e: word end".to_string());
            editor.clear_command_buffer();
        }

        // Line movement
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('0'), _)
        | (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Home, _) => {
            editor.move_line_start();
            editor.set_debug_message("0: line start".to_string());
        }
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('$'), _)
        | (AppMode::ReadOnly | AppMode::Highlight, KeyCode::End, _) => {
            editor.move_line_end();
            editor.set_debug_message("$: line end".to_string());
        }

        // Document movement
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('g'), _) => {
            if editor.get_command_buffer() == "g" {
                editor.move_first_line();
                editor.set_debug_message("gg: first field (Fruit)".to_string());
                editor.clear_command_buffer();
            } else {
                editor.clear_command_buffer();
                editor.add_to_command_buffer('g');
                editor.set_debug_message("g".to_string());
            }
        }
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('G'), _) => {
            editor.move_last_line();
            editor.set_debug_message("G: last field (Color)".to_string());
            editor.clear_command_buffer();
        }

        // === EDIT MODE MOVEMENT ===
        (AppMode::Edit, KeyCode::Left, m) if m.contains(KeyModifiers::CONTROL) => {
            editor.move_word_prev();
            editor.set_debug_message("Ctrl+← word back".to_string());
        }
        (AppMode::Edit, KeyCode::Right, m) if m.contains(KeyModifiers::CONTROL) => {
            editor.move_word_next();
            editor.set_debug_message("Ctrl+→ word forward".to_string());
        }
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
        (AppMode::Edit, KeyCode::Home, _) => {
            editor.move_line_start();
        }
        (AppMode::Edit, KeyCode::End, _) => {
            editor.move_line_end();
        }

        // === DELETE OPERATIONS ===
        (AppMode::Edit, KeyCode::Backspace, _) => {
            editor.delete_backward()?;

            // Update suggestions after deletion
            if editor.data_provider().supports_suggestions(editor.current_field()) {
                let current_text = editor.current_text().to_string();
                if current_text.is_empty() {
                    let _ = editor.trigger_suggestions(suggestions_provider).await;
                    editor.set_debug_message(format!("✨ {} total suggestions", editor.suggestions().len()));
                    editor.update_inline_completion();
                } else {
                    match editor.trigger_suggestions(suggestions_provider).await {
                        Ok(_) => {
                            if editor.suggestions().is_empty() {
                                editor.set_debug_message(format!("🔍 No matches for '{}'", current_text));
                            } else {
                                editor.set_debug_message(format!("✨ {} matches for '{}'", editor.suggestions().len(), current_text));
                            }
                        }
                        Err(e) => {
                            editor.set_debug_message(format!("❌ Suggestion error: {}", e));
                        }
                    }
                    editor.update_inline_completion();
                }
            }
        }
        (AppMode::Edit, KeyCode::Delete, _) => {
            editor.delete_forward()?;

            // Update suggestions after deletion
            if editor.data_provider().supports_suggestions(editor.current_field()) {
                let current_text = editor.current_text().to_string();
                if current_text.is_empty() {
                    let _ = editor.trigger_suggestions(suggestions_provider).await;
                    editor.set_debug_message(format!("✨ {} total suggestions", editor.suggestions().len()));
                    editor.update_inline_completion();
                } else {
                    match editor.trigger_suggestions(suggestions_provider).await {
                        Ok(_) => {
                            if editor.suggestions().is_empty() {
                                editor.set_debug_message(format!("🔍 No matches for '{}'", current_text));
                            } else {
                                editor.set_debug_message(format!("✨ {} matches for '{}'", editor.suggestions().len(), current_text));
                            }
                        }
                        Err(e) => {
                            editor.set_debug_message(format!("❌ Suggestion error: {}", e));
                        }
                    }
                    editor.update_inline_completion();
                }
            }
        }

        // Delete operations in normal mode (vim x)
        (AppMode::ReadOnly, KeyCode::Char('x'), _) => {
            editor.delete_forward()?;
            editor.set_debug_message("x: deleted character".to_string());
        }
        (AppMode::ReadOnly, KeyCode::Char('X'), _) => {
            editor.delete_backward()?;
            editor.set_debug_message("X: deleted character backward".to_string());
        }

        // === CHARACTER INPUT WITH REAL-TIME SUGGESTIONS ===
        (AppMode::Edit, KeyCode::Char(c), m) if !m.contains(KeyModifiers::CONTROL) => {
            editor.insert_char(c)?;

            // Auto-trigger suggestions after typing
            if editor.data_provider().supports_suggestions(editor.current_field()) {
                match editor.trigger_suggestions(suggestions_provider).await {
                    Ok(_) => {
                        let current_text = editor.current_text().to_string();
                        if editor.suggestions().is_empty() {
                            editor.set_debug_message(format!("🔍 No matches for '{}'", current_text));
                        } else {
                            editor.set_debug_message(format!("✨ {} matches for '{}'", editor.suggestions().len(), current_text));
                        }
                        editor.update_inline_completion();
                    }
                    Err(e) => {
                        editor.set_debug_message(format!("❌ Suggestion error: {}", e));
                    }
                }
            }
        }

        // === DEBUG/INFO COMMANDS ===
        (AppMode::ReadOnly, KeyCode::Char('?'), _) => {
            let field_names = ["Fruit🍎", "Job💼", "Language💻", "Country🌍", "Color🎨"];
            let current_field_name = field_names.get(editor.current_field()).unwrap_or(&"Unknown");
            editor.set_debug_message(format!(
                "Field: {} ({}/{}), Pos: {}, Mode: {:?}",
                current_field_name,
                editor.current_field() + 1,
                editor.data_provider().field_count(),
                editor.cursor_position(),
                editor.mode()
            ));
        }

        _ => {
            if editor.has_pending_command() {
                editor.clear_command_buffer();
                editor.set_debug_message("Invalid command sequence".to_string());
            } else {
                let field_names = ["Fruit", "Job", "Language", "Country", "Color"];
                let current_field = field_names.get(editor.current_field()).unwrap_or(&"Field");
                editor.set_debug_message(format!(
                    "{} field - Try: i=insert, Tab=suggestions, j/k=move. Key: {:?}",
                    current_field, key
                ));
            }
        }
    }

    Ok(true)
}

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut editor: AutoCursorFormEditor<MultiFieldDemoData>,
) -> io::Result<()> {
    let mut suggestions_provider = ComprehensiveSuggestionsProvider::new();

    loop {
        terminal.draw(|f| ui(f, &editor))?;

        if let Event::Key(key) = event::read()? {
            match handle_key_press(key.code, key.modifiers, &mut editor, &mut suggestions_provider).await {
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

fn ui(f: &mut Frame, editor: &AutoCursorFormEditor<MultiFieldDemoData>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(12)])
        .split(f.area());

    let active_field_rect = render_enhanced_canvas(f, chunks[0], editor);

    // Render suggestions dropdown if active
    if let Some(input_rect) = active_field_rect {
        render_suggestions_dropdown(
            f,
            chunks[0],
            input_rect,
            &canvas::canvas::theme::DefaultCanvasTheme::default(),
            &editor.editor,
        );
    }

    render_status_and_help(f, chunks[1], editor);
}

fn render_enhanced_canvas(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    editor: &AutoCursorFormEditor<MultiFieldDemoData>,
) -> Option<ratatui::layout::Rect> {
    render_canvas_default(f, area, &editor.editor)
}

fn render_status_and_help(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    editor: &AutoCursorFormEditor<MultiFieldDemoData>,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(9)])
        .split(area);

    // Status bar with current field and cursor information
    let field_names = ["Fruit🍎", "Job💼", "Language💻", "Country🌍", "Color🎨"];
    let current_field_name = field_names.get(editor.current_field()).unwrap_or(&"Unknown");

    let mode_text = match editor.mode() {
        AppMode::Edit => "INSERT | (bar cursor)",
        AppMode::ReadOnly => "NORMAL █ (block cursor)",
        AppMode::Highlight => "VISUAL █ (blinking block)",
        _ => "NORMAL █ (block cursor)",
    };

    let suggestions_info = if editor.is_suggestions_active() {
        if editor.editor.ui_state().is_suggestions_loading() {
            " | ⏳ Loading suggestions...".to_string()
        } else if !editor.suggestions().is_empty() {
            format!(" | ✨ {} suggestions", editor.suggestions().len())
        } else {
            " | 🔍 No matches".to_string()
        }
    } else {
        "".to_string()
    };

    let status_text = format!(
        "-- {} -- {} | Field: {}{}",
        mode_text,
        editor.debug_message(),
        current_field_name,
        suggestions_info
    );

    let status = Paragraph::new(Line::from(Span::raw(status_text)))
        .block(Block::default().borders(Borders::ALL).title("🎯 Multi-Field Suggestions Demo"));

    f.render_widget(status, chunks[0]);

    // Comprehensive help text
    let help_text = match editor.mode() {
        AppMode::ReadOnly => {
            "🎯 MULTI-FIELD SUGGESTIONS DEMO: Normal █ | Insert | | Visual █\n\
             Movement: j/k or ↑↓=fields, h/l or ←→=chars, gg/G=first/last, w/b/e=words\n\
             Actions: i/a/A=insert, v/V=visual, x/X=delete, ?=info, Enter=next field\n\
             🍎 Fruits: Apple, Banana, Cherry... | 💼 Jobs: Engineer, Manager, Designer...\n\
             💻 Languages: Rust, Python, JS... | 🌍 Countries: USA, Canada, UK...\n\
             🎨 Colors: Red, Blue, Green... | Tab=suggestions, Enter=select\n\
             Edge cases to test: empty→suggestions, partial matches, field navigation!"
        }
        AppMode::Edit => {
            "🎯 INSERT MODE - Cursor: | (bar)\n\
             Type to filter suggestions! Tab=show/cycle, Enter=select, Esc=normal\n\
             Test cases: 'r'→Red/Rust, 's'→Software Engineer/Swift, 'c'→Canada/Cherry...\n\
             Navigation: arrows=move, Ctrl+arrows=words, Home/End=line edges\n\
             Try different fields for different suggestion behaviors and timing!"
        }
        AppMode::Highlight => {
            "🎯 VISUAL MODE - Cursor: █ (blinking block)\n\
             Selection: hjkl/arrows=extend, w/b/e=word selection, Esc=normal\n\
             Test multi-character selections across different suggestion field types!"
        }
        _ => "🎯 Multi-field suggestions! 5 fields × 8 suggestions each = lots of testing!"
    };

    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("🚀 Comprehensive Testing Guide"))
        .style(Style::default().fg(Color::Gray));

    f.render_widget(help, chunks[1]);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Print comprehensive demo information
    println!("🎯 Multi-Field Suggestions Demo - Perfect for Testing Edge Cases!");
    println!("✅ cursor-style feature: ENABLED");
    println!("✅ suggestions feature: ENABLED");
    println!("🚀 Automatic cursor management: ACTIVE");
    println!("✨ 5 different suggestion types: ACTIVE");
    println!();
    println!("📋 Test These 5 Fields:");
    println!("   🍎 Fruits: Apple, Banana, Cherry, Date, Elderberry, Fig, Grape, Honeydew");
    println!("   💼 Jobs: Software Engineer, Product Manager, Data Scientist, UX Designer...");
    println!("   💻 Languages: Rust, Python, JavaScript, TypeScript, Go, Java, C++, Swift");
    println!("   🌍 Countries: USA, Canada, UK, Germany, France, Japan, Australia, Brazil");
    println!("   🎨 Colors: Red, Blue, Green, Yellow, Purple, Orange, Pink, Black");
    println!();
    println!("🧪 Edge Cases to Test:");
    println!("   • Navigation between suggestion/non-suggestion fields");
    println!("   • Empty field → Tab → see all suggestions");
    println!("   • Partial typing → Tab → filtered suggestions");
    println!("   • Different loading times per field (100-300ms)");
    println!("   • Field switching while suggestions active");
    println!("   • Visual mode selections across suggestion fields");
    println!();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let data = MultiFieldDemoData::new();
    let mut editor = AutoCursorFormEditor::new(data);

    // Initialize with normal mode - library automatically sets block cursor
    editor.set_mode(AppMode::ReadOnly);

    // Demonstrate that CursorManager is available and working
    CursorManager::update_for_mode(AppMode::ReadOnly)?;

    let res = run_app(&mut terminal, editor).await;

    // Library automatically resets cursor on FormEditor::drop()
    // But we can also manually reset if needed
    CursorManager::reset()?;

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

    println!("🎯 Multi-field testing complete! Great for finding edge cases!");
    Ok(())
}
