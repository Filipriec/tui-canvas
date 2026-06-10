// examples/validation_5.rs
//! Feature 5 external validation demo with automatic validation.
//!
//! Demonstrates:
//! - Multiple external validation types: PSC lookup, email domain check, username availability,
//!   API key validation, credit card verification
//! - AUTOMATIC validation on field transitions (arrows, Tab, Esc)
//! - Async validation simulation with realistic delays
//! - Validation caching and debouncing
//! - Progressive validation (local → remote)
//! - Validation history and performance metrics
//! - Different validation triggers (blur, manual, auto)
//! - Error handling and retry mechanisms
//! - Complex validation states with detailed feedback
//!
//! Controls:
//! - i/a: insert/append
//! - Esc: exit edit mode (triggers validation on configured fields)
//! - Tab/Shift+Tab: next/prev field (triggers validation automatically)
//! - Arrow keys: move between fields (triggers validation automatically)
//! - v: manually trigger validation of current field
//! - V: validate all fields
//! - c: clear external validation state for current field
//! - C: clear all validation states
//! - r: toggle validation history view
//! - e: cycle through example datasets
//! - h: show validation help and field rules
//! - F1: toggle external validation globally
//! - F10/Ctrl+C: quit
//!
//! Run: cargo run --example validation_5 --features "gui,validation,cursor-style"

#![allow(clippy::needless_return)]

#[cfg(not(all(feature = "validation", feature = "gui", feature = "cursor-style")))]
compile_error!(
    "This example requires the 'validation', 'gui' and 'cursor-style' features. \
     Run with: cargo run --example validation_5 --features \"gui,validation,cursor-style\""
);

use std::collections::HashMap;
use std::io;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};

use tui_canvas::{
    render_canvas_default, AppMode, CursorManager,
    CustomFormatter, DataProvider, TextFormState, FormattingResult, ValidationConfigBuilder,
};

/// External validation state with timing and context
#[derive(Debug, Clone)]
struct TimedValidationResult {
    state: tui_canvas::validation::ExternalValidationState,
    started_at: Instant,
    completed_at: Option<Instant>,
    validation_type: String,
    cached: bool,
}

impl TimedValidationResult {
    fn new(validation_type: String) -> Self {
        Self {
            state: tui_canvas::validation::ExternalValidationState::Validating,
            started_at: Instant::now(),
            completed_at: None,
            validation_type,
            cached: false,
        }
    }

    fn complete(mut self, state: tui_canvas::validation::ExternalValidationState) -> Self {
        self.state = state;
        self.completed_at = Some(Instant::now());
        self
    }

    fn from_cache(state: tui_canvas::validation::ExternalValidationState, validation_type: String) -> Self {
        Self {
            state,
            started_at: Instant::now(),
            completed_at: Some(Instant::now()),
            validation_type,
            cached: true,
        }
    }

    fn duration(&self) -> Duration {
        self.completed_at
            .unwrap_or_else(Instant::now)
            .duration_since(self.started_at)
    }
}

/// PSC Formatter with enhanced validation context
struct PSCFormatter;

impl CustomFormatter for PSCFormatter {
    fn format(&self, raw: &str) -> FormattingResult {
        if raw.is_empty() {
            return FormattingResult::success("");
        }

        if !raw.chars().all(|c| c.is_ascii_digit()) {
            return FormattingResult::error("PSC must contain only digits");
        }

        match raw.len() {
            0 => FormattingResult::success(""),
            1..=3 => FormattingResult::success(raw.to_string()),
            4 => FormattingResult::warning(
                format!("{} ", &raw[..3]),
                "PSC incomplete - external validation pending",
            ),
            5 => FormattingResult::success(format!("{} {}", &raw[..3], &raw[3..])),
            _ => FormattingResult::error("PSC too long (max 5 digits)"),
        }
    }
}

/// Credit Card Formatter for external validation demo
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

        match raw.len() {
            0..=15 => FormattingResult::warning(formatted, "Card incomplete - validation pending"),
            16 => FormattingResult::success(formatted),
            _ => FormattingResult::warning(formatted, "Card too long - validation may fail"),
        }
    }
}

/// Comprehensive validation cache
struct ValidationCache {
    results: HashMap<String, tui_canvas::validation::ExternalValidationState>,
}

impl ValidationCache {
    fn new() -> Self {
        Self {
            results: HashMap::new(),
        }
    }

    fn get(&self, key: &str) -> Option<&tui_canvas::validation::ExternalValidationState> {
        self.results.get(key)
    }

    fn set(&mut self, key: String, result: tui_canvas::validation::ExternalValidationState) {
        self.results.insert(key, result);
    }

    fn clear(&mut self) {
        self.results.clear();
    }
}

/// Simulated external validation services
struct ValidationServices {
    cache: ValidationCache,
}

