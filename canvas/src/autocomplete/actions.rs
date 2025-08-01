// src/autocomplete/actions.rs
//! Legacy autocomplete actions - deprecated in favor of FormEditor

use crate::canvas::actions::types::{CanvasAction, ActionResult};
use anyhow::Result;

/// Legacy function - use FormEditor.trigger_autocomplete() instead
/// 
/// # Migration Guide
/// 
/// **Old way:**
/// ```rust,ignore
/// execute_with_autocomplete(action, &mut state).await?;
/// ```
/// 
/// **New way:**
/// ```rust,ignore
/// let mut editor = FormEditor::new(your_data_provider);
/// match action {
///     CanvasAction::TriggerAutocomplete => {
///         editor.trigger_autocomplete(&mut autocomplete_provider).await?;
///     }
///     CanvasAction::InsertChar(c) => {
///         editor.insert_char(c)?;
///     }
///     // ... etc
/// }
/// ```
#[deprecated(note = "Use FormEditor.trigger_autocomplete() and related methods instead")]
pub async fn execute_with_autocomplete<T>(
    _action: CanvasAction,
    _state: &mut T,
) -> Result<ActionResult> {
    Err(anyhow::anyhow!(
        "execute_with_autocomplete is deprecated. Use FormEditor API instead.\n\
         Migration: Replace CanvasState trait with DataProvider trait and use FormEditor."
    ))
}

/// Legacy function - use FormEditor methods instead
#[deprecated(note = "Use FormEditor methods instead")]
pub fn handle_autocomplete_feature_action<T>(
    _action: &CanvasAction,
    _state: &T,
) -> Option<String> {
    Some("handle_autocomplete_feature_action is deprecated. Use FormEditor API instead.".to_string())
}
