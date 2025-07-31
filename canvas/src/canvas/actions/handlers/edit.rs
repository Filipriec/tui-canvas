// src/canvas/actions/handlers/edit.rs
//! Edit mode action handler
//!
//! Handles user input when in edit mode, supporting text entry, deletion,
//! and cursor movement with edit-specific behavior (cursor can go past end of text).

use crate::canvas::actions::types::{CanvasAction, ActionResult};
use crate::canvas::actions::movement::*;
use crate::canvas::state::CanvasState;
use anyhow::Result;

/// Edit mode uses cursor-past-end behavior for text insertion
const FOR_EDIT_MODE: bool = true;

/// Handle actions in edit mode with edit-specific cursor behavior
///
/// Edit mode allows text modification and uses cursor positioning that can
/// go past the end of existing text to facilitate insertion.
///
/// # Arguments
/// * `action` - The action to perform
/// * `state` - Mutable canvas state
/// * `ideal_cursor_column` - Desired column for vertical movement (maintained across line changes)
pub async fn handle_edit_action<S: CanvasState>(
    action: CanvasAction,
    state: &mut S,
    ideal_cursor_column: &mut usize,
) -> Result<ActionResult> {
    match action {
        CanvasAction::InsertChar(c) => {
            // Insert character at cursor position and advance cursor
            let cursor_pos = state.current_cursor_pos();
            let input = state.get_current_input_mut();
            input.insert(cursor_pos, c);
            state.set_current_cursor_pos(cursor_pos + 1);
            state.set_has_unsaved_changes(true);
            *ideal_cursor_column = cursor_pos + 1;
            Ok(ActionResult::success())
        }

        CanvasAction::DeleteBackward => {
            // Delete character before cursor (Backspace behavior)
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
            // Delete character at cursor position (Delete key behavior)
            let cursor_pos = state.current_cursor_pos();
            let input = state.get_current_input_mut();
            if cursor_pos < input.len() {
                input.remove(cursor_pos);
                state.set_has_unsaved_changes(true);
            }
            Ok(ActionResult::success())
        }

        // Cursor movement actions
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

        // Field navigation (treating single-line fields as "lines")
        CanvasAction::MoveUp => {
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

        // Line-based movement
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

        // Document-level movement (first/last field)
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

        // Word-based movement
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

        // Field navigation with simple wrapping behavior
        CanvasAction::NextField | CanvasAction::PrevField => {
            let current_field = state.current_field();
            let total_fields = state.fields().len();

            let new_field = match action {
                CanvasAction::NextField => {
                    (current_field + 1) % total_fields // Simple wrap
                }
                CanvasAction::PrevField => {
                    if current_field == 0 { total_fields - 1 } else { current_field - 1 } // Simple wrap
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
