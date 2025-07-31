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
    /// Generate complete vim defaults from introspection system
    /// This ensures defaults are always in sync with actual handler capabilities
    pub fn with_vim_defaults() -> Self {
        let registry = ActionRegistry::from_handlers();
        Self::generate_from_registry(&registry)
    }

    /// Generate keybindings from action registry (used by both defaults and config generation)
    /// This is the single source of truth for what keybindings should exist
    fn generate_from_registry(registry: &ActionRegistry) -> Self {
        let mut keybindings = Self::default();

        // Generate keybindings for each mode discovered by introspection
        for (mode_name, mode_registry) in &registry.modes {
            let mode_bindings = match mode_name.as_str() {
                "edit" => &mut keybindings.edit,
                "read_only" => &mut keybindings.read_only,
                "highlight" => &mut keybindings.global, // Highlight actions go in global
                _ => {
                    // Handle any future modes discovered by introspection
                    eprintln!("Warning: Unknown mode '{}' discovered by introspection", mode_name);
                    continue;
                }
            };

            // Add ALL required actions
            for (action_name, action_spec) in &mode_registry.required {
                if !action_spec.examples.is_empty() {
                    mode_bindings.insert(
                        action_name.clone(),
                        action_spec.examples.clone()
                    );
                }
            }

            // Add ALL optional actions
            for (action_name, action_spec) in &mode_registry.optional {
                if !action_spec.examples.is_empty() {
                    mode_bindings.insert(
                        action_name.clone(),
                        action_spec.examples.clone()
                    );
                }
            }
        }

        keybindings
    }

    /// Generate a minimal fallback configuration if introspection fails
    /// This should rarely be used, but provides safety net
    fn minimal_fallback() -> Self {
        let mut keybindings = Self::default();

        // Absolute minimum required for basic functionality
        keybindings.read_only.insert("move_left".to_string(), vec!["h".to_string()]);
        keybindings.read_only.insert("move_right".to_string(), vec!["l".to_string()]);
        keybindings.read_only.insert("move_up".to_string(), vec!["k".to_string()]);
        keybindings.read_only.insert("move_down".to_string(), vec!["j".to_string()]);

        keybindings.edit.insert("delete_char_backward".to_string(), vec!["Backspace".to_string()]);
        keybindings.edit.insert("move_left".to_string(), vec!["Left".to_string()]);
        keybindings.edit.insert("move_right".to_string(), vec!["Right".to_string()]);
        keybindings.edit.insert("move_up".to_string(), vec!["Up".to_string()]);
        keybindings.edit.insert("move_down".to_string(), vec!["Down".to_string()]);

        keybindings
    }

    /// Validate that generated keybindings match the current introspection state
    /// This helps catch when handlers change but defaults become stale
    pub fn validate_against_introspection(&self) -> Result<(), Vec<String>> {
        let registry = ActionRegistry::from_handlers();
        let expected = Self::generate_from_registry(&registry);
        let mut errors = Vec::new();

        // Check each mode
        for (mode_name, expected_bindings) in [
            ("edit", &expected.edit),
            ("read_only", &expected.read_only),
            ("global", &expected.global),
        ] {
            let actual_bindings = match mode_name {
                "edit" => &self.edit,
                "read_only" => &self.read_only,
                "global" => &self.global,
                _ => continue,
            };

            // Check for missing actions
            for action_name in expected_bindings.keys() {
                if !actual_bindings.contains_key(action_name) {
                    errors.push(format!(
                        "Missing action '{}' in {} mode (expected by introspection)",
                        action_name, mode_name
                    ));
                }
            }

            // Check for unexpected actions
            for action_name in actual_bindings.keys() {
                if !expected_bindings.contains_key(action_name) {
                    errors.push(format!(
                        "Unexpected action '{}' in {} mode (not found in introspection)",
                        action_name, mode_name
                    ));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl CanvasConfig {
    /// Enhanced load method with introspection validation
    pub fn load() -> Self {
        match Self::load_and_validate() {
            Ok(config) => config,
            Err(e) => {
                eprintln!("Failed to load config file: {}", e);
                eprintln!("Using auto-generated defaults from introspection");
                Self::default()
            }
        }
    }

    /// Load and validate configuration with enhanced introspection checks
    pub fn load_and_validate() -> Result<Self> {
        // Try to load canvas_config.toml from current directory
        let config = if let Ok(config) = Self::from_file(std::path::Path::new("canvas_config.toml")) {
            config
        } else {
            // Use auto-generated defaults if file doesn't exist
            eprintln!("Config file not found, using auto-generated defaults");
            Self::default()
        };

        // Validate the configuration against current introspection state
        let registry = ActionRegistry::from_handlers();

        // Validate handlers are working correctly
        if let Err(handler_errors) = registry.validate_against_implementation() {
            eprintln!("Handler validation warnings:");
            for error in handler_errors {
                eprintln!("  - {}", error);
            }
        }

        // Validate the configuration against the dynamic registry
        let validator = ConfigValidator::new(registry);
        let validation_result = validator.validate_keybindings(&config.keybindings);

        if !validation_result.is_valid {
            eprintln!("Configuration validation failed:");
            validator.print_validation_result(&validation_result);
        } else if !validation_result.warnings.is_empty() {
            eprintln!("Configuration validation warnings:");
            validator.print_validation_result(&validation_result);
        }

        // Optional: Validate that our defaults match introspection
        if let Err(sync_errors) = config.keybindings.validate_against_introspection() {
            eprintln!("Default keybindings out of sync with introspection:");
            for error in sync_errors {
                eprintln!("  - {}", error);
            }
        }

        Ok(config)
    }

    /// Generate a complete configuration template that matches current defaults
    /// This ensures the generated config file has the same content as defaults
    pub fn generate_complete_template() -> String {
        let registry = ActionRegistry::from_handlers();
        let defaults = CanvasKeybindings::generate_from_registry(&registry);

        let mut template = String::new();
        template.push_str("# Canvas Library Configuration\n");
        template.push_str("# Auto-generated from handler introspection\n");
        template.push_str("# This config contains ALL available actions\n\n");

        // Generate sections for each mode
        for (mode_name, bindings) in [
            ("read_only", &defaults.read_only),
            ("edit", &defaults.edit),
            ("global", &defaults.global),
        ] {
            if bindings.is_empty() {
                continue;
            }

            template.push_str(&format!("[keybindings.{}]\n", mode_name));

            // Get mode registry for categorization
            if let Some(mode_registry) = registry.get_mode_registry(mode_name) {
                // Required actions first
                let mut found_required = false;
                for (action_name, keybindings) in bindings {
                    if mode_registry.required.contains_key(action_name) {
                        if !found_required {
                            template.push_str("# Required\n");
                            found_required = true;
                        }
                        template.push_str(&format!("{} = {:?}\n", action_name, keybindings));
                    }
                }

                // Optional actions second
                let mut found_optional = false;
                for (action_name, keybindings) in bindings {
                    if mode_registry.optional.contains_key(action_name) {
                        if !found_optional {
                            template.push_str("# Optional\n");
                            found_optional = true;
                        }
                        template.push_str(&format!("{} = {:?}\n", action_name, keybindings));
                    }
                }
            } else {
                // Fallback: just list all actions
                for (action_name, keybindings) in bindings {
                    template.push_str(&format!("{} = {:?}\n", action_name, keybindings));
                }
            }

            template.push('\n');
        }

        template
    }

    /// Generate config that only contains actions different from defaults
    /// Useful for minimal user configs
    pub fn generate_minimal_template() -> String {
        let defaults = CanvasKeybindings::with_vim_defaults();

        let mut template = String::new();
        template.push_str("# Minimal Canvas Configuration\n");
        template.push_str("# Only uncomment and modify the keybindings you want to change\n");
        template.push_str("# All other actions will use their default vim-style keybindings\n\n");

        for (mode_name, bindings) in [
            ("read_only", &defaults.read_only),
            ("edit", &defaults.edit),
            ("global", &defaults.global),
        ] {
            if bindings.is_empty() {
                continue;
            }

            template.push_str(&format!("# [keybindings.{}]\n", mode_name));

            for (action_name, keybindings) in bindings {
                template.push_str(&format!("# {} = {:?}\n", action_name, keybindings));
            }

            template.push('\n');
        }

        template
    }

    /// Generate template from actual handler capabilities (legacy method for compatibility)
    pub fn generate_template() -> String {
        Self::generate_complete_template()
    }

    /// Generate clean template from actual handler capabilities (legacy method for compatibility)
    pub fn generate_clean_template() -> String {
        let registry = ActionRegistry::from_handlers();
        
        // Validate handlers first
        if let Err(errors) = registry.validate_against_implementation() {
            for error in errors {
                eprintln!("   - {}", error);
            }
        }

        registry.generate_clean_template()
    }

    /// Validate current configuration against actual implementation
    pub fn validate(&self) -> ValidationResult {
        let registry = ActionRegistry::from_handlers();
        let validator = ConfigValidator::new(registry);
        validator.validate_keybindings(&self.keybindings)
    }

    /// Print validation results for current config
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

    /// Check if autocomplete should auto-trigger (simple logic)
    pub fn should_auto_trigger_autocomplete(&self) -> bool {
        // If trigger_autocomplete keybinding exists anywhere, use manual mode only
        // If no trigger_autocomplete keybinding, use auto-trigger mode
        !self.has_trigger_autocomplete_keybinding()
    }

    /// Check if user has configured manual trigger keybinding
    pub fn has_trigger_autocomplete_keybinding(&self) -> bool {
        self.keybindings.edit.contains_key("trigger_autocomplete") ||
        self.keybindings.read_only.contains_key("trigger_autocomplete") ||
        self.keybindings.global.contains_key("trigger_autocomplete")
    }

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

    fn matches_keybinding(&self, binding: &str, key: KeyCode, modifiers: KeyModifiers) -> bool {
        // Special handling for shift+character combinations
        if binding.to_lowercase().starts_with("shift+") {
            let parts: Vec<&str> = binding.split('+').collect();
            if parts.len() == 2 && parts[1].len() == 1 {
                let expected_lowercase = parts[1].chars().next().unwrap().to_lowercase().next().unwrap();
                let expected_uppercase = expected_lowercase.to_uppercase().next().unwrap();
                if let KeyCode::Char(actual_char) = key {
                    if actual_char == expected_uppercase && modifiers.contains(KeyModifiers::SHIFT) {
                        return true;
                    }
                }
            }
        }

        // Handle Shift+Tab -> BackTab
        if binding.to_lowercase() == "shift+tab" && key == KeyCode::BackTab && modifiers.is_empty() {
            return true;
        }

        // Handle multi-character bindings (all standard keys without modifiers)
        if binding.len() > 1 && !binding.contains('+') {
            return match binding.to_lowercase().as_str() {
                // Navigation keys
                "left" => key == KeyCode::Left,
                "right" => key == KeyCode::Right,
                "up" => key == KeyCode::Up,
                "down" => key == KeyCode::Down,
                "home" => key == KeyCode::Home,
                "end" => key == KeyCode::End,
                "pageup" | "pgup" => key == KeyCode::PageUp,
                "pagedown" | "pgdn" => key == KeyCode::PageDown,

                // Editing keys
                "insert" | "ins" => key == KeyCode::Insert,
                "delete" | "del" => key == KeyCode::Delete,
                "backspace" => key == KeyCode::Backspace,

                // Tab keys
                "tab" => key == KeyCode::Tab,
                "backtab" => key == KeyCode::BackTab,

                // Special keys
                "enter" | "return" => key == KeyCode::Enter,
                "escape" | "esc" => key == KeyCode::Esc,
                "space" => key == KeyCode::Char(' '),

                // Function keys F1-F24
                "f1" => key == KeyCode::F(1),
                "f2" => key == KeyCode::F(2),
                "f3" => key == KeyCode::F(3),
                "f4" => key == KeyCode::F(4),
                "f5" => key == KeyCode::F(5),
                "f6" => key == KeyCode::F(6),
                "f7" => key == KeyCode::F(7),
                "f8" => key == KeyCode::F(8),
                "f9" => key == KeyCode::F(9),
                "f10" => key == KeyCode::F(10),
                "f11" => key == KeyCode::F(11),
                "f12" => key == KeyCode::F(12),
                "f13" => key == KeyCode::F(13),
                "f14" => key == KeyCode::F(14),
                "f15" => key == KeyCode::F(15),
                "f16" => key == KeyCode::F(16),
                "f17" => key == KeyCode::F(17),
                "f18" => key == KeyCode::F(18),
                "f19" => key == KeyCode::F(19),
                "f20" => key == KeyCode::F(20),
                "f21" => key == KeyCode::F(21),
                "f22" => key == KeyCode::F(22),
                "f23" => key == KeyCode::F(23),
                "f24" => key == KeyCode::F(24),

                // Lock keys (may not work reliably in all terminals)
                "capslock" => key == KeyCode::CapsLock,
                "scrolllock" => key == KeyCode::ScrollLock,
                "numlock" => key == KeyCode::NumLock,

                // System keys
                "printscreen" => key == KeyCode::PrintScreen,
                "pause" => key == KeyCode::Pause,
                "menu" => key == KeyCode::Menu,
                "keypadbegin" => key == KeyCode::KeypadBegin,

                // Media keys (rarely supported but included for completeness)
                "mediaplay" => key == KeyCode::Media(crossterm::event::MediaKeyCode::Play),
                "mediapause" => key == KeyCode::Media(crossterm::event::MediaKeyCode::Pause),
                "mediaplaypause" => key == KeyCode::Media(crossterm::event::MediaKeyCode::PlayPause),
                "mediareverse" => key == KeyCode::Media(crossterm::event::MediaKeyCode::Reverse),
                "mediastop" => key == KeyCode::Media(crossterm::event::MediaKeyCode::Stop),
                "mediafastforward" => key == KeyCode::Media(crossterm::event::MediaKeyCode::FastForward),
                "mediarewind" => key == KeyCode::Media(crossterm::event::MediaKeyCode::Rewind),
                "mediatracknext" => key == KeyCode::Media(crossterm::event::MediaKeyCode::TrackNext),
                "mediatrackprevious" => key == KeyCode::Media(crossterm::event::MediaKeyCode::TrackPrevious),
                "mediarecord" => key == KeyCode::Media(crossterm::event::MediaKeyCode::Record),
                "medialowervolume" => key == KeyCode::Media(crossterm::event::MediaKeyCode::LowerVolume),
                "mediaraisevolume" => key == KeyCode::Media(crossterm::event::MediaKeyCode::RaiseVolume),
                "mediamutevolume" => key == KeyCode::Media(crossterm::event::MediaKeyCode::MuteVolume),

                // Modifier keys (these work better as part of combinations)
                "leftshift" => key == KeyCode::Modifier(crossterm::event::ModifierKeyCode::LeftShift),
                "leftcontrol" | "leftctrl" => key == KeyCode::Modifier(crossterm::event::ModifierKeyCode::LeftControl),
                "leftalt" => key == KeyCode::Modifier(crossterm::event::ModifierKeyCode::LeftAlt),
                "leftsuper" | "leftwindows" | "leftcmd" => key == KeyCode::Modifier(crossterm::event::ModifierKeyCode::LeftSuper),
                "lefthyper" => key == KeyCode::Modifier(crossterm::event::ModifierKeyCode::LeftHyper),
                "leftmeta" => key == KeyCode::Modifier(crossterm::event::ModifierKeyCode::LeftMeta),
                "rightshift" => key == KeyCode::Modifier(crossterm::event::ModifierKeyCode::RightShift),
                "rightcontrol" | "rightctrl" => key == KeyCode::Modifier(crossterm::event::ModifierKeyCode::RightControl),
                "rightalt" => key == KeyCode::Modifier(crossterm::event::ModifierKeyCode::RightAlt),
                "rightsuper" | "rightwindows" | "rightcmd" => key == KeyCode::Modifier(crossterm::event::ModifierKeyCode::RightSuper),
                "righthyper" => key == KeyCode::Modifier(crossterm::event::ModifierKeyCode::RightHyper),
                "rightmeta" => key == KeyCode::Modifier(crossterm::event::ModifierKeyCode::RightMeta),
                "isolevel3shift" => key == KeyCode::Modifier(crossterm::event::ModifierKeyCode::IsoLevel3Shift),
                "isolevel5shift" => key == KeyCode::Modifier(crossterm::event::ModifierKeyCode::IsoLevel5Shift),

                // Multi-key sequences need special handling
                "gg" => false, // This needs sequence handling
                _ => {
                    // Handle single characters and punctuation
                    if binding.len() == 1 {
                        if let Some(c) = binding.chars().next() {
                            key == KeyCode::Char(c)
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
            };
        }

        // Handle modifier combinations (like "Ctrl+F5", "Alt+Shift+A")
        let parts: Vec<&str> = binding.split('+').collect();
        let mut expected_modifiers = KeyModifiers::empty();
        let mut expected_key = None;

        for part in parts {
            match part.to_lowercase().as_str() {
                // Modifiers
                "ctrl" | "control" => expected_modifiers |= KeyModifiers::CONTROL,
                "shift" => expected_modifiers |= KeyModifiers::SHIFT,
                "alt" => expected_modifiers |= KeyModifiers::ALT,
                "super" | "windows" | "cmd" => expected_modifiers |= KeyModifiers::SUPER,
                "hyper" => expected_modifiers |= KeyModifiers::HYPER,
                "meta" => expected_modifiers |= KeyModifiers::META,

                // Navigation keys
                "left" => expected_key = Some(KeyCode::Left),
                "right" => expected_key = Some(KeyCode::Right),
                "up" => expected_key = Some(KeyCode::Up),
                "down" => expected_key = Some(KeyCode::Down),
                "home" => expected_key = Some(KeyCode::Home),
                "end" => expected_key = Some(KeyCode::End),
                "pageup" | "pgup" => expected_key = Some(KeyCode::PageUp),
                "pagedown" | "pgdn" => expected_key = Some(KeyCode::PageDown),

                // Editing keys
                "insert" | "ins" => expected_key = Some(KeyCode::Insert),
                "delete" | "del" => expected_key = Some(KeyCode::Delete),
                "backspace" => expected_key = Some(KeyCode::Backspace),

                // Tab keys
                "tab" => expected_key = Some(KeyCode::Tab),
                "backtab" => expected_key = Some(KeyCode::BackTab),

                // Special keys
                "enter" | "return" => expected_key = Some(KeyCode::Enter),
                "escape" | "esc" => expected_key = Some(KeyCode::Esc),
                "space" => expected_key = Some(KeyCode::Char(' ')),

                // Function keys
                "f1" => expected_key = Some(KeyCode::F(1)),
                "f2" => expected_key = Some(KeyCode::F(2)),
                "f3" => expected_key = Some(KeyCode::F(3)),
                "f4" => expected_key = Some(KeyCode::F(4)),
                "f5" => expected_key = Some(KeyCode::F(5)),
                "f6" => expected_key = Some(KeyCode::F(6)),
                "f7" => expected_key = Some(KeyCode::F(7)),
                "f8" => expected_key = Some(KeyCode::F(8)),
                "f9" => expected_key = Some(KeyCode::F(9)),
                "f10" => expected_key = Some(KeyCode::F(10)),
                "f11" => expected_key = Some(KeyCode::F(11)),
                "f12" => expected_key = Some(KeyCode::F(12)),
                "f13" => expected_key = Some(KeyCode::F(13)),
                "f14" => expected_key = Some(KeyCode::F(14)),
                "f15" => expected_key = Some(KeyCode::F(15)),
                "f16" => expected_key = Some(KeyCode::F(16)),
                "f17" => expected_key = Some(KeyCode::F(17)),
                "f18" => expected_key = Some(KeyCode::F(18)),
                "f19" => expected_key = Some(KeyCode::F(19)),
                "f20" => expected_key = Some(KeyCode::F(20)),
                "f21" => expected_key = Some(KeyCode::F(21)),
                "f22" => expected_key = Some(KeyCode::F(22)),
                "f23" => expected_key = Some(KeyCode::F(23)),
                "f24" => expected_key = Some(KeyCode::F(24)),

                // Lock keys
                "capslock" => expected_key = Some(KeyCode::CapsLock),
                "scrolllock" => expected_key = Some(KeyCode::ScrollLock),
                "numlock" => expected_key = Some(KeyCode::NumLock),

                // System keys
                "printscreen" => expected_key = Some(KeyCode::PrintScreen),
                "pause" => expected_key = Some(KeyCode::Pause),
                "menu" => expected_key = Some(KeyCode::Menu),
                "keypadbegin" => expected_key = Some(KeyCode::KeypadBegin),

                // Single character (letters, numbers, punctuation)
                part => {
                    if part.len() == 1 {
                        if let Some(c) = part.chars().next() {
                            expected_key = Some(KeyCode::Char(c));
                        }
                    }
                }
            }
        }

        modifiers == expected_modifiers && Some(key) == expected_key
    }
}