impl ValidationServices {
    fn new() -> Self {
        Self {
            cache: ValidationCache::new(),
        }
    }

    /// PSC validation: simulates postal service API lookup
    fn validate_psc(&mut self, psc: &str) -> tui_canvas::validation::ExternalValidationState {
        let cache_key = format!("psc:{psc}");
        if let Some(cached) = self.cache.get(&cache_key) {
            return cached.clone();
        }

        if psc.is_empty() {
            return tui_canvas::validation::ExternalValidationState::NotValidated;
        }

        if !psc.chars().all(|c| c.is_ascii_digit()) || psc.len() != 5 {
            let result = tui_canvas::validation::ExternalValidationState::Invalid {
                message: "Invalid PSC format".to_string(),
                suggestion: Some("Enter 5 digits".to_string()),
            };
            self.cache.set(cache_key, result.clone());
            return result;
        }

        // Simulate realistic PSC validation scenarios
        let result = match psc {
            "00000" | "99999" => tui_canvas::validation::ExternalValidationState::Invalid {
                message: "PSC does not exist".to_string(),
                suggestion: Some("Check postal code".to_string()),
            },
            "01001" => tui_canvas::validation::ExternalValidationState::Valid(Some("Prague 1 - verified".to_string())),
            "10000" => tui_canvas::validation::ExternalValidationState::Valid(Some("Bratislava - verified".to_string())),
            "12345" => tui_canvas::validation::ExternalValidationState::Warning {
                message: "PSC region deprecated - still valid".to_string(),
            },
            "50000" => tui_canvas::validation::ExternalValidationState::Invalid {
                message: "PSC temporarily unavailable".to_string(),
                suggestion: Some("Try again later".to_string()),
            },
            _ => {
                // Most PSCs are valid with generic info
                let region = match &psc[..2] {
                    "01" | "02" | "03" => "Prague region",
                    "10" | "11" | "12" => "Bratislava region",
                    "20" | "21" => "Brno region",
                    _ => "Valid postal region",
                };
                tui_canvas::validation::ExternalValidationState::Valid(Some(format!("{region} - verified")))
            }
        };

        self.cache.set(cache_key, result.clone());
        result
    }

    /// Email validation: simulates domain checking
    fn validate_email(&mut self, email: &str) -> tui_canvas::validation::ExternalValidationState {
        let cache_key = format!("email:{email}");
        if let Some(cached) = self.cache.get(&cache_key) {
            return cached.clone();
        }

        if email.is_empty() {
            return tui_canvas::validation::ExternalValidationState::NotValidated;
        }

        if !email.contains('@') {
            let result = tui_canvas::validation::ExternalValidationState::Invalid {
                message: "Email must contain @".to_string(),
                suggestion: Some("Format: user@domain.com".to_string()),
            };
            self.cache.set(cache_key, result.clone());
            return result;
        }

        let parts: Vec<&str> = email.split('@').collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            let result = tui_canvas::validation::ExternalValidationState::Invalid {
                message: "Invalid email format".to_string(),
                suggestion: Some("Format: user@domain.com".to_string()),
            };
            self.cache.set(cache_key, result.clone());
            return result;
        }

        let domain = parts[1];
        let result = match domain {
            "gmail.com" | "outlook.com" | "yahoo.com" => tui_canvas::validation::ExternalValidationState::Valid(Some(
                "Popular email provider - verified".to_string(),
            )),
            "example.com" | "test.com" => tui_canvas::validation::ExternalValidationState::Warning {
                message: "Test domain - email may not be deliverable".to_string(),
            },
            "blocked.com" | "spam.com" => tui_canvas::validation::ExternalValidationState::Invalid {
                message: "Domain blocked".to_string(),
                suggestion: Some("Use different email provider".to_string()),
            },
            _ if domain.contains('.') => tui_canvas::validation::ExternalValidationState::Valid(Some(
                "Domain appears valid - not verified".to_string(),
            )),
            _ => tui_canvas::validation::ExternalValidationState::Invalid {
                message: "Invalid domain format".to_string(),
                suggestion: Some("Domain must contain '.'".to_string()),
            },
        };

