// src/config/registry.rs

use std::collections::HashMap;
use crate::canvas::modes::AppMode;

#[derive(Debug, Clone)]
pub struct ActionSpec {
    pub name: String,
    pub description: String,
    pub examples: Vec<String>,
    pub mode_specific: bool, // true if different behavior per mode
}

#[derive(Debug, Clone)]
pub struct ModeRegistry {
    pub required: HashMap<String, ActionSpec>,
    pub optional: HashMap<String, ActionSpec>,
    pub auto_handled: Vec<String>, // Never appear in config
}

#[derive(Debug, Clone)]
pub struct ActionRegistry {
    pub edit_mode: ModeRegistry,
    pub readonly_mode: ModeRegistry,
    pub suggestions: ModeRegistry,
    pub global: ModeRegistry,
}

impl ActionRegistry {
    pub fn new() -> Self {
        Self {
            edit_mode: Self::edit_mode_registry(),
            readonly_mode: Self::readonly_mode_registry(),
            suggestions: Self::suggestions_registry(),
            global: Self::global_registry(),
        }
    }

    fn edit_mode_registry() -> ModeRegistry {
        let mut required = HashMap::new();
        let mut optional = HashMap::new();

        // REQUIRED - These MUST be configured
        required.insert("move_left".to_string(), ActionSpec {
            name: "move_left".to_string(),
            description: "Move cursor one position to the left".to_string(),
            examples: vec!["Left".to_string(), "h".to_string()],
            mode_specific: false,
        });
        
        required.insert("move_right".to_string(), ActionSpec {
            name: "move_right".to_string(),
            description: "Move cursor one position to the right".to_string(),
            examples: vec!["Right".to_string(), "l".to_string()],
            mode_specific: false,
        });

        required.insert("move_up".to_string(), ActionSpec {
            name: "move_up".to_string(),
            description: "Move to previous field or line".to_string(),
            examples: vec!["Up".to_string(), "k".to_string()],
            mode_specific: false,
        });

        required.insert("move_down".to_string(), ActionSpec {
            name: "move_down".to_string(),
            description: "Move to next field or line".to_string(),
            examples: vec!["Down".to_string(), "j".to_string()],
            mode_specific: false,
        });

        required.insert("delete_char_backward".to_string(), ActionSpec {
            name: "delete_char_backward".to_string(),
            description: "Delete character before cursor".to_string(),
            examples: vec!["Backspace".to_string()],
            mode_specific: false,
        });

        required.insert("next_field".to_string(), ActionSpec {
            name: "next_field".to_string(),
            description: "Move to next input field".to_string(),
            examples: vec!["Tab".to_string(), "Enter".to_string()],
            mode_specific: false,
        });

        required.insert("prev_field".to_string(), ActionSpec {
            name: "prev_field".to_string(),
            description: "Move to previous input field".to_string(),
            examples: vec!["Shift+Tab".to_string()],
            mode_specific: false,
        });

        // OPTIONAL - These can be configured or omitted
        optional.insert("move_word_next".to_string(), ActionSpec {
            name: "move_word_next".to_string(),
            description: "Move cursor to start of next word".to_string(),
            examples: vec!["Ctrl+Right".to_string(), "w".to_string()],
            mode_specific: false,
        });

        optional.insert("move_word_prev".to_string(), ActionSpec {
            name: "move_word_prev".to_string(),
            description: "Move cursor to start of previous word".to_string(),
            examples: vec!["Ctrl+Left".to_string(), "b".to_string()],
            mode_specific: false,
        });

        optional.insert("move_word_end".to_string(), ActionSpec {
            name: "move_word_end".to_string(),
            description: "Move cursor to end of current/next word".to_string(),
            examples: vec!["e".to_string()],
            mode_specific: false,
        });

        optional.insert("move_word_end_prev".to_string(), ActionSpec {
            name: "move_word_end_prev".to_string(),
            description: "Move cursor to end of previous word".to_string(),
            examples: vec!["ge".to_string()],
            mode_specific: false,
        });

        optional.insert("move_line_start".to_string(), ActionSpec {
            name: "move_line_start".to_string(),
            description: "Move cursor to beginning of line".to_string(),
            examples: vec!["Home".to_string(), "0".to_string()],
            mode_specific: false,
        });

        optional.insert("move_line_end".to_string(), ActionSpec {
            name: "move_line_end".to_string(),
            description: "Move cursor to end of line".to_string(),
            examples: vec!["End".to_string(), "$".to_string()],
            mode_specific: false,
        });

        optional.insert("move_first_line".to_string(), ActionSpec {
            name: "move_first_line".to_string(),
            description: "Move to first field".to_string(),
            examples: vec!["Ctrl+Home".to_string(), "gg".to_string()],
            mode_specific: false,
        });

        optional.insert("move_last_line".to_string(), ActionSpec {
            name: "move_last_line".to_string(),
            description: "Move to last field".to_string(),
            examples: vec!["Ctrl+End".to_string(), "G".to_string()],
            mode_specific: false,
        });

        optional.insert("delete_char_forward".to_string(), ActionSpec {
            name: "delete_char_forward".to_string(),
            description: "Delete character after cursor".to_string(),
            examples: vec!["Delete".to_string()],
            mode_specific: false,
        });

        ModeRegistry {
            required,
            optional,
            auto_handled: vec![
                "insert_char".to_string(), // Any printable character
            ],
        }
    }

