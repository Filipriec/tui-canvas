// src/canvas/actions/handlers/highlight.rs

use crate::canvas::actions::types::{CanvasAction, ActionResult};
use crate::canvas::actions::movement::*;
use crate::canvas::state::CanvasState;
use crate::config::CanvasConfig;
use anyhow::Result;

const FOR_EDIT_MODE: bool = false; // Highlight mode uses read-only cursor behavior

/// Handle actions in highlight/visual mode
/// TODO: Implement selection logic and highlight-specific behaviors
pub async fn handle_highlight_action<S: CanvasState>(
    action: CanvasAction,
    state: &mut S,
    ideal_cursor_column: &mut usize,
    config: Option<&CanvasConfig>,
) -> Result<ActionResult> {
    match action {
        // Movement actions work similar to read-only mode but with selection
        CanvasAction::MoveLeft => {
            let new_pos = move_left(state.current_cursor_pos());
            state.set_current_cursor_pos(new_pos);
            *ideal_cursor_column = new_pos;
            // TODO: Update selection range
            Ok(ActionResult::success())
        }

        CanvasAction::MoveRight => {
            let current_input = state.get_current_input();
            let current_pos = state.current_cursor_pos();
            let new_pos = move_right(current_pos, current_input, FOR_EDIT_MODE);
            state.set_current_cursor_pos(new_pos);
            *ideal_cursor_column = new_pos;
            // TODO: Update selection range
            Ok(ActionResult::success())
        }

        CanvasAction::MoveWordNext => {
            let current_input = state.get_current_input();
            if !current_input.is_empty() {
                let new_pos = find_next_word_start(current_input, state.current_cursor_pos());
                let final_pos = clamp_cursor_position(new_pos, current_input, FOR_EDIT_MODE);
                state.set_current_cursor_pos(final_pos);
                *ideal_cursor_column = final_pos;
                // TODO: Update selection range
            }
            Ok(ActionResult::success())
        }

        CanvasAction::MoveWordEnd => {
            let current_input = state.get_current_input();
            if !current_input.is_empty() {
                let new_pos = find_word_end(current_input, state.current_cursor_pos());
                let final_pos = clamp_cursor_position(new_pos, current_input, FOR_EDIT_MODE);
                state.set_current_cursor_pos(final_pos);
                *ideal_cursor_column = final_pos;
                // TODO: Update selection range
            }
            Ok(ActionResult::success())
        }

        CanvasAction::MoveWordPrev => {
            let current_input = state.get_current_input();
            if !current_input.is_empty() {
                let new_pos = find_prev_word_start(current_input, state.current_cursor_pos());
                state.set_current_cursor_pos(new_pos);
                *ideal_cursor_column = new_pos;
                // TODO: Update selection range
            }
            Ok(ActionResult::success())
        }

        CanvasAction::MoveLineStart => {
            let new_pos = line_start_position();
            state.set_current_cursor_pos(new_pos);
            *ideal_cursor_column = new_pos;
            // TODO: Update selection range
            Ok(ActionResult::success())
        }

        CanvasAction::MoveLineEnd => {
            let current_input = state.get_current_input();
            let new_pos = line_end_position(current_input, FOR_EDIT_MODE);
            state.set_current_cursor_pos(new_pos);
            *ideal_cursor_column = new_pos;
            // TODO: Update selection range
            Ok(ActionResult::success())
        }

        // Highlight mode doesn't handle editing actions
        CanvasAction::InsertChar(_) | 
        CanvasAction::DeleteBackward | 
        CanvasAction::DeleteForward => {
            Ok(ActionResult::success_with_message("Action not available in highlight mode"))
        }

        CanvasAction::Custom(action_str) => {
            Ok(ActionResult::success_with_message(&format!("Custom highlight action: {}", action_str)))
        }

        _ => {
            Ok(ActionResult::success_with_message("Action not implemented for highlight mode"))
        }
    }
}