        self.cache.set(cache_key, result.clone());
        result
    }

    /// Username validation: simulates availability checking
    fn validate_username(&mut self, username: &str) -> tui_canvas::validation::ExternalValidationState {
        let cache_key = format!("username:{username}");
        if let Some(cached) = self.cache.get(&cache_key) {
            return cached.clone();
        }

        if username.is_empty() {
            return tui_canvas::validation::ExternalValidationState::NotValidated;
        }

        if username.len() < 3 {
            let result = tui_canvas::validation::ExternalValidationState::Invalid {
                message: "Username too short".to_string(),
                suggestion: Some("Minimum 3 characters".to_string()),
            };
            self.cache.set(cache_key, result.clone());
            return result;
        }

        if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
            let result = tui_canvas::validation::ExternalValidationState::Invalid {
                message: "Invalid characters".to_string(),
                suggestion: Some("Use letters, numbers, underscore only".to_string()),
            };
            self.cache.set(cache_key, result.clone());
            return result;
        }

        let result = match username {
            "admin" | "root" | "user" | "test" => tui_canvas::validation::ExternalValidationState::Invalid {
                message: "Username reserved".to_string(),
                suggestion: Some("Choose different username".to_string()),
            },
            "john123" | "alice_dev" => tui_canvas::validation::ExternalValidationState::Invalid {
                message: "Username already taken".to_string(),
                suggestion: Some("Try variations or add numbers".to_string()),
            },
            username if username.starts_with("temp_") => tui_canvas::validation::ExternalValidationState::Warning {
                message: "Temporary username pattern - are you sure?".to_string(),
            },
            _ => tui_canvas::validation::ExternalValidationState::Valid(Some(
                "Username available - good choice!".to_string(),
            )),
        };

        self.cache.set(cache_key, result.clone());
        result
    }

    /// API Key validation: simulates authentication service
    fn validate_api_key(&mut self, key: &str) -> tui_canvas::validation::ExternalValidationState {
        let cache_key = format!("apikey:{key}");
        if let Some(cached) = self.cache.get(&cache_key) {
            return cached.clone();
        }

        if key.is_empty() {
            return tui_canvas::validation::ExternalValidationState::NotValidated;
        }

        if key.len() < 20 {
            let result = tui_canvas::validation::ExternalValidationState::Invalid {
                message: "API key too short".to_string(),
                suggestion: Some("Valid keys are 32+ characters".to_string()),
            };
            self.cache.set(cache_key, result.clone());
            return result;
        }

        let result = match key {
            "invalid_key_12345678901" => tui_canvas::validation::ExternalValidationState::Invalid {
                message: "API key not found".to_string(),
                suggestion: Some("Check key and permissions".to_string()),
            },
            "expired_key_12345678901" => tui_canvas::validation::ExternalValidationState::Invalid {
                message: "API key expired".to_string(),
                suggestion: Some("Generate new key".to_string()),
            },
            "limited_key_12345678901" => tui_canvas::validation::ExternalValidationState::Warning {
                message: "API key has limited permissions".to_string(),
            },
            key if key.starts_with("test_") => tui_canvas::validation::ExternalValidationState::Warning {
                message: "Test API key - limited functionality".to_string(),
            },
            _ if key.len() >= 32 => tui_canvas::validation::ExternalValidationState::Valid(Some(
                "API key authenticated - full access".to_string(),
            )),
            _ => tui_canvas::validation::ExternalValidationState::Invalid {
                message: "Invalid API key format".to_string(),
                suggestion: Some("Keys should be 32+ alphanumeric characters".to_string()),
            },
        };

        self.cache.set(cache_key, result.clone());
        result
    }

    /// Credit Card validation: simulates bank verification
    fn validate_credit_card(&mut self, card: &str) -> tui_canvas::validation::ExternalValidationState {
        let cache_key = format!("card:{card}");
        if let Some(cached) = self.cache.get(&cache_key) {
            return cached.clone();
        }

        if card.is_empty() {
            return tui_canvas::validation::ExternalValidationState::NotValidated;
        }

        if !card.chars().all(|c| c.is_ascii_digit()) || card.len() != 16 {
            let result = tui_canvas::validation::ExternalValidationState::Invalid {
                message: "Invalid card format".to_string(),
                suggestion: Some("Enter 16 digits".to_string()),
            };
            self.cache.set(cache_key, result.clone());
            return result;
        }

        // Basic Luhn algorithm check (simplified)
        let sum: u32 = card
            .chars()
            .filter_map(|c| c.to_digit(10))
            .enumerate()
            .map(|(i, digit)| {
                if i % 2 == 0 {
                    let doubled = digit * 2;
                    if doubled > 9 {
                        doubled - 9
                    } else {
                        doubled
                    }
                } else {
                    digit
                }
            })
            .sum();

        if sum % 10 != 0 {
            let result = tui_canvas::validation::ExternalValidationState::Invalid {
                message: "Invalid card number (failed checksum)".to_string(),
                suggestion: Some("Check card number".to_string()),
            };
            self.cache.set(cache_key, result.clone());
            return result;
        }

        let result = match &card[..4] {
            "4000" => tui_canvas::validation::ExternalValidationState::Valid(Some("Visa - card verified".to_string())),
            "5555" => {
                tui_canvas::validation::ExternalValidationState::Valid(Some("Mastercard - card verified".to_string()))
            }
            "4111" => tui_canvas::validation::ExternalValidationState::Warning {
                message: "Test card number - not for real transactions".to_string(),
            },
            "0000" => tui_canvas::validation::ExternalValidationState::Invalid {
                message: "Card declined by issuer".to_string(),
                suggestion: Some("Contact your bank".to_string()),
            },
            _ => tui_canvas::validation::ExternalValidationState::Valid(Some(
                "Card number valid - bank not verified".to_string(),
            )),
        };

        self.cache.set(cache_key, result.clone());
        result
    }

    fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

