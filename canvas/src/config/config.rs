// src/config/config.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crossterm::event::{KeyCode, KeyModifiers};
use anyhow::{Context, Result};

// Import from sibling modules
use super::registry::ActionRegistry;
use super::validation::{ConfigValidator, ValidationError, ValidationResult, ValidationWarning};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasKeybindings {
    pub edit: HashMap<String, Vec<String>>,
    pub read_only: HashMap<String, Vec<String>>,
    pub global: HashMap<String, Vec<String>>,
}

impl Default for CanvasKeybindings {
    fn default() -> Self {
        Self {
            edit: HashMap::new(),
            read_only: HashMap::new(),
            global: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasBehavior {
    pub confirm_on_save: bool,
    pub auto_indent: bool,
    pub wrap_search: bool,
    pub wrap_around_fields: bool,
}

impl Default for CanvasBehavior {
    fn default() -> Self {
        Self {
            confirm_on_save: true,
            auto_indent: true,
            wrap_search: true,
            wrap_around_fields: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasAppearance {
    pub line_numbers: bool,
    pub syntax_highlighting: bool,
    pub current_line_highlight: bool,
}

impl Default for CanvasAppearance {
    fn default() -> Self {
        Self {
            line_numbers: true,
            syntax_highlighting: true,
            current_line_highlight: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasConfig {
    pub keybindings: CanvasKeybindings,
    pub behavior: CanvasBehavior,
    pub appearance: CanvasAppearance,
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
        // TODO: Could be generated from introspection too
        let mut keybindings = Self::default();

        // Read-only mode (vim-style navigation)
        keybindings.read_only.insert("move_left".to_string(), vec!["h".to_string()]);
        keybindings.read_only.insert("move_right".to_string(), vec!["l".to_string()]);
        keybindings.read_only.insert("move_up".to_string(), vec!["k".to_string()]);
        keybindings.read_only.insert("move_down".to_string(), vec!["j".to_string()]);

        // Edit mode
        keybindings.edit.insert("delete_char_backward".to_string(), vec!["Backspace".to_string()]);
        keybindings.edit.insert("move_left".to_string(), vec!["Left".to_string()]);
        keybindings.edit.insert("move_right".to_string(), vec!["Right".to_string()]);
        keybindings.edit.insert("move_up".to_string(), vec!["Up".to_string()]);
        keybindings.edit.insert("move_down".to_string(), vec!["Down".to_string()]);
        keybindings.edit.insert("next_field".to_string(), vec!["Tab".to_string()]);
        keybindings.edit.insert("prev_field".to_string(), vec!["Shift+Tab".to_string()]);

        keybindings
    }
}

impl CanvasConfig {
    /// NEW: Load and validate configuration using dynamic registry
    pub fn load() -> Self {
        match Self::load_and_validate() {
            Ok(config) => config,
            Err(e) => {
                eprintln!("⚠️  Failed to load canvas config: {}", e);
                eprintln!("   Using default configuration");
                Self::default()
            }
        }
    }

    /// NEW: Load configuration with validation using dynamic registry
    pub fn load_and_validate() -> Result<Self> {
        // Try to load canvas_config.toml from current directory
        let config = if let Ok(config) = Self::from_file(std::path::Path::new("canvas_config.toml")) {
            config
        } else {
            // Use default if file doesn't exist
            Self::default()
        };

        // NEW: Use dynamic registry from actual handlers
        let registry = ActionRegistry::from_handlers();

        // Validate the handlers match their claimed capabilities
        if let Err(handler_errors) = registry.validate_against_implementation() {
            eprintln!("⚠️  Handler validation failed:");
            for error in handler_errors {
                eprintln!("   - {}", error);
            }
        }

        // Validate the configuration against the dynamic registry
        let validator = ConfigValidator::new(registry);
        let validation_result = validator.validate_keybindings(&config.keybindings);

        if !validation_result.is_valid {
            eprintln!("❌ Canvas configuration validation failed:");
            validator.print_validation_result(&validation_result);
            eprintln!();
            eprintln!("🔧 To generate a working config template:");
            eprintln!("   CanvasConfig::generate_template()");
            eprintln!();
            eprintln!("📁 Expected config file location: canvas_config.toml");
        } else if !validation_result.warnings.is_empty() {
            eprintln!("⚠️  Canvas configuration has warnings:");
            validator.print_validation_result(&validation_result);
        }

        Ok(config)
    }

    /// NEW: Generate template from actual handler capabilities
    pub fn generate_template() -> String {
        let registry = ActionRegistry::from_handlers();

        // Validate handlers first
        if let Err(errors) = registry.validate_against_implementation() {
            eprintln!("⚠️  Warning: Handler validation failed while generating template:");
            for error in errors {
                eprintln!("   - {}", error);
            }
        }

        registry.generate_config_template()
    }

    /// NEW: Generate clean template from actual handler capabilities
    pub fn generate_clean_template() -> String {
        let registry = ActionRegistry::from_handlers();

        // Validate handlers first
        if let Err(errors) = registry.validate_against_implementation() {
            eprintln!("⚠️  Warning: Handler validation failed while generating template:");
            for error in errors {
                eprintln!("   - {}", error);
            }
        }

        registry.generate_clean_template()
    }

    /// NEW: Validate current configuration against actual implementation
    pub fn validate(&self) -> ValidationResult {
        let registry = ActionRegistry::from_handlers();
        let validator = ConfigValidator::new(registry);
        validator.validate_keybindings(&self.keybindings)
    }

    /// NEW: Print validation results for current config
    pub fn print_validation(&self) {
        let registry = ActionRegistry::from_handlers();
        let validator = ConfigValidator::new(registry);
        let result = validator.validate_keybindings(&self.keybindings);
        validator.print_validation_result(&result);
    }

    /// Load from TOML string
    pub fn from_toml(toml_str: &str) -> Result<Self> {
        toml::from_str(toml_str)
            .context("Failed to parse TOML configuration")
    }

    /// Load from file
    pub fn from_file(path: &std::path::Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .context("Failed to read config file")?;
        Self::from_toml(&contents)
    }

    /// RESTORED: Check if autocomplete should auto-trigger (simple logic)
    pub fn should_auto_trigger_autocomplete(&self) -> bool {
        // If trigger_autocomplete keybinding exists anywhere, use manual mode only
        // If no trigger_autocomplete keybinding, use auto-trigger mode
        !self.has_trigger_autocomplete_keybinding()
    }

    /// RESTORED: Check if user has configured manual trigger keybinding
    pub fn has_trigger_autocomplete_keybinding(&self) -> bool {
        self.keybindings.edit.contains_key("trigger_autocomplete") ||
        self.keybindings.read_only.contains_key("trigger_autocomplete") ||
        self.keybindings.global.contains_key("trigger_autocomplete")
    }

    // ... keep all your existing key matching methods ...

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

    /// Get action for key (mode-aware)
    pub fn get_action_for_key(&self, key: KeyCode, modifiers: KeyModifiers, is_edit_mode: bool, _has_suggestions: bool) -> Option<&str> {
        // Check mode-specific
        if is_edit_mode {
            self.get_edit_action(key, modifiers)
        } else {
            self.get_read_only_action(key, modifiers)
        }
    }

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

    fn matches_keybinding(&self, _binding: &str, _key: KeyCode, _modifiers: KeyModifiers) -> bool {
        // Keep your existing implementation - this is just a placeholder
        true
    }

    /// Debug method to print loaded keybindings with validation
    pub fn debug_keybindings(&self) {
        println!("📋 Canvas keybindings loaded:");
        println!("  Read-only: {} actions", self.keybindings.read_only.len());
        println!("  Edit: {} actions", self.keybindings.edit.len());

        // NEW: Show validation status against actual implementation
        let validation = self.validate();
        if validation.is_valid {
            println!("  ✅ Configuration matches actual implementation");
        } else {
            println!("  ❌ Configuration has {} errors vs implementation", validation.errors.len());
        }
        if !validation.warnings.is_empty() {
            println!("  ⚠️  Configuration has {} warnings", validation.warnings.len());
        }
    }
}
