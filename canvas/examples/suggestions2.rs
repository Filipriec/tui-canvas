// examples/suggestions2.rs
//! Demonstrates automatic cursor management + INTELLIGENT SUGGESTIONS
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
use std::collections::HashMap;
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
            debug_message: "🎯 Automatic Cursor + Suggestions Demo - cursor-style feature enabled!".to_string(),
            command_buffer: String::new(),
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
        self.editor.current_text()
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

// Demo form data with interesting text for cursor demonstration + SUGGESTIONS
struct CursorDemoData {
    fields: Vec<(String, String)>,
}

impl CursorDemoData {
    fn new() -> Self {
        Self {
            fields: vec![
                ("👤 Name".to_string(), "John-Paul McDonald".to_string()),
                ("📧 Email".to_string(), "user@exa".to_string()), // Partial for suggestions demo
                ("📱 Phone".to_string(), "+1 (555) 123-4567".to_string()),
                ("🏢 Company".to_string(), "tech".to_string()), // Partial for suggestions demo  
                ("🏠 Address".to_string(), "123 Main St, Apt 4B".to_string()),
                ("💻 Skills".to_string(), "rust,py".to_string()), // Multi-tag demo
                ("📝 Notes".to_string(), "Watch the cursor change! Normal=█ Insert=| Visual=blinking█".to_string()),
                ("🎯 Cursor + Suggestions".to_string(), "Try: Email (user@exa), Company (tech), Skills (rust,py) + Tab for smart suggestions!".to_string()),
            ],
        }
    }
}

impl DataProvider for CursorDemoData {
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
        // Enable suggestions for email, company, and skills fields
        matches!(field_index, 1 | 3 | 5)
    }

    fn display_value(&self, _index: usize) -> Option<&str> {
        None
    }
}

// ===================================================================
// INTELLIGENT SUGGESTIONS PROVIDER - More logical behavior
// ===================================================================

struct PowerfulSuggestionsProvider {
    // Cache for performance (realistic behavior)
    email_cache: HashMap<String, Vec<SuggestionItem>>,
    company_cache: HashMap<String, Vec<SuggestionItem>>,
}

impl PowerfulSuggestionsProvider {
    fn new() -> Self {
        Self {
            email_cache: HashMap::new(),
            company_cache: HashMap::new(),
        }
    }
}

#[async_trait]
impl SuggestionsProvider for PowerfulSuggestionsProvider {
    async fn fetch_suggestions(&mut self, field_index: usize, query: &str) -> Result<Vec<SuggestionItem>> {
        // Minimum query length check (realistic behavior)
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }

        // Simulate realistic network delay based on field type
        let delay_ms = match field_index {
            1 => 150, // Email: fast local lookup
            3 => 300, // Company: API call
            5 => 100, // Skills: local data
            _ => 50,
        };
        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;

        // Route to appropriate suggestion logic
        match field_index {
            1 => self.get_email_suggestions(query).await,
            3 => self.get_company_suggestions(query).await, 
            5 => self.get_skills_suggestions(query).await,
            _ => Ok(Vec::new()),
        }
    }
}