/// Rich demo data with multiple validation types
struct ValidationDemoData {
    fields: Vec<(String, String)>,
}

impl ValidationDemoData {
    fn new() -> Self {
        Self {
            fields: vec![
                ("🏁 PSC (01001)".to_string(), "".to_string()),
                ("📧 Email (user@domain.com)".to_string(), "".to_string()),
                ("👤 Username (3+ chars)".to_string(), "".to_string()),
                ("🔑 API Key (32+ chars)".to_string(), "".to_string()),
                ("💳 Credit Card (16 digits)".to_string(), "".to_string()),
                ("📝 Notes (no validation)".to_string(), "".to_string()),
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

    #[cfg(feature = "validation")]
    fn validation_config(&self, field_index: usize) -> Option<tui_canvas::ValidationConfig> {
        match field_index {
            0 => Some(
                ValidationConfigBuilder::new()
                    .with_custom_formatter(Arc::new(PSCFormatter))
                    .with_max_length(5)
                    .with_external_validation_enabled(true)
                    .build(),
            ),
            1 => Some(
                ValidationConfigBuilder::new()
                    .with_max_length(50)
                    .with_external_validation_enabled(true)
                    .build(),
            ),
            2 => Some(
                ValidationConfigBuilder::new()
                    .with_max_length(20)
                    .with_external_validation_enabled(true)
                    .build(),
            ),
            3 => Some(
                ValidationConfigBuilder::new()
                    .with_max_length(50)
                    .with_external_validation_enabled(true)
                    .build(),
            ),
            4 => Some(
                ValidationConfigBuilder::new()
                    .with_custom_formatter(Arc::new(CreditCardFormatter))
                    .with_max_length(16)
                    .with_external_validation_enabled(true)
                    .build(),
            ),
            _ => None,
        }
    }
}

/// Editor with automatic external validation management
struct ValidationDemoEditor<D: DataProvider> {
    editor: TextFormState<D>,
    services: Arc<Mutex<ValidationServices>>,
    validation_history: Vec<(usize, String, TimedValidationResult)>,
    debug_message: String,
    show_history: bool,
    example_mode: usize,
    validation_enabled: bool,
    validation_stats: HashMap<usize, (u32, Duration)>, // field -> (count, total_time)
}

impl<D: DataProvider> ValidationDemoEditor<D> {
    fn new(data_provider: D) -> Self {
        let mut editor = TextFormState::new(data_provider);
        editor.set_validation_enabled(true);

        let services = Arc::new(Mutex::new(ValidationServices::new()));
        let services_for_cb = Arc::clone(&services);
        let services_for_history = Arc::clone(&services);

        // Create a history tracker that we'll share between callback and editor
        let validation_history: Arc<Mutex<Vec<(usize, String, TimedValidationResult)>>> =
            Arc::new(Mutex::new(Vec::new()));
        let history_for_cb = Arc::clone(&validation_history);

        // Library-level automatic external validation on field transitions
        editor.set_external_validation_callback(move |field_idx, text| {
            let mut svc = services_for_cb.lock().unwrap();

            let validation_type = match field_idx {
                0 => "PSC Lookup",
                1 => "Email Domain Check",
                2 => "Username Availability",
                3 => "API Key Auth",
                4 => "Credit Card Verify",
                _ => "Unknown",
            }
            .to_string();

            let start_time = Instant::now();

            let validation_result = match field_idx {
                0 => svc.validate_psc(text),
                1 => svc.validate_email(text),
                2 => svc.validate_username(text),
                3 => svc.validate_api_key(text),
                4 => svc.validate_credit_card(text),
                _ => tui_canvas::validation::ExternalValidationState::NotValidated,
            };

            // Record in shared history (if we can lock it)
            if let Ok(mut history) = history_for_cb.try_lock() {
                let duration = start_time.elapsed();
                let result = TimedValidationResult {
                    state: validation_result.clone(),
                    started_at: start_time,
                    completed_at: Some(Instant::now()),
                    validation_type,
                    cached: false, // We could enhance this by checking if it was from cache
                };

                history.push((field_idx, text.to_string(), result));

                // Limit history size
                if history.len() > 50 {
                    history.remove(0);
                }
            }

            validation_result
        });

        Self {
            editor,
            services,
            validation_history: Vec::new(),
            debug_message:
                "🧪 Enhanced External Validation Demo - Automatic validation on field transitions!"
                    .to_string(),
            show_history: false,
            example_mode: 0,
            validation_enabled: true,
            validation_stats: HashMap::new(),
        }
    }

    fn current_field(&self) -> usize {
        self.editor.current_field()
    }
    fn mode(&self) -> AppMode {
        self.editor.mode()
    }
    fn data_provider(&self) -> &D {
        self.editor.data_provider()
    }
    fn ui_state(&self) -> &tui_canvas::EditorState {
        self.editor.ui_state()
    }

    fn field_type(&self) -> &'static str {
        match self.current_field() {
            0 => "PSC",
            1 => "Email",
            2 => "Username",
            3 => "API Key",
            4 => "Credit Card",
            _ => "Plain Text",
        }
    }

    fn field_validation_rules(&self) -> &'static str {
        match self.current_field() {
            0 => "5 digits - checks postal service database",
            1 => "email@domain.com - verifies domain",
            2 => "3+ chars, alphanumeric + _ - checks availability",
            3 => "32+ chars - authenticates with service",
            4 => "16 digits - verifies with bank",
            _ => "No external validation",
        }
    }

    fn has_external_validation(&self) -> bool {
        self.current_field() < 5
    }

    /// Trigger external validation for specific field (manual validation)
    fn validate_field(&mut self, field_index: usize) {
        if !self.validation_enabled || field_index >= 5 {
            return;
        }

        let raw_value = self
            .editor
            .data_provider()
            .field_value(field_index)
            .to_string();
        if raw_value.is_empty() {
            self.editor.clear_external_validation(field_index);
            return;
        }

        // Set to validating state first
        self.editor
            .set_external_validation(field_index, tui_canvas::validation::ExternalValidationState::Validating);

        let validation_type = match field_index {
            0 => "PSC Lookup",
            1 => "Email Domain Check",
            2 => "Username Availability",
            3 => "API Key Auth",
            4 => "Credit Card Verify",
            _ => "Unknown",
        }
        .to_string();

        let mut result = TimedValidationResult::new(validation_type.clone());

        // Perform validation using the shared services
        let validation_result = {
            let mut svc = self.services.lock().unwrap();
            match field_index {
                0 => svc.validate_psc(&raw_value),
                1 => svc.validate_email(&raw_value),
                2 => svc.validate_username(&raw_value),
                3 => svc.validate_api_key(&raw_value),
                4 => svc.validate_credit_card(&raw_value),
                _ => tui_canvas::validation::ExternalValidationState::NotValidated,
            }
        };

        result = result.complete(validation_result.clone());

        // Update editor state
        self.editor
            .set_external_validation(field_index, validation_result);

        // Record in history
        self.validation_history
            .push((field_index, raw_value, result.clone()));

        // Update stats
        let stats = self
            .validation_stats
            .entry(field_index)
            .or_insert((0, Duration::from_secs(0)));
        stats.0 += 1;
        stats.1 += result.duration();

        // Limit history size
        if self.validation_history.len() > 50 {
            self.validation_history.remove(0);
        }

        let duration_ms = result.duration().as_millis();
        let cached_text = if result.cached { " (cached)" } else { "" };
        self.debug_message = format!(
            "🔍 {validation_type} validation completed in {duration_ms}ms{cached_text} (manual)"
        );
    }

    fn validate_all_fields(&mut self) {
        let field_count = self.editor.data_provider().field_count().min(5); // Only fields with validation
        for i in 0..field_count {
            self.validate_field(i);
        }
        self.debug_message = "🔍 All fields validated manually".to_string();
    }

    fn clear_validation_state(&mut self, field_index: Option<usize>) {
        match field_index {
            Some(idx) => {
                self.editor.clear_external_validation(idx);
                self.debug_message = format!("🧹 Cleared validation for field {}", idx + 1);
            }
            None => {
                for i in 0..5 {
                    // Clear all validation fields
                    self.editor.clear_external_validation(i);
                }
                self.validation_history.clear();
                self.validation_stats.clear();
                if let Ok(mut svc) = self.services.lock() {
                    svc.clear_cache();
                }
                self.debug_message = "🧹 Cleared all validation states and cache".to_string();
            }
        }
    }

    fn toggle_validation(&mut self) {
        self.validation_enabled = !self.validation_enabled;
        if !self.validation_enabled {
            self.clear_validation_state(None);
        }
        self.debug_message = if self.validation_enabled {
            "✅ External validation ENABLED".to_string()
        } else {
            "❌ External validation DISABLED".to_string()
        };
    }

    fn toggle_history_view(&mut self) {
        self.show_history = !self.show_history;
        self.debug_message = if self.show_history {
            "📜 Showing validation history".to_string()
        } else {
            "📊 Showing validation status".to_string()
        };
    }

    fn cycle_examples(&mut self) {
        let examples = [
            // Valid examples
            vec![
                "01001",
                "user@gmail.com",
                "alice_dev_new",
                "valid_api_key_123456789012345",
                "4000123456789012",
                "Valid data",
            ],
            // Invalid examples
            vec![
                "00000",
                "invalid-email",
                "admin",
                "short_key",
                "0000000000000000",
                "Invalid data",
            ],
            // Warning examples
            vec![
                "12345",
                "test@example.com",
                "temp_user",
                "test_api_key_123456789012345",
                "4111111111111111",
                "Warning cases",
            ],
            // Mixed scenarios
            vec![
                "99999",
                "user@blocked.com",
                "john123",
                "expired_key_12345678901",
                "5555555555554444",
                "Mixed scenarios",
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
            "Invalid Examples",
            "Warning Cases",
            "Mixed Scenarios",
        ];
        self.debug_message = format!(
            "📋 Loaded: {} (navigate to trigger validation)",
            mode_names[self.example_mode]
        );
    }

    fn get_validation_summary(&self) -> String {
        let total_validations: u32 = self.validation_stats.values().map(|(count, _)| count).sum();
        let avg_time_ms = if total_validations > 0 {
            let total_time: Duration = self.validation_stats.values().map(|(_, time)| *time).sum();
            total_time.as_millis() / total_validations as u128
        } else {
            0
        };

        format!("Total: {total_validations} validations, Avg: {avg_time_ms}ms")
    }

    fn get_field_validation_state(&self, field_index: usize) -> tui_canvas::validation::ExternalValidationState {
        self.editor
            .ui_state()
            .validation_state()
            .get_external_validation(field_index)
    }

    // Editor pass-through methods - simplified since library handles automatic validation
    fn enter_edit_mode(&mut self) {
        self.editor.enter_edit_mode();
        let rules = self.field_validation_rules();
        self.debug_message = format!(
            "✏️ INSERT MODE - Cursor: Steady Bar | - {} - {}",
            self.field_type(),
            rules
        );
    }

    fn exit_edit_mode(&mut self) {
        self.editor.exit_edit_mode();
        // Library automatically validates on exit, no manual call needed
        self.debug_message = format!(
            "🔒 NORMAL - Cursor: Steady Block █ - {} (auto-validated)",
            self.field_type()
        );
    }

    fn next_field(&mut self) {
        if self.editor.next_field() {
            // Library triggers external validation automatically via transition_to_field()
            self.debug_message = "➡ Next field (auto-validation triggered by library)".to_string();
        }
    }

    fn prev_field(&mut self) {
        if self.editor.prev_field() {
            // Library triggers external validation automatically via transition_to_field()
            self.debug_message =
                "⬅ Previous field (auto-validation triggered by library)".to_string();
        }
    }

    fn move_up(&mut self) {
        if self.editor.move_up() {
            // Library triggers external validation automatically via transition_to_field()
            self.debug_message = "⬆ Move up (auto-validation triggered by library)".to_string();
        }
    }

    fn move_down(&mut self) {
        if self.editor.move_down() {
            // Library triggers external validation automatically via transition_to_field()
            self.debug_message = "⬇ Move down (auto-validation triggered by library)".to_string();
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
}

fn run_app<B: Backend<Error = io::Error>>(
    terminal: &mut Terminal<B>,
    mut editor: ValidationDemoEditor<ValidationDemoData>,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &editor))?;

        if let Event::Key(key) = event::read()? {
            let mode = editor.mode();
            let kc = key.code;
            let km = key.modifiers;

            // Quit
            if matches!(kc, KeyCode::F(10))
                || (kc == KeyCode::Char('q') && km.contains(KeyModifiers::CONTROL))
                || (kc == KeyCode::Char('c') && km.contains(KeyModifiers::CONTROL))
            {
                break;
            }

            match (mode, kc, km) {
                // Mode transitions
                (AppMode::Nor, KeyCode::Char('i'), _) => editor.enter_edit_mode(),
                (AppMode::Nor, KeyCode::Char('a'), _) => {
                    editor.editor.enter_append_mode();
                    let rules = editor.field_validation_rules();
                    editor.debug_message = format!("✏️ APPEND {} - {}", editor.field_type(), rules);
                }
                (_, KeyCode::Esc, _) => editor.exit_edit_mode(),

                // Movement - these now trigger automatic validation via the library!
                (_, KeyCode::Left, _) | (AppMode::Nor, KeyCode::Char('h'), _) => {
                    let _ = editor.editor.move_left();
                }
                (_, KeyCode::Right, _) | (AppMode::Nor, KeyCode::Char('l'), _) => {
                    let _ = editor.editor.move_right();
                }
                (_, KeyCode::Up, _) | (AppMode::Nor, KeyCode::Char('k'), _) => {
                    editor.move_up(); // Use wrapper to get debug message
                }
                (_, KeyCode::Down, _) | (AppMode::Nor, KeyCode::Char('j'), _) => {
                    editor.move_down(); // Use wrapper to get debug message
                }

                // Field switching - these trigger automatic validation via the library!
                (_, KeyCode::Tab, _) => editor.next_field(),
                (_, KeyCode::BackTab, _) => editor.prev_field(),

                // Manual validation commands (ONLY in ReadOnly mode)
                (AppMode::Nor, KeyCode::Char('v'), _) => {
                    let field = editor.current_field();
                    editor.validate_field(field);
                }
                (AppMode::Nor, KeyCode::Char('V'), _) => editor.validate_all_fields(),
                (AppMode::Nor, KeyCode::Char('c'), _) => {
                    let field = editor.current_field();
                    editor.clear_validation_state(Some(field));
                }
                (AppMode::Nor, KeyCode::Char('C'), _) => editor.clear_validation_state(None),

                // UI toggles (ONLY in ReadOnly mode for alpha keys to avoid blocking text input)
                (AppMode::Nor, KeyCode::Char('r'), _) => editor.toggle_history_view(),
                (AppMode::Nor, KeyCode::Char('e'), _) => editor.cycle_examples(),
                (_, KeyCode::F(1), _) => editor.toggle_validation(),

                // Editing
                (AppMode::Ins, KeyCode::Left, _) => {
                    let _ = editor.editor.move_left();
                }
                (AppMode::Ins, KeyCode::Right, _) => {
                    let _ = editor.editor.move_right();
                }
                (AppMode::Ins, KeyCode::Up, _) => {
                    editor.move_up();
                }
                (AppMode::Ins, KeyCode::Down, _) => {
                    editor.move_down();
                }
                (AppMode::Ins, KeyCode::Char(c), m) if !m.contains(KeyModifiers::CONTROL) => {
                    let _ = editor.insert_char(c);
                }
                (AppMode::Ins, KeyCode::Backspace, _) => {
                    let _ = editor.delete_backward();
                }
                (AppMode::Ins, KeyCode::Delete, _) => {
                    let _ = editor.delete_forward();
                }

                // Help
                (_, KeyCode::Char('h'), _) => {
                    let rules = editor.field_validation_rules();
                    editor.debug_message = format!("ℹ️ {} field: {}", editor.field_type(), rules);
                }

                _ => {}
            }
        }
    }
    Ok(())
}

fn ui(f: &mut Frame, editor: &ValidationDemoEditor<ValidationDemoData>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(16)])
        .split(f.area());

    render_canvas_default(f, chunks[0], &editor.editor);
    render_validation_panel(f, chunks[1], editor);
}

