// src/canvas/state.rs
//! Canvas state trait and related types
//! 
//! This module defines the core trait that any form or input system must implement
//! to work with the canvas library.

use crate::canvas::actions::CanvasAction;
use crate::canvas::modes::AppMode;

/// Context information passed to feature-specific action handlers
#[derive(Debug)]
pub struct ActionContext {
    /// Original key code that triggered this action (for backwards compatibility)
    pub key_code: Option<crossterm::event::KeyCode>,
    /// Current ideal cursor column for vertical movement
    pub ideal_cursor_column: usize,
    /// Current input text
    pub current_input: String,
    /// Current field index
    pub current_field: usize,
}

/// Core trait that any form-like state must implement to work with canvas
/// 
/// This trait enables the same mode behaviors (edit, read-only, highlight) to work
/// across any implementation - login forms, data entry forms, configuration screens, etc.
/// 
/// # Required Implementation
/// 
/// Your struct needs to track:
/// - Current field index and cursor position
/// - All input field values 
/// - Current interaction mode
/// - Whether there are unsaved changes
/// 
/// # Example Implementation
/// 
/// ```rust
/// struct MyForm {
///     fields: Vec<String>,
///     current_field: usize, 
///     cursor_pos: usize,
///     mode: AppMode,
///     dirty: bool,
/// }
/// 
/// impl CanvasState for MyForm {
///     fn current_field(&self) -> usize { self.current_field }
///     fn current_cursor_pos(&self) -> usize { self.cursor_pos }
///     // ... implement other required methods
/// }
/// ```
pub trait CanvasState {
    // --- Core Navigation ---
    
    /// Get current field index (0-based)
    fn current_field(&self) -> usize;
    
    /// Get current cursor position within the current field
    fn current_cursor_pos(&self) -> usize;
    
    /// Set current field index (should clamp to valid range)
    fn set_current_field(&mut self, index: usize);
    
    /// Set cursor position within current field (should clamp to valid range)
    fn set_current_cursor_pos(&mut self, pos: usize);

    // --- Mode Information ---
    
    /// Get current interaction mode (edit, read-only, highlight, etc.)
    fn current_mode(&self) -> AppMode;

    // --- Data Access ---
    
    /// Get immutable reference to current field's text
    fn get_current_input(&self) -> &str;
    
    /// Get mutable reference to current field's text
    fn get_current_input_mut(&mut self) -> &mut String;
    
    /// Get all input values as immutable references
    fn inputs(&self) -> Vec<&String>;
    
    /// Get all field names/labels
    fn fields(&self) -> Vec<&str>;

    // --- State Management ---
    
    /// Check if there are unsaved changes
    fn has_unsaved_changes(&self) -> bool;
    
    /// Mark whether there are unsaved changes
    fn set_has_unsaved_changes(&mut self, changed: bool);

    // --- Optional Overrides ---

    /// Handle application-specific actions not covered by standard handlers
    /// Return Some(message) if the action was handled, None to use standard handling
    fn handle_feature_action(&mut self, _action: &CanvasAction, _context: &ActionContext) -> Option<String> {
        None // Default: no custom handling
    }

    /// Get display value for a field (may differ from actual value)
    /// Used for things like password masking or computed display values
    fn get_display_value_for_field(&self, index: usize) -> &str {
        self.inputs()
            .get(index)
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    /// Check if a field has a custom display value
    /// Return true if get_display_value_for_field returns something different than the actual value
    fn has_display_override(&self, _index: usize) -> bool {
        false
    }
}