    fn readonly_mode_registry() -> ModeRegistry {
        let mut required = HashMap::new();
        let mut optional = HashMap::new();

        // REQUIRED - Navigation is essential in read-only mode
        required.insert("move_left".to_string(), ActionSpec {
            name: "move_left".to_string(),
            description: "Move cursor one position to the left".to_string(),
            examples: vec!["h".to_string(), "Left".to_string()],
            mode_specific: true,
        });
        
        required.insert("move_right".to_string(), ActionSpec {
            name: "move_right".to_string(),
            description: "Move cursor one position to the right".to_string(),
            examples: vec!["l".to_string(), "Right".to_string()],
            mode_specific: true,
        });

        required.insert("move_up".to_string(), ActionSpec {
            name: "move_up".to_string(),
            description: "Move to previous field".to_string(),
            examples: vec!["k".to_string(), "Up".to_string()],
            mode_specific: true,
        });

        required.insert("move_down".to_string(), ActionSpec {
            name: "move_down".to_string(),
            description: "Move to next field".to_string(),
            examples: vec!["j".to_string(), "Down".to_string()],
            mode_specific: true,
        });

        // OPTIONAL - Advanced navigation
        optional.insert("move_word_next".to_string(), ActionSpec {
            name: "move_word_next".to_string(),
            description: "Move cursor to start of next word".to_string(),
            examples: vec!["w".to_string()],
            mode_specific: true,
        });

        optional.insert("move_word_prev".to_string(), ActionSpec {
            name: "move_word_prev".to_string(),
            description: "Move cursor to start of previous word".to_string(),
            examples: vec!["b".to_string()],
            mode_specific: true,
        });

        optional.insert("move_word_end".to_string(), ActionSpec {
            name: "move_word_end".to_string(),
            description: "Move cursor to end of current/next word".to_string(),
            examples: vec!["e".to_string()],
            mode_specific: true,
        });

        optional.insert("move_word_end_prev".to_string(), ActionSpec {
            name: "move_word_end_prev".to_string(),
            description: "Move cursor to end of previous word".to_string(),
            examples: vec!["ge".to_string()],
            mode_specific: true,
        });

        optional.insert("move_line_start".to_string(), ActionSpec {
            name: "move_line_start".to_string(),
            description: "Move cursor to beginning of line".to_string(),
            examples: vec!["0".to_string()],
            mode_specific: true,
        });

        optional.insert("move_line_end".to_string(), ActionSpec {
            name: "move_line_end".to_string(),
            description: "Move cursor to end of line".to_string(),
            examples: vec!["$".to_string()],
            mode_specific: true,
        });

        optional.insert("move_first_line".to_string(), ActionSpec {
            name: "move_first_line".to_string(),
            description: "Move to first field".to_string(),
            examples: vec!["gg".to_string()],
            mode_specific: true,
        });

        optional.insert("move_last_line".to_string(), ActionSpec {
            name: "move_last_line".to_string(),
            description: "Move to last field".to_string(),
            examples: vec!["G".to_string()],
            mode_specific: true,
        });

        optional.insert("next_field".to_string(), ActionSpec {
            name: "next_field".to_string(),
            description: "Move to next input field".to_string(),
            examples: vec!["Tab".to_string()],
            mode_specific: true,
        });

        optional.insert("prev_field".to_string(), ActionSpec {
            name: "prev_field".to_string(),
            description: "Move to previous input field".to_string(),
            examples: vec!["Shift+Tab".to_string()],
            mode_specific: true,
        });

        ModeRegistry {
            required,
            optional,
            auto_handled: vec![], // Read-only mode has no auto-handled actions
        }
    }

    fn suggestions_registry() -> ModeRegistry {
        let mut required = HashMap::new();

        // REQUIRED - Essential for suggestion navigation
        required.insert("suggestion_up".to_string(), ActionSpec {
            name: "suggestion_up".to_string(),
            description: "Move selection to previous suggestion".to_string(),
            examples: vec!["Up".to_string(), "Ctrl+p".to_string()],
            mode_specific: false,
        });

        required.insert("suggestion_down".to_string(), ActionSpec {
            name: "suggestion_down".to_string(),
            description: "Move selection to next suggestion".to_string(),
            examples: vec!["Down".to_string(), "Ctrl+n".to_string()],
            mode_specific: false,
        });

        required.insert("select_suggestion".to_string(), ActionSpec {
            name: "select_suggestion".to_string(),
            description: "Select the currently highlighted suggestion".to_string(),
            examples: vec!["Enter".to_string(), "Tab".to_string()],
            mode_specific: false,
        });

        required.insert("exit_suggestions".to_string(), ActionSpec {
            name: "exit_suggestions".to_string(),
            description: "Close suggestions without selecting".to_string(),
            examples: vec!["Esc".to_string()],
            mode_specific: false,
        });

        ModeRegistry {
            required,
            optional: HashMap::new(),
            auto_handled: vec![],
        }
    }

