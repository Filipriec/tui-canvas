// src/canvas/actions/handlers/edit.rs
//! Edit mode action handler with EditorState

use crate::canvas::actions::types::{CanvasAction, ActionResult};
use crate::canvas::actions::movement::*;
use crate::canvas::state::EditorState;

/// Edit mode uses cursor-past-end behavior for text insertion
const FOR_EDIT_MODE: bool = true;

/// Handle actions in edit mode with edit-specific cursor behavior
pub(crate) fn handle_edit_action(
    action: CanvasAction,
    editor_state: &mut EditorState,
    current_text: &str,
) -> ActionResult {
    match action {
        // Note: Text insertion is handled at the FormEditor level
        // These handlers only deal with cursor movement and navigation
        
        // Cursor movement actions
        CanvasAction::MoveLeft => {
            let new_pos = move_left(editor_state.cursor_pos);
            editor_state.cursor_pos = new_pos;
            editor_state.ideal_cursor_column = new_pos;
            ActionResult::success()
        }

        CanvasAction::MoveRight => {
            let new_pos = move_right(editor_state.cursor_pos, current_text, FOR_EDIT_MODE);
            editor_state.cursor_pos = new_pos;
            editor_state.ideal_cursor_column = new_pos;
            ActionResult::success()
        }

        // Field navigation (treating single-line fields as "lines")
        CanvasAction::MoveUp => {
            if editor_state.current_field > 0 {
                editor_state.current_field -= 1;
                let new_pos = safe_cursor_position(current_text, editor_state.ideal_cursor_column, FOR_EDIT_MODE);
                editor_state.cursor_pos = new_pos;
            }
            ActionResult::success()
        }

        CanvasAction::MoveDown => {
            // Note: field count validation happens at FormEditor level
            editor_state.current_field += 1;
            let new_pos = safe_cursor_position(current_text, editor_state.ideal_cursor_column, FOR_EDIT_MODE);
            editor_state.cursor_pos = new_pos;
            ActionResult::success()
        }

        // Line-based movement
        CanvasAction::MoveLineStart => {
            let new_pos = line_start_position();
            editor_state.cursor_pos = new_pos;
            editor_state.ideal_cursor_column = new_pos;
            ActionResult::success()
        }

        CanvasAction::MoveLineEnd => {
            let new_pos = line_end_position(current_text, FOR_EDIT_MODE);
            editor_state.cursor_pos = new_pos;
            editor_state.ideal_cursor_column = new_pos;
            ActionResult::success()
        }

        // Document-level movement (first/last field)
        CanvasAction::MoveFirstLine => {
            editor_state.current_field = 0;
            let new_pos = safe_cursor_position(current_text, 0, FOR_EDIT_MODE);
            editor_state.cursor_pos = new_pos;
            editor_state.ideal_cursor_column = new_pos;
            ActionResult::success()
        }

        CanvasAction::MoveLastLine => {
            // Note: field count validation happens at FormEditor level
            let new_pos = line_end_position(current_text, FOR_EDIT_MODE);
            editor_state.cursor_pos = new_pos;
            editor_state.ideal_cursor_column = new_pos;
            ActionResult::success()
        }

        // Word-based movement
        CanvasAction::MoveWordNext => {
            if !current_text.is_empty() {
                let new_pos = find_next_word_start(current_text, editor_state.cursor_pos);
                editor_state.cursor_pos = new_pos;
                editor_state.ideal_cursor_column = new_pos;
            }
            ActionResult::success()
        }

        CanvasAction::MoveWordEnd => {
            if !current_text.is_empty() {
                let new_pos = find_word_end(current_text, editor_state.cursor_pos);
                editor_state.cursor_pos = new_pos;
                editor_state.ideal_cursor_column = new_pos;
            }
            ActionResult::success()
        }

        CanvasAction::MoveWordPrev => {
            if !current_text.is_empty() {
                let new_pos = find_prev_word_start(current_text, editor_state.cursor_pos);
                editor_state.cursor_pos = new_pos;
                editor_state.ideal_cursor_column = new_pos;
            }
            ActionResult::success()
        }

        CanvasAction::MoveWordEndPrev => {
            if !current_text.is_empty() {
                let new_pos = find_prev_word_end(current_text, editor_state.cursor_pos);
                editor_state.cursor_pos = new_pos;
                editor_state.ideal_cursor_column = new_pos;
            }
            ActionResult::success()
        }

        // Field navigation - handled at FormEditor level for bounds checking
        CanvasAction::NextField | CanvasAction::PrevField => {
            ActionResult::success_with_message("Field navigation handled by FormEditor")
        }

        // Text editing actions - handled at FormEditor level
        CanvasAction::InsertChar(_) |
        CanvasAction::DeleteBackward |
        CanvasAction::DeleteForward => {
            ActionResult::success_with_message("Text editing handled by FormEditor")
        }

        CanvasAction::Custom(action_str) => {
            ActionResult::success_with_message(&format!("Custom edit action: {}", action_str))
        }

        _ => {
            ActionResult::success_with_message("Action not implemented for edit mode")
        }
    }
}
