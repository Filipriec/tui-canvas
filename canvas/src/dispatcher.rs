// canvas/src/dispatcher.rs

use crate::canvas::state::CanvasState;
use crate::canvas::actions::{CanvasAction, ActionResult, execute_canvas_action};
use crate::config::CanvasConfig;
use crossterm::event::{KeyCode, KeyModifiers};

/// High-level action dispatcher that coordinates between different action types
pub struct ActionDispatcher;

impl ActionDispatcher {
    /// Dispatch any action to the appropriate handler
    pub async fn dispatch<S: CanvasState>(
        action: CanvasAction,
        state: &mut S,
        ideal_cursor_column: &mut usize,
    ) -> anyhow::Result<ActionResult> {
        // Load config once here instead of threading it everywhere
        execute_canvas_action(action, state, ideal_cursor_column, Some(&CanvasConfig::load())).await
    }

    /// Quick action dispatch from KeyCode using config
    pub async fn dispatch_key<S: CanvasState>(
        key: KeyCode,
        modifiers: KeyModifiers,
        state: &mut S,
        ideal_cursor_column: &mut usize,
        is_edit_mode: bool,
        has_suggestions: bool,
    ) -> anyhow::Result<Option<ActionResult>> {
        let config = CanvasConfig::load();
        
        if let Some(action_name) = config.get_action_for_key(key, modifiers, is_edit_mode, has_suggestions) {
            let action = CanvasAction::from_string(action_name);
            let result = Self::dispatch(action, state, ideal_cursor_column).await?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    /// Batch dispatch multiple actions
    pub async fn dispatch_batch<S: CanvasState>(
        actions: Vec<CanvasAction>,
        state: &mut S,
        ideal_cursor_column: &mut usize,
    ) -> anyhow::Result<Vec<ActionResult>> {
        let mut results = Vec::new();
        for action in actions {
            let result = Self::dispatch(action, state, ideal_cursor_column).await?;
            let is_success = result.is_success(); // Check success before moving
            results.push(result);

            // Stop on first error
            if !is_success {
                break;
            }
        }
        Ok(results)
    }
}
