// canvas/src/config.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crossterm::event::{KeyCode, KeyModifiers};
use anyhow::{Context, Result};

use super::registry::{ActionRegistry, ActionSpec, ModeRegistry};
use super::validation::{ConfigValidator, ValidationError, ValidationResult, ValidationWarning};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasConfig {
    #[serde(default)]
    pub keybindings: CanvasKeybindings,
    #[serde(default)]
    pub behavior: CanvasBehavior,
    #[serde(default)]
    pub appearance: CanvasAppearance,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CanvasKeybindings {
    #[serde(default)]
    pub read_only: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub edit: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub suggestions: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub global: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasBehavior {
    #[serde(default = "default_wrap_around")]
    pub wrap_around_fields: bool,
    #[serde(default = "default_auto_save")]
    pub auto_save_on_field_change: bool,
    #[serde(default = "default_word_chars")]
    pub word_chars: String,
    #[serde(default = "default_suggestion_limit")]
    pub max_suggestions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasAppearance {
    #[serde(default = "default_cursor_style")]
    pub cursor_style: String, // "block", "bar", "underline"
    #[serde(default = "default_show_field_numbers")]
    pub show_field_numbers: bool,
    #[serde(default = "default_highlight_current_field")]
    pub highlight_current_field: bool,
}

// Default values
fn default_wrap_around() -> bool { true }
fn default_auto_save() -> bool { false }
fn default_word_chars() -> String { "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_".to_string() }
fn default_suggestion_limit() -> usize { 10 }
fn default_cursor_style() -> String { "block".to_string() }
fn default_show_field_numbers() -> bool { false }
fn default_highlight_current_field() -> bool { true }

impl Default for CanvasBehavior {
    fn default() -> Self {
        Self {
            wrap_around_fields: default_wrap_around(),
            auto_save_on_field_change: default_auto_save(),
            word_chars: default_word_chars(),
            max_suggestions: default_suggestion_limit(),
        }
    }
}

impl Default for CanvasAppearance {
    fn default() -> Self {
        Self {
            cursor_style: default_cursor_style(),
            show_field_numbers: default_show_field_numbers(),
            highlight_current_field: default_highlight_current_field(),
        }
    }
}

impl Default for CanvasConfig {
    fn default() -> Self {
        Self {
            keybindings: CanvasKeybindings::with_vim_defaults(),
            behavior: CanvasBehavior::default(),
            appearance: CanvasAppearance::default(),
        }
    }
}

impl CanvasKeybindings {
    pub fn with_vim_defaults() -> Self {
        let mut keybindings = Self::default();

        // Read-only mode (vim-style navigation)
        keybindings.read_only.insert("move_left".to_string(), vec!["h".to_string()]);
        keybindings.read_only.insert("move_right".to_string(), vec!["l".to_string()]);
        keybindings.read_only.insert("move_up".to_string(), vec!["k".to_string()]);
        keybindings.read_only.insert("move_down".to_string(), vec!["j".to_string()]);
        keybindings.read_only.insert("move_word_next".to_string(), vec!["w".to_string()]);
        keybindings.read_only.insert("move_word_end".to_string(), vec!["e".to_string()]);
        keybindings.read_only.insert("move_word_prev".to_string(), vec!["b".to_string()]);
        keybindings.read_only.insert("move_word_end_prev".to_string(), vec!["ge".to_string()]);
        keybindings.read_only.insert("move_line_start".to_string(), vec!["0".to_string()]);
        keybindings.read_only.insert("move_line_end".to_string(), vec!["$".to_string()]);
        keybindings.read_only.insert("move_first_line".to_string(), vec!["gg".to_string()]);
        keybindings.read_only.insert("move_last_line".to_string(), vec!["G".to_string()]);
        keybindings.read_only.insert("next_field".to_string(), vec!["Tab".to_string()]);
        keybindings.read_only.insert("prev_field".to_string(), vec!["Shift+Tab".to_string()]);

        // Edit mode
        keybindings.edit.insert("delete_char_backward".to_string(), vec!["Backspace".to_string()]);
        keybindings.edit.insert("delete_char_forward".to_string(), vec!["Delete".to_string()]);
        keybindings.edit.insert("move_left".to_string(), vec!["Left".to_string()]);
        keybindings.edit.insert("move_right".to_string(), vec!["Right".to_string()]);
        keybindings.edit.insert("move_up".to_string(), vec!["Up".to_string()]);
        keybindings.edit.insert("move_down".to_string(), vec!["Down".to_string()]);
        keybindings.edit.insert("move_line_start".to_string(), vec!["Home".to_string()]);
        keybindings.edit.insert("move_line_end".to_string(), vec!["End".to_string()]);
        keybindings.edit.insert("move_word_next".to_string(), vec!["Ctrl+Right".to_string()]);
        keybindings.edit.insert("move_word_prev".to_string(), vec!["Ctrl+Left".to_string()]);
        keybindings.edit.insert("next_field".to_string(), vec!["Tab".to_string()]);
        keybindings.edit.insert("prev_field".to_string(), vec!["Shift+Tab".to_string()]);

        // Suggestions
        keybindings.suggestions.insert("suggestion_up".to_string(), vec!["Up".to_string(), "Ctrl+p".to_string()]);
        keybindings.suggestions.insert("suggestion_down".to_string(), vec!["Down".to_string(), "Ctrl+n".to_string()]);
        keybindings.suggestions.insert("select_suggestion".to_string(), vec!["Enter".to_string(), "Tab".to_string()]);
        keybindings.suggestions.insert("exit_suggestions".to_string(), vec!["Esc".to_string()]);

        // Global (works in both modes)
        keybindings.global.insert("move_up".to_string(), vec!["Up".to_string()]);
        keybindings.global.insert("move_down".to_string(), vec!["Down".to_string()]);

        keybindings
    }

    pub fn with_emacs_defaults() -> Self {
        let mut keybindings = Self::default();

        // Emacs-style bindings
        keybindings.read_only.insert("move_left".to_string(), vec!["Ctrl+b".to_string()]);
        keybindings.read_only.insert("move_right".to_string(), vec!["Ctrl+f".to_string()]);
        keybindings.read_only.insert("move_up".to_string(), vec!["Ctrl+p".to_string()]);
        keybindings.read_only.insert("move_down".to_string(), vec!["Ctrl+n".to_string()]);
        keybindings.read_only.insert("move_word_next".to_string(), vec!["Alt+f".to_string()]);
        keybindings.read_only.insert("move_word_prev".to_string(), vec!["Alt+b".to_string()]);
        keybindings.read_only.insert("move_line_start".to_string(), vec!["Ctrl+a".to_string()]);
        keybindings.read_only.insert("move_line_end".to_string(), vec!["Ctrl+e".to_string()]);

        keybindings.edit.insert("delete_char_backward".to_string(), vec!["Ctrl+h".to_string(), "Backspace".to_string()]);
        keybindings.edit.insert("delete_char_forward".to_string(), vec!["Ctrl+d".to_string(), "Delete".to_string()]);

        keybindings
    }
}

impl CanvasConfig {
    /// NEW: Load and validate configuration
    pub fn load() -> Self {
        match Self::load_and_validate() {
            Ok(config) => config,
            Err(e) => {
                eprintln!("⚠️  Canvas config validation failed: {}", e);
                eprintln!("   Using vim defaults. Run CanvasConfig::generate_template() for help.");
                Self::default()
            }
        }
    }

    /// NEW: Load configuration with validation
    pub fn load_and_validate() -> Result<Self> {
        // Try to load canvas_config.toml from current directory
        let config = if let Ok(config) = Self::from_file(std::path::Path::new("canvas_config.toml")) {
            config
        } else {
            // Fallback to vim defaults
            Self::default()
        };

        // Validate the configuration
        let validator = ConfigValidator::new();
        let validation_result = validator.validate_keybindings(&config.keybindings);

        if !validation_result.is_valid {
            // Print validation errors
            validator.print_validation_result(&validation_result);
            
            // Create error with suggestions
            let error_msg = format!(
                "Configuration validation failed with {} errors", 
                validation_result.errors.len()
            );
            return Err(anyhow::anyhow!(error_msg));
        }

        // Print warnings if any
        if !validation_result.warnings.is_empty() {
            validator.print_validation_result(&validation_result);
        }

        Ok(config)
    }

    /// NEW: Generate a complete configuration template
    pub fn generate_template() -> String {
        let registry = ActionRegistry::new();
        registry.generate_config_template()
    }

    /// NEW: Generate a clean, minimal configuration template
    pub fn generate_clean_template() -> String {
        let registry = ActionRegistry::new();
        registry.generate_clean_template()
    }

    /// NEW: Validate current configuration
    pub fn validate(&self) -> ValidationResult {
        let validator = ConfigValidator::new();
        validator.validate_keybindings(&self.keybindings)
    }

    /// NEW: Print validation results for current config
    pub fn print_validation(&self) {
        let validator = ConfigValidator::new();
        let result = validator.validate_keybindings(&self.keybindings);
        validator.print_validation_result(&result);
    }

    /// NEW: Generate config for missing required actions
    pub fn generate_missing_config(&self) -> String {
        let validator = ConfigValidator::new();
        validator.generate_missing_config(&self.keybindings)
    }

    /// Load from TOML string
    pub fn from_toml(toml_str: &str) -> Result<Self> {
        toml::from_str(toml_str)
            .with_context(|| "Failed to parse canvas config TOML")
    }

    /// Load from file
    pub fn from_file(path: &std::path::Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {:?}", path))?;
        Self::from_toml(&contents)
    }

    /// NEW: Check if autocomplete should auto-trigger (simple logic)
    pub fn should_auto_trigger_autocomplete(&self) -> bool {
        // If trigger_autocomplete keybinding exists anywhere, use manual mode only
        // If no trigger_autocomplete keybinding, use auto-trigger mode
        !self.has_trigger_autocomplete_keybinding()
    }

    /// NEW: Check if user has configured manual trigger keybinding
    pub fn has_trigger_autocomplete_keybinding(&self) -> bool {
        self.keybindings.edit.contains_key("trigger_autocomplete") ||
        self.keybindings.read_only.contains_key("trigger_autocomplete") ||
        self.keybindings.global.contains_key("trigger_autocomplete")
    }

    // ... rest of your existing methods stay the same ...
    
    /// Get action for key in read-only mode
    pub fn get_read_only_action(&self, key: KeyCode, modifiers: KeyModifiers) -> Option<&str> {
        self.get_action_in_mode(&self.keybindings.read_only, key, modifiers)
            .or_else(|| self.get_action_in_mode(&self.keybindings.global, key, modifiers))
    }

    /// Get action for key in edit mode
    pub fn get_edit_action(&self, key: KeyCode, modifiers: KeyModifiers) -> Option<&str> {
        self.get_action_in_mode(&self.keybindings.edit, key, modifiers)
            .or_else(|| self.get_action_in_mode(&self.keybindings.global, key, modifiers))
    }

    /// Get action for key in suggestions mode
    pub fn get_suggestion_action(&self, key: KeyCode, modifiers: KeyModifiers) -> Option<&str> {
        self.get_action_in_mode(&self.keybindings.suggestions, key, modifiers)
    }

    /// Get action for key (mode-aware)
    pub fn get_action_for_key(&self, key: KeyCode, modifiers: KeyModifiers, is_edit_mode: bool, has_suggestions: bool) -> Option<&str> {
        // Suggestions take priority when active
        if has_suggestions {
            if let Some(action) = self.get_suggestion_action(key, modifiers) {
                return Some(action);
            }
        }

        // Then check mode-specific
        if is_edit_mode {
            self.get_edit_action(key, modifiers)
        } else {
            self.get_read_only_action(key, modifiers)
        }
    }

    // ... keep all your existing private methods ...
    fn get_action_in_mode<'a>(&self, mode_bindings: &'a HashMap<String, Vec<String>>, key: KeyCode, modifiers: KeyModifiers) -> Option<&'a str> {
        for (action, bindings) in mode_bindings {
            for binding in bindings {
                if self.matches_keybinding(binding, key, modifiers) {
                    return Some(action);
                }
            }
        }
        None
    }

    fn matches_keybinding(&self, binding: &str, key: KeyCode, modifiers: KeyModifiers) -> bool {
        // ... keep all your existing key matching logic ...
        // (This is a very long method, so I'm just indicating to keep it as-is)
        
        // Your existing implementation here...
        true // placeholder - use your actual implementation
    }

    /// Convenience method to create vim preset
    pub fn vim_preset() -> Self {
        Self {
            keybindings: CanvasKeybindings::with_vim_defaults(),
            behavior: CanvasBehavior::default(),
            appearance: CanvasAppearance::default(),
        }
    }

    /// Convenience method to create emacs preset
    pub fn emacs_preset() -> Self {
        Self {
            keybindings: CanvasKeybindings::with_emacs_defaults(),
            behavior: CanvasBehavior::default(),
            appearance: CanvasAppearance::default(),
        }
    }

    /// Debug method to print loaded keybindings
    pub fn debug_keybindings(&self) {
        println!("📋 Canvas keybindings loaded:");
        println!("  Read-only: {} actions", self.keybindings.read_only.len());
        println!("  Edit: {} actions", self.keybindings.edit.len());
        println!("  Suggestions: {} actions", self.keybindings.suggestions.len());
        println!("  Global: {} actions", self.keybindings.global.len());
        
        // NEW: Show validation status
        let validation = self.validate();
        if validation.is_valid {
            println!("  ✅ Configuration is valid");
        } else {
            println!("  ❌ Configuration has {} errors", validation.errors.len());
        }
        if !validation.warnings.is_empty() {
            println!("  ⚠️  Configuration has {} warnings", validation.warnings.len());
        }
    }
}

// Re-export for convenience
pub use crate::canvas::actions::CanvasAction;
pub use crate::dispatcher::ActionDispatcher;