impl PowerfulSuggestionsProvider {
    /// Smart email suggestions with domain completion and validation
    async fn get_email_suggestions(&mut self, query: &str) -> Result<Vec<SuggestionItem>> {
        // Check cache first (realistic performance optimization)
        if let Some(cached) = self.email_cache.get(query) {
            return Ok(cached.clone());
        }

        let suggestions = if let Some(at_pos) = query.find('@') {
            // Domain completion mode: "user@exa" → complete domains
            let username = &query[..at_pos];
            let domain_part = &query[at_pos + 1..];
            
            // Validate username (basic email validation)
            if username.is_empty() || username.len() > 64 {
                return Ok(Vec::new());
            }
            
            let domains = vec![
                ("example.com", "Example Domain - For testing"),
                ("gmail.com", "Gmail - Google's email service"),
                ("outlook.com", "Outlook - Microsoft email"),
                ("yahoo.com", "Yahoo Mail - Classic email"),
                ("hotmail.com", "Hotmail - Microsoft legacy"),
                ("company.com", "Company Email - Corporate"),
                ("university.edu", "University - Educational"),
                ("protonmail.com", "ProtonMail - Secure email"),
                ("icloud.com", "iCloud - Apple's email service"),
                ("fastmail.com", "FastMail - Premium email"),
            ];

            // Smart filtering: exact prefix match first, then contains
            let mut exact_matches = Vec::new();
            let mut fuzzy_matches = Vec::new();
            
            for (domain, desc) in domains {
                if domain.starts_with(domain_part) {
                    exact_matches.push((domain, desc));
                } else if domain.contains(domain_part) && !domain_part.is_empty() {
                    fuzzy_matches.push((domain, desc));
                }
            }
            
            // Combine results: exact matches first, then fuzzy
            let mut all_matches = exact_matches;
            all_matches.extend(fuzzy_matches);
            
            all_matches.into_iter()
                .take(8) // Limit results (realistic UX)
                .map(|(domain, desc)| {
                    let full_email = format!("{}@{}", username, domain);
                    SuggestionItem {
                        display_text: format!("{} - {}", full_email, desc),
                        value_to_store: full_email,
                    }
                })
                .collect()
        } else {
            // Username mode: "user" → suggest common email formats
            if query.len() >= 2 && query.len() <= 32 {
                vec![
                    SuggestionItem {
                        display_text: format!("{}@gmail.com - Most popular email", query),
                        value_to_store: format!("{}@gmail.com", query),
                    },
                    SuggestionItem {
                        display_text: format!("{}@outlook.com - Microsoft email", query),
                        value_to_store: format!("{}@outlook.com", query),
                    },
                    SuggestionItem {
                        display_text: format!("{}@company.com - Work email", query),
                        value_to_store: format!("{}@company.com", query),
                    },
                ]
            } else {
                Vec::new()
            }
        };

        // Cache the result (realistic performance)
        self.email_cache.insert(query.to_string(), suggestions.clone());
        Ok(suggestions)
    }

    /// Intelligent company suggestions with fuzzy matching and scoring
    async fn get_company_suggestions(&mut self, query: &str) -> Result<Vec<SuggestionItem>> {
        // Check cache first
        if let Some(cached) = self.company_cache.get(query) {
            return Ok(cached.clone());
        }

        let companies = vec![
            ("Google", "Technology", "Search, Cloud, AI", 10),
            ("Microsoft", "Technology", "Software, Cloud, Gaming", 10),
            ("Apple", "Technology", "Consumer Electronics", 10),
            ("Amazon", "E-commerce", "Cloud, Retail, Logistics", 10),
            ("Meta", "Social Media", "Facebook, Instagram, WhatsApp", 9),
            ("Tesla", "Automotive", "Electric Vehicles, Energy", 8),
            ("Netflix", "Entertainment", "Streaming, Content", 8),
            ("SpaceX", "Aerospace", "Space Exploration", 7),
            ("Uber", "Transportation", "Ride-sharing, Delivery", 7),
            ("Airbnb", "Hospitality", "Home Sharing", 7),
            ("Stripe", "Fintech", "Payment Processing", 8),
            ("Shopify", "E-commerce", "Online Store Platform", 7),
            ("Slack", "Productivity", "Team Communication", 6),
            ("Zoom", "Communication", "Video Conferencing", 6),
            ("Figma", "Design", "Collaborative Design Tools", 5),
            ("Tech Startup", "Technology", "Early Stage Company", 3),
            ("Tech Corporation", "Technology", "Large Enterprise", 4),
            ("Technology Solutions", "Technology", "Custom Software", 3),
        ];

        let query_lower = query.to_lowercase();
        
        // Smart scoring algorithm
        let mut scored_companies: Vec<_> = companies.iter()
            .filter_map(|(name, category, desc, popularity)| {
                let name_lower = name.to_lowercase();
                let category_lower = category.to_lowercase();
                let desc_lower = desc.to_lowercase();
                
                let mut score = 0;
                
                // Exact name prefix (highest score)
                if name_lower.starts_with(&query_lower) {
                    score += 100;
                }
                // Name contains query
                else if name_lower.contains(&query_lower) {
                    score += 50;
                }
                // Category match
                else if category_lower.contains(&query_lower) {
                    score += 30;
                }
                // Description match
                else if desc_lower.contains(&query_lower) {
                    score += 20;
                }
                // Fuzzy match (simple version)
                else if fuzzy_match_simple(&name_lower, &query_lower) {
                    score += 10;
                }
                
                if score > 0 {
                    score += popularity; // Add popularity bonus
                    Some((name, category, desc, score))
                } else {
                    None
                }
            })
            .collect();
        
        // Sort by score (highest first)
        scored_companies.sort_by(|a, b| b.3.cmp(&a.3));
        
        let suggestions: Vec<SuggestionItem> = scored_companies.into_iter()
            .take(10) // Limit results
            .map(|(name, category, desc, _score)| SuggestionItem {
                display_text: format!("{} - {} • {}", name, category, desc),
                value_to_store: name.to_string(),
            })
            .collect();

        // Cache the result
        self.company_cache.insert(query.to_string(), suggestions.clone());
        Ok(suggestions)
    }

