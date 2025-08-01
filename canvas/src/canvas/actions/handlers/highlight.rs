// src/canvas/actions/handlers/highlight.rs
//! Highlight mode action handler with EditorState

use crate::canvas::actions::types::{CanvasAction, ActionResult};
use crate::canvas::actions::movement::*;
use crate::canvas::state::EditorState;

const FOR_EDIT_MODE: bool = false; // Highlight mode uses read-only cursor behavior

/// Handle actions in highlight/visual mode
pub(crate) fn handle_highlight_action(
    action: CanvasAction,
    editor_state: &mut EditorState,
    current_text: &str,
) -> ActionResult {
    match action {
        // Movement actions work similar to read-only mode but with selection
        CanvasAction::MoveLeft => {
            let new_pos = move_left(editor_state.cursor_pos);
            editor_state.cursor_pos = new_pos;
            editor_state.ideal_cursor_column = new_pos;
            // TODO: Update selection range
            ActionResult::success()
        }

        CanvasAction::MoveRight => {
            let new_pos = move_right(editor_state.cursor_pos, current_text, FOR_EDIT_MODE);
            editor_state.cursor_pos = new_pos;
            editor_state.ideal_cursor_column = new_pos;
            // TODO: Update selection range
            ActionResult::success()
        }

        CanvasAction::MoveWordNext => {
            if !current_text.is_empty() {
                let new_pos = find_next_word_start(current_text, editor_state.cursor_pos);
                let final_pos = clamp_cursor_position(new_pos, current_text, FOR_EDIT_MODE);
                editor_state.cursor_pos = final_pos;
                editor_state.ideal_cursor_column = final_pos;
                // TODO: Update selection range
            }
            ActionResult::success()
        }

        CanvasAction::MoveWordEnd => {
            if !current_text.is_empty() {
                let new_pos = find_word_end(current_text, editor_state.cursor_pos);
                let final_pos = clamp_cursor_position(new_pos, current_text, FOR_EDIT_MODE);
                editor_state.cursor_pos = final_pos;
                editor_state.ideal_cursor_column = final_pos;
                // TODO: Update selection range
            }
            ActionResult::success()
        }

        CanvasAction::MoveWordPrev => {
            if !current_text.is_empty() {
                let new_pos = find_prev_word_start(current_text, editor_state.cursor_pos);
                editor_state.cursor_pos = new_pos;
                editor_state.ideal_cursor_column = new_pos;
                // TODO: Update selection range
            }
            ActionResult::success()
        }

        CanvasAction::MoveLineStart => {
            let new_pos = line_start_position();
            editor_state.cursor_pos = new_pos;
            editor_state.ideal_cursor_column = new_pos;
            // TODO: Update selection range
            ActionResult::success()
        }

        CanvasAction::MoveLineEnd => {
            let new_pos = line_end_position(current_text, FOR_EDIT_MODE);
            editor_state.cursor_pos = new_pos;
            editor_state.ideal_cursor_column = new_pos;
            // TODO: Update selection range
            ActionResult::success()
        }

        // Highlight mode doesn't handle editing actions
        CanvasAction::InsertChar(_) |
        CanvasAction::DeleteBackward |
        CanvasAction::DeleteForward => {
            ActionResult::success_with_message("Action not available in highlight mode")
        }

        CanvasAction::Custom(action_str) => {
            ActionResult::success_with_message(&format!("Custom highlight action: {}", action_str))
        }

        _ => {
            ActionResult::success_with_message("Action not implemented for highlight mode")
        }
    }
}
