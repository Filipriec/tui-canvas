// src/config/validation.rs

use std::collections::HashMap;
use thiserror::Error;
use crate::config::registry::{ActionRegistry, ModeRegistry};
use crate::config::CanvasKeybindings;

#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Missing required action '{action}' in {mode} mode")]
    MissingRequired { 
        action: String, 
        mode: String,
        suggestion: String,
    },
    
    #[error("Unknown action '{action}' in {mode} mode")]
    UnknownAction { 
        action: String, 
        mode: String,
        similar: Vec<String>,
    },
    
    #[error("Multiple validation errors")]
    Multiple(Vec<ValidationError>),
}

#[derive(Debug)]
pub struct ValidationWarning {
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug)]
pub struct ValidationResult {
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub is_valid: bool,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
            is_valid: true,
        }
    }

    pub fn add_error(&mut self, error: ValidationError) {
        self.errors.push(error);
        self.is_valid = false;
    }

    pub fn add_warning(&mut self, warning: ValidationWarning) {
        self.warnings.push(warning);
    }

    pub fn merge(&mut self, other: ValidationResult) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
        if !other.is_valid {
            self.is_valid = false;
        }
    }
}

pub struct ConfigValidator {
    registry: ActionRegistry,
}

impl ConfigValidator {
    pub fn new() -> Self {
        Self {
            registry: ActionRegistry::new(),
        }
    }

    pub fn validate_keybindings(&self, keybindings: &CanvasKeybindings) -> ValidationResult {
        let mut result = ValidationResult::new();

        // Validate each mode
        result.merge(self.validate_mode_bindings(
            "edit", 
            &keybindings.edit, 
            self.registry.get_mode_registry("edit")
        ));

        result.merge(self.validate_mode_bindings(
            "read_only", 
            &keybindings.read_only, 
            self.registry.get_mode_registry("read_only")
        ));

        result.merge(self.validate_mode_bindings(
            "suggestions", 
            &keybindings.suggestions, 
            self.registry.get_mode_registry("suggestions")
        ));

        result.merge(self.validate_mode_bindings(
            "global", 
            &keybindings.global, 
            self.registry.get_mode_registry("global")
        ));

        result
    }

    fn validate_mode_bindings(
        &self, 
        mode_name: &str, 
        bindings: &HashMap<String, Vec<String>>, 
        registry: &ModeRegistry
    ) -> ValidationResult {
        let mut result = ValidationResult::new();

        // Check for missing required actions
        for (action_name, spec) in &registry.required {
            if !bindings.contains_key(action_name) {
                result.add_error(ValidationError::MissingRequired {
                    action: action_name.clone(),
                    mode: mode_name.to_string(),
                    suggestion: format!(
                        "Add to config: {} = {:?}", 
                        action_name, 
                        spec.examples
                    ),
                });
            }
        }

        // Check for unknown actions
        let all_known: std::collections::HashSet<_> = registry.required.keys()
            .chain(registry.optional.keys())
            .collect();

        for action_name in bindings.keys() {
            if !all_known.contains(action_name) {
                let similar = self.find_similar_actions(action_name, &all_known);
                result.add_error(ValidationError::UnknownAction {
                    action: action_name.clone(),
                    mode: mode_name.to_string(),
                    similar,
                });
            }
        }

        // Check for empty keybinding arrays
        for (action_name, key_list) in bindings {
            if key_list.is_empty() {
                result.add_warning(ValidationWarning {
                    message: format!(
                        "Action '{}' in {} mode has empty keybinding list", 
                        action_name, mode_name
                    ),
                    suggestion: Some(format!(
                        "Either add keybindings or remove the action from config"
                    )),
                });
            }
        }

        // Warn about auto-handled actions that shouldn't be in config
        for auto_action in &registry.auto_handled {
            if bindings.contains_key(auto_action) {
                result.add_warning(ValidationWarning {
                    message: format!(
                        "Action '{}' in {} mode is auto-handled and shouldn't be in config", 
                        auto_action, mode_name
                    ),
                    suggestion: Some(format!(
                        "Remove '{}' from config - it's handled automatically", 
                        auto_action
                    )),
                });
            }
        }

        result
    }

    fn find_similar_actions(&self, action: &str, known_actions: &std::collections::HashSet<&String>) -> Vec<String> {
        let mut similar = Vec::new();
        
        for known in known_actions {
            if self.is_similar(action, known) {
                similar.push(known.to_string());
            }
        }

        similar.sort();
        similar.truncate(3); // Limit to 3 suggestions
        similar
    }

    fn is_similar(&self, a: &str, b: &str) -> bool {
        // Simple similarity check - could be improved with proper edit distance
        let a_lower = a.to_lowercase();
        let b_lower = b.to_lowercase();
        
        // Check if one contains the other
        if a_lower.contains(&b_lower) || b_lower.contains(&a_lower) {
            return true;
        }

        // Check for common prefixes
        let common_prefixes = ["move_", "delete_", "suggestion_"];
        for prefix in &common_prefixes {
            if a_lower.starts_with(prefix) && b_lower.starts_with(prefix) {
                return true;
            }
        }

        false
    }

    pub fn print_validation_result(&self, result: &ValidationResult) {
        if result.is_valid && result.warnings.is_empty() {
            println!("✅ Canvas configuration is valid!");
            return;
        }

        if !result.errors.is_empty() {
            println!("❌ Canvas configuration has errors:");
            for error in &result.errors {
                match error {
                    ValidationError::MissingRequired { action, mode, suggestion } => {
                        println!("  • Missing required action '{}' in {} mode", action, mode);
                        println!("    💡 {}", suggestion);
                    }
                    ValidationError::UnknownAction { action, mode, similar } => {
                        println!("  • Unknown action '{}' in {} mode", action, mode);
                        if !similar.is_empty() {
                            println!("    💡 Did you mean: {}", similar.join(", "));
                        }
                    }
                    ValidationError::Multiple(_) => {
                        println!("  • Multiple errors occurred");
                    }
                }
                println!();
            }
        }

        if !result.warnings.is_empty() {
            println!("⚠️  Canvas configuration has warnings:");
            for warning in &result.warnings {
                println!("  • {}", warning.message);
                if let Some(suggestion) = &warning.suggestion {
                    println!("    💡 {}", suggestion);
                }
                println!();
            }
        }

        if !result.is_valid {
            println!("🔧 To generate a config template, use:");
            println!("   CanvasConfig::generate_template()");
        }
    }

    pub fn generate_missing_config(&self, keybindings: &CanvasKeybindings) -> String {
        let mut config = String::new();
        let validation = self.validate_keybindings(keybindings);

        for error in &validation.errors {
            if let ValidationError::MissingRequired { action, mode, suggestion } = error {
                if config.is_empty() {
                    config.push_str(&format!("# Missing required actions for canvas\n\n"));
                    config.push_str(&format!("[keybindings.{}]\n", mode));
                }
                config.push_str(&format!("{}\n", suggestion));
            }
        }

        config
    }
}
