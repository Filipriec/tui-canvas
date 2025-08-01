// src/canvas/actions/handlers/dispatcher.rs

use crate::canvas::state::EditorState;
use crate::canvas::actions::{CanvasAction, ActionResult};
use crate::canvas::modes::AppMode;

use super::{handle_edit_action, handle_readonly_action, handle_highlight_action};

/// Internal action dispatcher - routes actions to mode-specific handlers
pub(crate) fn dispatch_action_internal(
    action: CanvasAction,
    editor_state: &mut EditorState,
    current_text: &str,
) -> ActionResult {
    // Route to mode-specific handler based on current mode
    match editor_state.current_mode {
        AppMode::Edit => {
            handle_edit_action(action, editor_state, current_text)
        }
        AppMode::ReadOnly => {
            handle_readonly_action(action, editor_state, current_text)
        }
        AppMode::Highlight => {
            handle_highlight_action(action, editor_state, current_text)
        }
        AppMode::General | AppMode::Command => {
            ActionResult::success_with_message("Mode does not handle canvas actions directly")
        }
    }
}