    /// Smart multi-tag skills suggestions with context awareness
    async fn get_skills_suggestions(&mut self, query: &str) -> Result<Vec<SuggestionItem>> {
        let skills_db = vec![
            // Programming Languages
            ("Rust", "Language", "Systems programming"),
            ("Python", "Language", "Versatile, data science"),
            ("JavaScript", "Language", "Web development"),
            ("TypeScript", "Language", "Typed JavaScript"),
            ("Go", "Language", "Cloud, microservices"),
            ("Java", "Language", "Enterprise, Android"),
            ("C++", "Language", "Performance critical"),
            ("C#", "Language", "Microsoft ecosystem"),
            ("Swift", "Language", "iOS development"),
            ("Kotlin", "Language", "Android, JVM"),
            
            // Frontend Frameworks
            ("React", "Frontend", "Popular UI library"),
            ("Vue", "Frontend", "Progressive framework"),
            ("Angular", "Frontend", "Full framework"),
            ("Svelte", "Frontend", "Compile-time framework"),
            
            // Backend Technologies
            ("Node.js", "Backend", "JavaScript runtime"),
            ("Express", "Backend", "Web framework"),
            ("Django", "Backend", "Python web framework"),
            ("Flask", "Backend", "Micro web framework"),
            ("Rails", "Backend", "Ruby web framework"),
            
            // Databases
            ("PostgreSQL", "Database", "Advanced SQL database"),
            ("MongoDB", "Database", "Document database"),
            ("Redis", "Database", "In-memory cache"),
            ("MySQL", "Database", "Popular SQL database"),
            
            // DevOps & Cloud
            ("Docker", "DevOps", "Containerization"),
            ("Kubernetes", "DevOps", "Container orchestration"),
            ("AWS", "Cloud", "Amazon Web Services"),
            ("GCP", "Cloud", "Google Cloud Platform"),
            ("Azure", "Cloud", "Microsoft cloud"),
            
            // Tools & Methodologies
            ("Git", "Tool", "Version control"),
            ("Linux", "Tool", "Operating system"),
            ("DevOps", "Methodology", "Development operations"),
            ("CI/CD", "Methodology", "Continuous integration"),
            ("TDD", "Methodology", "Test-driven development"),
            ("Agile", "Methodology", "Project management"),
            ("Scrum", "Methodology", "Agile framework"),
            
            // Emerging Tech
            ("Machine Learning", "AI", "Predictive models"),
            ("Data Science", "Analytics", "Data analysis"),
            ("Blockchain", "Distributed", "Decentralized systems"),
            ("WebAssembly", "Performance", "High-performance web"),
        ];

        // Parse existing tags to avoid duplicates
        let current_tags: Vec<&str> = query.split(',').map(|s| s.trim()).collect();
        let last_tag = current_tags.last().unwrap_or(&"").to_lowercase();
        
        // Don't suggest if last tag is empty
        if last_tag.is_empty() {
            return Ok(Vec::new());
        }

        // Find matching skills
        let mut matching_skills: Vec<_> = skills_db.iter()
            .filter(|(skill, _, _)| {
                let skill_lower = skill.to_lowercase();
                // Check if skill matches and isn't already selected
                skill_lower.contains(&last_tag) && 
                !current_tags[..current_tags.len()-1].iter().any(|&existing| 
                    existing.eq_ignore_ascii_case(skill)
                )
            })
            .collect();

        // Sort by relevance (exact prefix first)
        matching_skills.sort_by(|a, b| {
            let a_exact = a.0.to_lowercase().starts_with(&last_tag);
            let b_exact = b.0.to_lowercase().starts_with(&last_tag);
            
            match (a_exact, b_exact) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.0.cmp(b.0),
            }
        });