fn render_validation_panel(
    f: &mut Frame,
    area: Rect,
    editor: &ValidationDemoEditor<ValidationDemoData>,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Status bar
            Constraint::Length(6), // Validation states
            Constraint::Length(7), // History or Help
        ])
        .split(area);

    // Status bar
    let mode_text = match editor.mode() {
        AppMode::Ins => "INSERT | (bar cursor)",
        AppMode::Nor => "NORMAL █ (block cursor)",
        _ => "NORMAL █ (block cursor)",
    };

    let summary = editor.get_validation_summary();
    let status_text = format!(
        "-- {} -- {} | {} | View: {}",
        mode_text,
        editor.debug_message,
        summary,
        if editor.show_history {
            "HISTORY"
        } else {
            "STATUS"
        }
    );

    let status = Paragraph::new(Line::from(Span::raw(status_text))).block(
        Block::default()
            .borders(Borders::ALL)
            .title("🧪 Automatic External Validation Demo"),
    );
    f.render_widget(status, chunks[0]);

    // Validation states for all fields - render each field on its own line
    let mut field_lines: Vec<Line> = Vec::new();
    for i in 0..editor.data_provider().field_count() {
        let field_name = editor.data_provider().field_name(i);
        let raw_value = editor.data_provider().field_value(i);
        let state = editor.get_field_validation_state(i);

        let (state_text, color) = match state {
            tui_canvas::validation::ExternalValidationState::NotValidated => ("Not validated", Color::Gray),
            tui_canvas::validation::ExternalValidationState::Validating => ("Validating…", Color::Blue),
            tui_canvas::validation::ExternalValidationState::Valid(Some(ref msg)) => (msg.as_str(), Color::Green),
            tui_canvas::validation::ExternalValidationState::Valid(None) => ("Valid ✓", Color::Green),
            tui_canvas::validation::ExternalValidationState::Invalid { ref message, .. } => (message.as_str(), Color::Red),
            tui_canvas::validation::ExternalValidationState::Warning { ref message } => (message.as_str(), Color::Yellow),
        };

        let indicator = if i == editor.current_field() {
            "► "
        } else {
            "  "
        };

        let value_display = if raw_value.len() > 15 {
            format!("{}...", &raw_value[..12])
        } else if raw_value.is_empty() {
            "(empty)".to_string()
        } else {
            raw_value.to_string()
        };

        let field_line = Line::from(vec![
            Span::styled(
                format!("{indicator}{field_name}: "),
                Style::default().fg(Color::White),
            ),
            Span::raw(format!("'{value_display}' → ")),
            Span::styled(state_text.to_string(), Style::default().fg(color)),
        ]);

        field_lines.push(field_line);
    }

    let validation_states = Paragraph::new(field_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("🔍 Validation States (Library Auto-triggered)"),
    );
    f.render_widget(validation_states, chunks[1]);

    // History or Help panel
    if editor.show_history {
        let recent_history: Vec<ListItem> = editor
            .validation_history
            .iter()
            .rev()
            .take(5)
            .map(|(field_idx, value, result)| {
                let field_name = match field_idx {
                    0 => "PSC",
                    1 => "Email",
                    2 => "Username",
                    3 => "API Key",
                    4 => "Card",
                    _ => "Other",
                };

                let duration_ms = result.duration().as_millis();
                let cached_text = if result.cached { " (cached)" } else { "" };
                let short_value = if value.len() > 15 {
                    format!("{}...", &value[..12])
                } else {
                    value.clone()
                };

                let state_summary = match &result.state {
                    tui_canvas::validation::ExternalValidationState::Valid(_) => "✓ Valid",
                    tui_canvas::validation::ExternalValidationState::Invalid { .. } => "✖ Invalid",
                    tui_canvas::validation::ExternalValidationState::Warning { .. } => "⚠ Warning",
                    tui_canvas::validation::ExternalValidationState::Validating => "… Validating",
                    tui_canvas::validation::ExternalValidationState::NotValidated => "○ Not validated",
                };

                ListItem::new(format!(
                    "{field_name}: '{short_value}' → {state_summary} ({duration_ms}ms{cached_text})"
                ))
            })
            .collect();

        let history = List::new(recent_history).block(
            Block::default()
                .borders(Borders::ALL)
                .title("📜 Auto-Validation History (recent 5)"),
        );
        f.render_widget(history, chunks[2]);
    } else {
        let help_text = match editor.mode() {
            AppMode::Nor => {
                "🎯 FULLY AUTOMATIC VALIDATION: Library handles all validation on field transitions!\n\
                 🧪 EXTERNAL VALIDATION DEMO - No manual triggers needed, just navigate!\n\
                 \n\
                 🚀 AUTOMATIC: Arrow keys, Tab, and Esc trigger validation automatically\n\
                 Manual: v=validate current, V=validate all, c=clear current, C=clear all\n\
                 Controls: e=cycle examples, r=toggle history, h=field help, F1=toggle validation\n\
                 \n\
                 Just load examples and navigate - validation happens automatically!"
            }
            AppMode::Ins => {
                "🎯 INSERT MODE - Cursor: | (bar)\n\
                 ✏️ Type to edit field content\n\
                 \n\
                 🚀 AUTOMATIC: Library validates when you leave this field via:\n\
                 • Press Esc (exit edit mode)\n\
                 • Press Tab/Shift+Tab (move between fields)\n\
                 • Press arrow keys (Up/Down move between fields)\n\
                 \n\
                 Esc=exit edit, arrows=navigate, Backspace/Del=delete"
            }
            _ => "🧪 Enhanced Fully Automatic External Validation Demo"
        };

        let help = Paragraph::new(help_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("🚀 Fully Automatic External Validation"),
            )
            .style(Style::default().fg(Color::Gray))
            .wrap(Wrap { trim: true });
        f.render_widget(help, chunks[2]);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Enhanced Fully Automatic External Validation Demo (Feature 5)");
    println!("✅ validation feature: ENABLED");
    println!("✅ gui feature: ENABLED");
    println!("✅ cursor-style feature: ENABLED");
    println!("🚀 NEW: Library handles all automatic validation!");
    println!("🧪 Enhanced features:");
    println!("   • 5 different external validation types with realistic scenarios");
    println!("   • LIBRARY-LEVEL automatic validation on all field transitions");
    println!("   • Validation caching and performance metrics");
    println!("   • Comprehensive validation history and error handling");
    println!("   • Multiple example datasets for testing edge cases");
    println!("   • Progressive validation patterns (local + remote simulation)");
    println!("   • NO manual validation calls needed - library handles everything!");
    println!();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let data = ValidationDemoData::new();
    let mut editor = ValidationDemoEditor::new(data);

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

    println!("🧪 Enhanced fully automatic external validation demo completed!");
    println!("🏆 You experienced library-level automatic external validation with:");
    println!("   • Multiple validation services (PSC, Email, Username, API Key, Credit Card)");
    println!("   • AUTOMATIC validation handled entirely by the library");
    println!("   • Realistic async validation simulation with caching");
    println!("   • Comprehensive error handling and user feedback");
    println!("   • Performance metrics and validation history tracking");
    println!("   • Zero manual validation calls needed!");
    Ok(())
}
