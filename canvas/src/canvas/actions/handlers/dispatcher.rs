// src/canvas/actions/handlers/dispatcher.rs

use crate::canvas::state::{CanvasState, ActionContext};
use crate::canvas::actions::{CanvasAction, ActionResult};
use crate::canvas::modes::AppMode;
use anyhow::Result;

use super::{handle_edit_action, handle_readonly_action, handle_highlight_action};

/// Main action dispatcher - routes actions to mode-specific handlers
pub async fn dispatch_action<S: CanvasState>(
    action: CanvasAction,
    state: &mut S,
    ideal_cursor_column: &mut usize,
) -> Result<ActionResult> {
    // Check if the application wants to handle this action first
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
            handle_edit_action(action, state, ideal_cursor_column).await
        }
        AppMode::ReadOnly => {
            handle_readonly_action(action, state, ideal_cursor_column).await
        }
        AppMode::Highlight => {
            handle_highlight_action(action, state, ideal_cursor_column).await
        }
        AppMode::General | AppMode::Command => {
            Ok(ActionResult::success_with_message("Mode does not handle canvas actions directly"))
        }
    }
}