        let suggestions = matching_skills.into_iter()
            .take(8) // Limit results
            .map(|(skill, category, desc)| {
                let mut new_tags = current_tags[..current_tags.len()-1].to_vec();
                new_tags.push(skill);
                let new_value = new_tags.join(", ");
                
                SuggestionItem {
                    display_text: format!("+ {} ({} • {})", skill, category, desc),
                    value_to_store: new_value,
                }
            })
            .collect();

        Ok(suggestions)
    }
}

/// Simple fuzzy matching: check if all characters of pattern appear in text in order
fn fuzzy_match_simple(text: &str, pattern: &str) -> bool {
    if pattern.is_empty() {
        return true;
    }
    
    let mut pattern_chars = pattern.chars();
    let mut current_char = pattern_chars.next();
    
    for text_char in text.chars() {
        if let Some(pattern_char) = current_char {
            if text_char == pattern_char {
                current_char = pattern_chars.next();
                if current_char.is_none() {
                    return true; // All pattern characters matched
                }
            }
        }
    }
    
    false
}

/// Automatic cursor management demonstration + SUGGESTIONS
/// Features the CursorManager directly to show it's working
async fn handle_key_press(
    key: KeyCode,
    modifiers: KeyModifiers,
    editor: &mut AutoCursorFormEditor<CursorDemoData>,
    suggestions_provider: &mut PowerfulSuggestionsProvider,
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
        // === SUGGESTIONS HANDLING (NEW!) ===
        
        // Tab: Trigger or navigate suggestions
        (_, KeyCode::Tab, _) => {
            if editor.is_suggestions_active() {
                editor.suggestions_next();
                editor.set_debug_message("📍 Next suggestion".to_string());
            } else if editor.data_provider().supports_suggestions(editor.current_field()) {
                match editor.trigger_suggestions(suggestions_provider).await {
                    Ok(_) => {
                        if editor.suggestions().is_empty() {
                            editor.set_debug_message("🔍 No suggestions found".to_string());
                        } else {
                            editor.set_debug_message(format!("✨ {} suggestions found", editor.suggestions().len()));
                        }
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
                    editor.set_debug_message(format!("✅ Applied: {}", applied));
                } else {
                    editor.set_debug_message("❌ No suggestion selected".to_string());
                }
            } else {
                editor.next_field();
                editor.set_debug_message("Enter: next field".to_string());
            }
        }

        // === MODE TRANSITIONS WITH AUTOMATIC CURSOR MANAGEMENT ===
        (AppMode::ReadOnly, KeyCode::Char('i'), _) => {
            editor.enter_edit_mode(); // 🎯 Automatic: cursor becomes bar |
            editor.clear_command_buffer();
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
        (AppMode::ReadOnly, KeyCode::Char('o'), _) => {
            editor.move_line_end();
            editor.enter_edit_mode(); // 🎯 Automatic: cursor becomes bar |
            editor.set_debug_message("✏️  INSERT (open line) - Cursor: Steady Bar |".to_string());
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

        // From Visual Mode: Switch between visual modes or exit
        (AppMode::Highlight, KeyCode::Char('v'), _) => {
            use canvas::canvas::state::SelectionState;
            match editor.editor.selection_state() {
                SelectionState::Characterwise { .. } => {
                    // Already in characterwise mode, exit visual mode (vim behavior)
                    editor.exit_visual_mode();
                    editor.set_debug_message("🔒 Exited visual mode".to_string());
                }
                _ => {
                    // Switch from linewise to characterwise mode
                    editor.editor.enter_highlight_mode();
                    editor.update_visual_selection();
                    editor.set_debug_message("🔥 Switched to VISUAL mode".to_string());
                }
            }
            editor.clear_command_buffer();
        }

        (AppMode::Highlight, KeyCode::Char('V'), _) => {
            use canvas::canvas::state::SelectionState;
            match editor.editor.selection_state() {
                SelectionState::Linewise { .. } => {
                    // Already in linewise mode, exit visual mode (vim behavior)
                    editor.exit_visual_mode();
                    editor.set_debug_message("🔒 Exited visual mode".to_string());
                }
                _ => {
                    // Switch from characterwise to linewise mode
                    editor.editor.enter_highlight_line_mode();
                    editor.update_visual_selection();
                    editor.set_debug_message("🔥 Switched to VISUAL LINE mode".to_string());
                }
            }
            editor.clear_command_buffer();
        }

        // Escape: Exit any mode back to normal (and cancel suggestions)
        (_, KeyCode::Esc, _) => {
            match mode {
                AppMode::Edit => {
                    editor.exit_edit_mode(); // Exit insert mode (suggestions auto-cancelled)
                }
                AppMode::Highlight => {
                    editor.exit_visual_mode(); // Exit visual mode
                }
                _ => {
                    // Already in normal mode, just clear command buffer
                    editor.clear_command_buffer();
                }
            }
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
            editor.set_debug_message("↓ next field".to_string());
            editor.clear_command_buffer();
        }
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('k'), _)
        | (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Up, _) => {
            editor.move_up();
            editor.set_debug_message("↑ previous field".to_string());
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

        // Field/document movement
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('g'), _) => {
            if editor.get_command_buffer() == "g" {
                editor.move_first_line();
                editor.set_debug_message("gg: first field".to_string());
                editor.clear_command_buffer();
            } else {
                editor.clear_command_buffer();
                editor.add_to_command_buffer('g');
                editor.set_debug_message("g".to_string());
            }
        }
        (AppMode::ReadOnly | AppMode::Highlight, KeyCode::Char('G'), _) => {
            editor.move_last_line();
            editor.set_debug_message("G: last field".to_string());
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
        }
        (AppMode::Edit, KeyCode::Delete, _) => {
            editor.delete_forward()?;
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

        // === CHARACTER INPUT ===
        (AppMode::Edit, KeyCode::Char(c), m) if !m.contains(KeyModifiers::CONTROL) => {
            editor.insert_char(c)?;
        }

        // === DEBUG/INFO COMMANDS ===
        (AppMode::ReadOnly, KeyCode::Char('?'), _) => {
            editor.set_debug_message(format!(
                "Field {}/{}, Pos {}, Mode: {:?} - Smart suggestions: Email domains, Company scoring, Skills multi-tag",
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
                editor.set_debug_message(format!(
                    "Unhandled: {:?} + {:?} in {:?} mode",
                    key, modifiers, mode
                ));
            }
        }
    }

    Ok(true)
}

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut editor: AutoCursorFormEditor<CursorDemoData>,
) -> io::Result<()> {
    let mut suggestions_provider = PowerfulSuggestionsProvider::new();

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

fn ui(f: &mut Frame, editor: &AutoCursorFormEditor<CursorDemoData>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(10)])
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
    editor: &AutoCursorFormEditor<CursorDemoData>,
) -> Option<ratatui::layout::Rect> {
    render_canvas_default(f, area, &editor.editor)
}

fn render_status_and_help(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    editor: &AutoCursorFormEditor<CursorDemoData>,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(7)])
        .split(area);

    // Status bar with cursor information + suggestions
    let mode_text = match editor.mode() {
        AppMode::Edit => "INSERT | (bar cursor)",
        AppMode::ReadOnly => "NORMAL █ (block cursor)",
        AppMode::Highlight => {
            // Use library selection state instead of editor.highlight_state()
            use canvas::canvas::state::SelectionState;
            match editor.editor.selection_state() {
                SelectionState::Characterwise { .. } => "VISUAL █ (blinking block)",
                SelectionState::Linewise { .. } => "VISUAL LINE █ (blinking block)",
                _ => "VISUAL █ (blinking block)",
            }
        },
        _ => "NORMAL █ (block cursor)",
    };

    let suggestions_info = if editor.is_suggestions_active() {
        if editor.editor.ui_state().is_suggestions_loading() {
            " | ⏳ Loading suggestions..."
        } else if !editor.suggestions().is_empty() {
            " | ✨ Suggestions available"
        } else {
            " | 🔍 No suggestions"
        }
    } else {
        ""
    };

    let status_text = if editor.has_pending_command() {
        format!("-- {} -- {} [{}]{}", mode_text, editor.debug_message(), editor.get_command_buffer(), suggestions_info)
    } else if editor.has_unsaved_changes() {
        format!("-- {} -- [Modified] {}{}", mode_text, editor.debug_message(), suggestions_info)
    } else {
        format!("-- {} -- {}{}", mode_text, editor.debug_message(), suggestions_info)
    };

    let status = Paragraph::new(Line::from(Span::raw(status_text)))
        .block(Block::default().borders(Borders::ALL).title("🎯 Smart Cursor + Intelligent Suggestions"));

    f.render_widget(status, chunks[0]);

    // Enhanced help text with suggestions info
    let help_text = match editor.mode() {
        AppMode::ReadOnly => {
            if editor.has_pending_command() {
                match editor.get_command_buffer() {
                    "g" => "Press 'g' again for first field, or any other key to cancel",
                    _ => "Pending command... (Esc to cancel)"
                }
            } else {
                "🎯 SMART SUGGESTIONS DEMO: Normal █ | Insert | | Visual blinking█\n\
                 Normal: hjkl/arrows=move, w/b/e=words, 0/$=line, gg/G=first/last\n\
                 i/a/A=insert, v/V=visual, x/X=delete, ?=info\n\
                 Tab=smart suggestions: Email domains, Company scoring, Multi-tag skills\n\
                 Features: caching, fuzzy matching, validation, scoring | F1/F2=cursor demo"
            }
        }
        AppMode::Edit => {
            "🎯 INSERT MODE - Cursor: | (bar)\n\
             arrows=move, Ctrl+arrows=words, Backspace/Del=delete\n\
             Tab=smart suggestions: domains, companies, skills with fuzzy matching\n\
             Enter=apply suggestion, Esc=normal mode"
        }
        AppMode::Highlight => {
            "🎯 VISUAL MODE - Cursor: █ (blinking block)\n\
             hjkl/arrows=extend selection, w/b/e=word selection\n\
             Esc=normal"
        }
        _ => "🎯 Watch the cursor change automatically! Tab for intelligent suggestions on certain fields!"
    };

    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("🚀 Smart Cursor + Intelligent Suggestions"))
        .style(Style::default().fg(Color::Gray));

    f.render_widget(help, chunks[1]);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Print feature status
    println!("🎯 Smart Cursor + Intelligent Suggestions Demo");
    println!("✅ cursor-style feature: ENABLED");
    println!("✅ suggestions feature: ENABLED");
    println!("🚀 Automatic cursor management: ACTIVE");
    println!("✨ Intelligent suggestions: ACTIVE");
    println!("📖 Watch your terminal cursor change based on mode!");
    println!();
    println!("🎯 Try these logical suggestions:");
    println!("   📧 Email field: 'user@exa' + Tab → smart domain completion");
    println!("   🏢 Company field: 'tech' + Tab → scored company matches");
    println!("   💻 Skills field: 'rust,py' + Tab → intelligent multi-tag skills");
    println!("   🧠 Features: caching, fuzzy matching, scoring, validation");
    println!();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let data = CursorDemoData::new();
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

    println!("🎯 Cursor automatically reset to default! Smart suggestions cache cleared!");
    Ok(())
}
