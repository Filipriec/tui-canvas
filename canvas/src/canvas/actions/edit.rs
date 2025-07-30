// src/canvas/actions/edit.rs
// COMPATIBILITY LAYER - maintains old API while using new handler system

use crate::canvas::state::{CanvasState, ActionContext};
use crate::canvas::actions::types::{CanvasAction, ActionResult};
use crate::config::CanvasConfig;
use anyhow::Result;

/// BACKWARD COMPATIBILITY: Execute a typed canvas action on any CanvasState implementation
/// This maintains the old API while routing to the new mode-aware system
pub async fn execute_canvas_action<S: CanvasState>(
    action: CanvasAction,
    state: &mut S,
    ideal_cursor_column: &mut usize,
    config: Option<&CanvasConfig>,
) -> Result<ActionResult> {
    // Route to new dispatcher system
    crate::dispatcher::ActionDispatcher::dispatch_with_config(
        action,
        state,
        ideal_cursor_column,
        config,
    ).await
}

/// BACKWARD COMPATIBILITY: Handle core canvas actions with full type safety
/// This function is kept for backward compatibility with autocomplete and other modules
pub async fn handle_generic_canvas_action<S: CanvasState>(
    action: CanvasAction,
    state: &mut S,
    ideal_cursor_column: &mut usize,
    config: Option<&CanvasConfig>,
) -> Result<ActionResult> {
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

    // Route to appropriate mode handler based on current mode
    match state.current_mode() {
        crate::canvas::modes::AppMode::Edit => {
            crate::canvas::actions::handlers::handle_edit_action(action, state, ideal_cursor_column, config).await
        }
        crate::canvas::modes::AppMode::ReadOnly => {
            crate::canvas::actions::handlers::handle_readonly_action(action, state, ideal_cursor_column, config).await
        }
        crate::canvas::modes::AppMode::Highlight => {
            crate::canvas::actions::handlers::handle_highlight_action(action, state, ideal_cursor_column, config).await
        }
        crate::canvas::modes::AppMode::General | crate::canvas::modes::AppMode::Command => {
            // These modes might not handle canvas actions directly
            Ok(ActionResult::success_with_message("Mode does not handle canvas actions"))
        }
    }
}
