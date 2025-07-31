// src/config/registry.rs

use std::collections::HashMap;
use crate::config::introspection::{HandlerDiscovery, ActionSpec, HandlerCapabilities};

#[derive(Debug, Clone)]
pub struct ModeRegistry {
    pub required: HashMap<String, ActionSpec>,
    pub optional: HashMap<String, ActionSpec>,
    pub auto_handled: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ActionRegistry {
    pub modes: HashMap<String, ModeRegistry>,
}

impl ActionRegistry {
    /// NEW: Create registry by discovering actual handler capabilities
    pub fn from_handlers() -> Self {
        let handler_capabilities = HandlerDiscovery::discover_all();
        let mut modes = HashMap::new();
        
        for (mode_name, capabilities) in handler_capabilities {
            let mode_registry = Self::build_mode_registry(capabilities);
            modes.insert(mode_name, mode_registry);
        }
        
        Self { modes }
    }
    
    /// Build a mode registry from handler capabilities
    fn build_mode_registry(capabilities: HandlerCapabilities) -> ModeRegistry {
        let mut required = HashMap::new();
        let mut optional = HashMap::new();
        
        for action_spec in capabilities.actions {
            if action_spec.is_required {
                required.insert(action_spec.name.clone(), action_spec);
            } else {
                optional.insert(action_spec.name.clone(), action_spec);
            }
        }
        
        ModeRegistry {
            required,
            optional,
            auto_handled: capabilities.auto_handled,
        }
    }
    
    /// Validate that the registry matches the actual implementation
    pub fn validate_against_implementation(&self) -> Result<(), Vec<String>> {
        HandlerDiscovery::validate_all_handlers()
    }
    
    pub fn get_mode_registry(&self, mode: &str) -> Option<&ModeRegistry> {
        self.modes.get(mode)
    }

    pub fn all_known_actions(&self) -> Vec<String> {
        let mut actions = Vec::new();
        
        for registry in self.modes.values() {
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
        template.push_str("# Generated automatically from actual handler capabilities\n\n");

        for (mode_name, registry) in &self.modes {
            template.push_str(&format!("[keybindings.{}]\n", mode_name));
            
            if !registry.required.is_empty() {
                template.push_str("# REQUIRED ACTIONS - These must be configured\n");
                for (name, spec) in &registry.required {
                    template.push_str(&format!("# {}\n", spec.description));
                    template.push_str(&format!("{} = {:?}\n\n", name, spec.examples));
                }
            }
            
            if !registry.optional.is_empty() {
                template.push_str("# OPTIONAL ACTIONS - Configure these if you want them enabled\n");
                for (name, spec) in &registry.optional {
                    template.push_str(&format!("# {}\n", spec.description));
                    template.push_str(&format!("# {} = {:?}\n\n", name, spec.examples));
                }
            }
            
            if !registry.auto_handled.is_empty() {
                template.push_str("# AUTO-HANDLED - These are handled automatically, don't configure:\n");
                for auto_action in &registry.auto_handled {
                    template.push_str(&format!("# {} (automatic)\n", auto_action));
                }
                template.push('\n');
            }
        }

        template
    }

    pub fn generate_clean_template(&self) -> String {
        let mut template = String::new();

        for (mode_name, registry) in &self.modes {
            template.push_str(&format!("[keybindings.{}]\n", mode_name));
            
            if !registry.required.is_empty() {
                template.push_str("# Required\n");
                for (name, spec) in &registry.required {
                    template.push_str(&format!("{} = {:?}\n", name, spec.examples));
                }
            }
            
            if !registry.optional.is_empty() {
                template.push_str("# Optional\n");
                for (name, spec) in &registry.optional {
                    template.push_str(&format!("{} = {:?}\n", name, spec.examples));
                }
            }
            
            template.push('\n');
        }

        template
    }
}
