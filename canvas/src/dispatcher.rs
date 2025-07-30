// src/dispatcher.rs

use crate::canvas::state::{CanvasState, ActionContext};
use crate::canvas::actions::{CanvasAction, ActionResult};
use crate::canvas::actions::handlers::{handle_edit_action, handle_readonly_action, handle_highlight_action};
use crate::canvas::modes::AppMode;
use crate::config::CanvasConfig;
use crossterm::event::{KeyCode, KeyModifiers};

/// Main entry point for executing canvas actions
pub async fn execute_canvas_action<S: CanvasState>(
    action: CanvasAction,
    state: &mut S,
    ideal_cursor_column: &mut usize,
    config: Option<&CanvasConfig>,
) -> anyhow::Result<ActionResult> {
    ActionDispatcher::dispatch_with_config(action, state, ideal_cursor_column, config).await
}

/// High-level action dispatcher that routes actions to mode-specific handlers
pub struct ActionDispatcher;

impl ActionDispatcher {
    /// Dispatch any action to the appropriate mode handler
    pub async fn dispatch<S: CanvasState>(
        action: CanvasAction,
        state: &mut S,
        ideal_cursor_column: &mut usize,
    ) -> anyhow::Result<ActionResult> {
        let config = CanvasConfig::load();
        Self::dispatch_with_config(action, state, ideal_cursor_column, Some(&config)).await
    }

    /// Dispatch action with provided config
    pub async fn dispatch_with_config<S: CanvasState>(
        action: CanvasAction,
        state: &mut S,
        ideal_cursor_column: &mut usize,
        config: Option<&CanvasConfig>,
    ) -> anyhow::Result<ActionResult> {
        // Check for feature-specific handling first
        let context = ActionContext {
            key_code: None,
            ideal_cursor_column: *ideal_cursor_column,
            current_input: state.get_current_input().to_string(),
            current_field: state.current_field(),
        };

        if let Some(result) = state.handle_feature_action(&action, &context) {
            return Ok(ActionResult::HandledByFeature(result));
        }

        // Route to mode-specific handler
        match state.current_mode() {
            AppMode::Edit => {
                handle_edit_action(action, state, ideal_cursor_column, config).await
            }
            AppMode::ReadOnly => {
                handle_readonly_action(action, state, ideal_cursor_column, config).await
            }
            AppMode::Highlight => {
                handle_highlight_action(action, state, ideal_cursor_column, config).await
            }
            AppMode::General | AppMode::Command => {
                // These modes might not handle canvas actions directly
                Ok(ActionResult::success_with_message("Mode does not handle canvas actions"))
            }
        }
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
            let result = Self::dispatch_with_config(action, state, ideal_cursor_column, Some(&config)).await?;
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
            let is_success = result.is_success();
            results.push(result);

            // Stop on first error
            if !is_success {
                break;
            }
        }
        Ok(results)
    }
}
