// src/canvas/actions/handlers/edit.rs

use crate::canvas::actions::types::{CanvasAction, ActionResult};
use crate::canvas::actions::movement::*;
use crate::canvas::state::CanvasState;
use crate::config::CanvasConfig;
use anyhow::Result;

const FOR_EDIT_MODE: bool = true; // Edit mode flag

/// Handle actions in edit mode with edit-specific cursor behavior
pub async fn handle_edit_action<S: CanvasState>(
    action: CanvasAction,
    state: &mut S,
    ideal_cursor_column: &mut usize,
    config: Option<&CanvasConfig>,
) -> Result<ActionResult> {
    match action {
        CanvasAction::InsertChar(c) => {
            let cursor_pos = state.current_cursor_pos();
            let input = state.get_current_input_mut();
            input.insert(cursor_pos, c);
            state.set_current_cursor_pos(cursor_pos + 1);
            state.set_has_unsaved_changes(true);
            *ideal_cursor_column = cursor_pos + 1;
            Ok(ActionResult::success())
        }

        CanvasAction::DeleteBackward => {
            let cursor_pos = state.current_cursor_pos();
            if cursor_pos > 0 {
                let input = state.get_current_input_mut();
                input.remove(cursor_pos - 1);
                state.set_current_cursor_pos(cursor_pos - 1);
                state.set_has_unsaved_changes(true);
                *ideal_cursor_column = cursor_pos - 1;
            }
            Ok(ActionResult::success())
        }

        CanvasAction::DeleteForward => {
            let cursor_pos = state.current_cursor_pos();
            let input = state.get_current_input_mut();
            if cursor_pos < input.len() {
                input.remove(cursor_pos);
                state.set_has_unsaved_changes(true);
            }
            Ok(ActionResult::success())
        }

        CanvasAction::MoveLeft => {
            let new_pos = move_left(state.current_cursor_pos());
            state.set_current_cursor_pos(new_pos);
            *ideal_cursor_column = new_pos;
            Ok(ActionResult::success())
        }

        CanvasAction::MoveRight => {
            let current_input = state.get_current_input();
            let current_pos = state.current_cursor_pos();
            let new_pos = move_right(current_pos, current_input, FOR_EDIT_MODE);
            state.set_current_cursor_pos(new_pos);
            *ideal_cursor_column = new_pos;
            Ok(ActionResult::success())
        }

        CanvasAction::MoveUp => {
            // For single-line fields, move to previous field
            let current_field = state.current_field();
            if current_field > 0 {
                state.set_current_field(current_field - 1);
                let current_input = state.get_current_input();
                let new_pos = safe_cursor_position(current_input, *ideal_cursor_column, FOR_EDIT_MODE);
                state.set_current_cursor_pos(new_pos);
            }
            Ok(ActionResult::success())
        }

        CanvasAction::MoveDown => {
            // For single-line fields, move to next field
            let current_field = state.current_field();
            let total_fields = state.fields().len();
            if current_field < total_fields - 1 {
                state.set_current_field(current_field + 1);
                let current_input = state.get_current_input();
                let new_pos = safe_cursor_position(current_input, *ideal_cursor_column, FOR_EDIT_MODE);
                state.set_current_cursor_pos(new_pos);
            }
            Ok(ActionResult::success())
        }

        CanvasAction::MoveLineStart => {
            let new_pos = line_start_position();
            state.set_current_cursor_pos(new_pos);
            *ideal_cursor_column = new_pos;
            Ok(ActionResult::success())
        }

        CanvasAction::MoveLineEnd => {
            let current_input = state.get_current_input();
            let new_pos = line_end_position(current_input, FOR_EDIT_MODE);
            state.set_current_cursor_pos(new_pos);
            *ideal_cursor_column = new_pos;
            Ok(ActionResult::success())
        }

        CanvasAction::MoveFirstLine => {
            state.set_current_field(0);
            let current_input = state.get_current_input();
            let new_pos = safe_cursor_position(current_input, 0, FOR_EDIT_MODE);
            state.set_current_cursor_pos(new_pos);
            *ideal_cursor_column = new_pos;
            Ok(ActionResult::success())
        }

        CanvasAction::MoveLastLine => {
            let last_field = state.fields().len() - 1;
            state.set_current_field(last_field);
            let current_input = state.get_current_input();
            let new_pos = line_end_position(current_input, FOR_EDIT_MODE);
            state.set_current_cursor_pos(new_pos);
            *ideal_cursor_column = new_pos;
            Ok(ActionResult::success())
        }

        CanvasAction::MoveWordNext => {
            let current_input = state.get_current_input();
            if !current_input.is_empty() {
                let new_pos = find_next_word_start(current_input, state.current_cursor_pos());
                state.set_current_cursor_pos(new_pos);
                *ideal_cursor_column = new_pos;
            }
            Ok(ActionResult::success())
        }

        CanvasAction::MoveWordEnd => {
            let current_input = state.get_current_input();
            if !current_input.is_empty() {
                let new_pos = find_word_end(current_input, state.current_cursor_pos());
                state.set_current_cursor_pos(new_pos);
                *ideal_cursor_column = new_pos;
            }
            Ok(ActionResult::success())
        }

        CanvasAction::MoveWordPrev => {
            let current_input = state.get_current_input();
            if !current_input.is_empty() {
                let new_pos = find_prev_word_start(current_input, state.current_cursor_pos());
                state.set_current_cursor_pos(new_pos);
                *ideal_cursor_column = new_pos;
            }
            Ok(ActionResult::success())
        }

        CanvasAction::MoveWordEndPrev => {
            let current_input = state.get_current_input();
            if !current_input.is_empty() {
                let new_pos = find_prev_word_end(current_input, state.current_cursor_pos());
                state.set_current_cursor_pos(new_pos);
                *ideal_cursor_column = new_pos;
            }
            Ok(ActionResult::success())
        }

        CanvasAction::NextField | CanvasAction::PrevField => {
            let current_field = state.current_field();
            let total_fields = state.fields().len();

            let new_field = match action {
                CanvasAction::NextField => {
                    if config.map_or(true, |c| c.behavior.wrap_around_fields) {
                        (current_field + 1) % total_fields
                    } else {
                        (current_field + 1).min(total_fields - 1)
                    }
                }
                CanvasAction::PrevField => {
                    if config.map_or(true, |c| c.behavior.wrap_around_fields) {
                        if current_field == 0 { total_fields - 1 } else { current_field - 1 }
                    } else {
                        current_field.saturating_sub(1)
                    }
                }
                _ => unreachable!(),
            };

            state.set_current_field(new_field);
            let current_input = state.get_current_input();
            let new_pos = safe_cursor_position(current_input, *ideal_cursor_column, FOR_EDIT_MODE);
            state.set_current_cursor_pos(new_pos);
            Ok(ActionResult::success())
        }

        CanvasAction::Custom(action_str) => {
            Ok(ActionResult::success_with_message(&format!("Custom edit action: {}", action_str)))
        }

        _ => {
            Ok(ActionResult::success_with_message("Action not implemented for edit mode"))
        }
    }
}
