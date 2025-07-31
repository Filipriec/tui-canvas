// src/config/introspection.rs

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ActionSpec {
    pub name: String,
    pub description: String,
    pub examples: Vec<String>,
    pub is_required: bool,
}

#[derive(Debug, Clone)]
pub struct HandlerCapabilities {
    pub mode_name: String,
    pub actions: Vec<ActionSpec>,
    pub auto_handled: Vec<String>, // Actions handled automatically (like insert_char)
}

/// Trait that each handler implements to report its capabilities
pub trait ActionHandlerIntrospection {
    /// Return the capabilities of this handler
    fn introspect() -> HandlerCapabilities;

    /// Validate that this handler actually supports the claimed actions
    fn validate_capabilities() -> Result<(), String> {
        // Default implementation - handlers can override for custom validation
        Ok(())
    }
}

/// System that discovers all handler capabilities
pub struct HandlerDiscovery;

impl HandlerDiscovery {
    /// Discover all handler capabilities by calling their introspect methods
    pub fn discover_all() -> HashMap<String, HandlerCapabilities> {
        let mut capabilities = HashMap::new();

        // Import and introspect each handler
        let edit_caps = crate::canvas::actions::handlers::edit::EditHandler::introspect();
        capabilities.insert("edit".to_string(), edit_caps);

        let readonly_caps = crate::canvas::actions::handlers::readonly::ReadOnlyHandler::introspect();
        capabilities.insert("read_only".to_string(), readonly_caps);

        let highlight_caps = crate::canvas::actions::handlers::highlight::HighlightHandler::introspect();
        capabilities.insert("highlight".to_string(), highlight_caps);

        capabilities
    }

    /// Validate that all handlers actually support their claimed actions
    pub fn validate_all_handlers() -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if let Err(e) = crate::canvas::actions::handlers::edit::EditHandler::validate_capabilities() {
            errors.push(format!("Edit handler: {}", e));
        }

        if let Err(e) = crate::canvas::actions::handlers::readonly::ReadOnlyHandler::validate_capabilities() {
            errors.push(format!("ReadOnly handler: {}", e));
        }

        if let Err(e) = crate::canvas::actions::handlers::highlight::HighlightHandler::validate_capabilities() {
            errors.push(format!("Highlight handler: {}", e));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