    fn global_registry() -> ModeRegistry {
        let mut optional = HashMap::new();

        // OPTIONAL - Global overrides
        optional.insert("move_up".to_string(), ActionSpec {
            name: "move_up".to_string(),
            description: "Global override for up movement".to_string(),
            examples: vec!["Up".to_string()],
            mode_specific: false,
        });

        optional.insert("move_down".to_string(), ActionSpec {
            name: "move_down".to_string(),
            description: "Global override for down movement".to_string(),
            examples: vec!["Down".to_string()],
            mode_specific: false,
        });

        ModeRegistry {
            required: HashMap::new(),
            optional,
            auto_handled: vec![],
        }
    }

    pub fn get_mode_registry(&self, mode: &str) -> &ModeRegistry {
        match mode {
            "edit" => &self.edit_mode,
            "read_only" => &self.readonly_mode,
            "suggestions" => &self.suggestions,
            "global" => &self.global,
            _ => &self.global, // fallback
        }
    }

    pub fn all_known_actions(&self) -> Vec<String> {
        let mut actions = Vec::new();
        
        for registry in [&self.edit_mode, &self.readonly_mode, &self.suggestions, &self.global] {
            actions.extend(registry.required.keys().cloned());
            actions.extend(registry.optional.keys().cloned());
        }
        
        actions.sort();
        actions.dedup();
        actions
    }

    pub fn generate_config_template(&self) -> String {
        let mut template = String::new();
        template.push_str("# Canvas Library Configuration Template\n");
        template.push_str("# Generated automatically - customize as needed\n\n");

        template.push_str("[keybindings.edit]\n");
        template.push_str("# REQUIRED ACTIONS - These must be configured\n");
        for (name, spec) in &self.edit_mode.required {
            template.push_str(&format!("# {}\n", spec.description));
            template.push_str(&format!("{} = {:?}\n\n", name, spec.examples));
        }
        
        template.push_str("# OPTIONAL ACTIONS - Configure these if you want them enabled\n");
        for (name, spec) in &self.edit_mode.optional {
            template.push_str(&format!("# {}\n", spec.description));
            template.push_str(&format!("# {} = {:?}\n\n", name, spec.examples));
        }

        template.push_str("[keybindings.read_only]\n");
        template.push_str("# REQUIRED ACTIONS - These must be configured\n");
        for (name, spec) in &self.readonly_mode.required {
            template.push_str(&format!("# {}\n", spec.description));
            template.push_str(&format!("{} = {:?}\n\n", name, spec.examples));
        }

        template.push_str("# OPTIONAL ACTIONS - Configure these if you want them enabled\n");
        for (name, spec) in &self.readonly_mode.optional {
            template.push_str(&format!("# {}\n", spec.description));
            template.push_str(&format!("# {} = {:?}\n\n", name, spec.examples));
        }

        template.push_str("[keybindings.suggestions]\n");
        template.push_str("# REQUIRED ACTIONS - These must be configured\n");
        for (name, spec) in &self.suggestions.required {
            template.push_str(&format!("# {}\n", spec.description));
            template.push_str(&format!("{} = {:?}\n\n", name, spec.examples));
        }

        template
    }

    pub fn generate_clean_template(&self) -> String {
        let mut template = String::new();

        // Edit Mode
        template.push_str("[keybindings.edit]\n");
        template.push_str("# Required\n");
        for (name, spec) in &self.edit_mode.required {
            template.push_str(&format!("{} = {:?}\n", name, spec.examples));
        }
        template.push_str("# Optional\n");
        for (name, spec) in &self.edit_mode.optional {
            template.push_str(&format!("{} = {:?}\n", name, spec.examples));
        }
        template.push('\n');

        // Read-Only Mode  
        template.push_str("[keybindings.read_only]\n");
        template.push_str("# Required\n");
        for (name, spec) in &self.readonly_mode.required {
            template.push_str(&format!("{} = {:?}\n", name, spec.examples));
        }
        template.push_str("# Optional\n");
        for (name, spec) in &self.readonly_mode.optional {
            template.push_str(&format!("{} = {:?}\n", name, spec.examples));
        }
        template.push('\n');

        // Suggestions Mode
        template.push_str("[keybindings.suggestions]\n");
        template.push_str("# Required\n");
        for (name, spec) in &self.suggestions.required {
            template.push_str(&format!("{} = {:?}\n", name, spec.examples));
        }
        template.push('\n');

        // Global (all optional)
        if !self.global.optional.is_empty() {
            template.push_str("[keybindings.global]\n");
            template.push_str("# Optional\n");
            for (name, spec) in &self.global.optional {
                template.push_str(&format!("{} = {:?}\n", name, spec.examples));
            }
        }

        template
    }
}
