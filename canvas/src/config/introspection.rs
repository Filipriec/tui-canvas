// src/config/introspection.rs
//! Handler capability introspection system
//! 
//! This module provides traits and utilities for handlers to report their capabilities,
//! enabling automatic configuration generation and validation.

use std::collections::HashMap;

/// Specification for a single action that a handler can perform
#[derive(Debug, Clone)]
pub struct ActionSpec {
    /// Action name (e.g., "move_left", "delete_char_backward")
    pub name: String,
    /// Human-readable description of what this action does
    pub description: String,
    /// Example keybindings for this action (e.g., ["Left", "h"])
    pub examples: Vec<String>,
    /// Whether this action is required for the handler to function properly
    pub is_required: bool,
}

/// Complete capability description for a single handler
#[derive(Debug, Clone)]
pub struct HandlerCapabilities {
    /// Mode name this handler operates in (e.g., "edit", "read_only")
    pub mode_name: String,
    /// All actions this handler can perform
    pub actions: Vec<ActionSpec>,
    /// Actions handled automatically without configuration (e.g., "insert_char")
    pub auto_handled: Vec<String>,
}

/// Trait that handlers implement to report their capabilities
/// 
/// This enables the configuration system to automatically discover what actions
/// are available and validate user configurations against actual implementations.
pub trait ActionHandlerIntrospection {
    /// Return complete capability information for this handler
    fn introspect() -> HandlerCapabilities;

    /// Validate that this handler actually supports its claimed actions
    /// Override this to add custom validation logic
    fn validate_capabilities() -> Result<(), String> {
        Ok(()) // Default: assume handler is valid
    }
}

/// Discovers capabilities from all registered handlers
pub struct HandlerDiscovery;

impl HandlerDiscovery {
    /// Discover capabilities from all known handlers
    /// Add new handlers to this function as they are created
    pub fn discover_all() -> HashMap<String, HandlerCapabilities> {
        let mut capabilities = HashMap::new();

        // Register all known handlers here
        let edit_caps = crate::canvas::actions::handlers::edit::EditHandler::introspect();
        capabilities.insert("edit".to_string(), edit_caps);

        let readonly_caps = crate::canvas::actions::handlers::readonly::ReadOnlyHandler::introspect();
        capabilities.insert("read_only".to_string(), readonly_caps);

        let highlight_caps = crate::canvas::actions::handlers::highlight::HighlightHandler::introspect();
        capabilities.insert("highlight".to_string(), highlight_caps);

        capabilities
    }

    /// Validate all handlers support their claimed capabilities
    pub fn validate_all_handlers() -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Validate each handler
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
