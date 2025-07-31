// src/canvas/actions/handlers/edit.rs

use crate::canvas::actions::types::{CanvasAction, ActionResult};
use crate::config::introspection::{ActionHandlerIntrospection, HandlerCapabilities, ActionSpec};
use crate::canvas::actions::movement::*;
use crate::canvas::state::CanvasState;
use crate::config::CanvasConfig;
use anyhow::Result;

const FOR_EDIT_MODE: bool = true; // Edit mode flag
                                  
pub struct EditHandler;

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

impl ActionHandlerIntrospection for EditHandler {
    fn introspect() -> HandlerCapabilities {
        let mut actions = Vec::new();

        // REQUIRED ACTIONS - These must be configured for edit mode to work
        actions.push(ActionSpec {
            name: "move_left".to_string(),
            description: "Move cursor one position to the left".to_string(),
            examples: vec!["Left".to_string(), "h".to_string()],
            is_required: true,
        });

        actions.push(ActionSpec {
            name: "move_right".to_string(),
            description: "Move cursor one position to the right".to_string(),
            examples: vec!["Right".to_string(), "l".to_string()],
            is_required: true,
        });

        actions.push(ActionSpec {
            name: "move_up".to_string(),
            description: "Move to previous field or line".to_string(),
            examples: vec!["Up".to_string(), "k".to_string()],
            is_required: true,
        });

        actions.push(ActionSpec {
            name: "move_down".to_string(),
            description: "Move to next field or line".to_string(),
            examples: vec!["Down".to_string(), "j".to_string()],
            is_required: true,
        });

        actions.push(ActionSpec {
            name: "delete_char_backward".to_string(),
            description: "Delete character before cursor".to_string(),
            examples: vec!["Backspace".to_string()],
            is_required: true,
        });

        actions.push(ActionSpec {
            name: "next_field".to_string(),
            description: "Move to next input field".to_string(),
            examples: vec!["Tab".to_string(), "Enter".to_string()],
            is_required: true,
        });

        actions.push(ActionSpec {
            name: "prev_field".to_string(),
            description: "Move to previous input field".to_string(),
            examples: vec!["Shift+Tab".to_string()],
            is_required: true,
        });

        // OPTIONAL ACTIONS - These enhance functionality but aren't required
        actions.push(ActionSpec {
            name: "move_word_next".to_string(),
            description: "Move cursor to start of next word".to_string(),
            examples: vec!["Ctrl+Right".to_string(), "w".to_string()],
            is_required: false,
        });

        actions.push(ActionSpec {
            name: "move_word_prev".to_string(),
            description: "Move cursor to start of previous word".to_string(),
            examples: vec!["Ctrl+Left".to_string(), "b".to_string()],
            is_required: false,
        });

        actions.push(ActionSpec {
            name: "move_word_end".to_string(),
            description: "Move cursor to end of current/next word".to_string(),
            examples: vec!["e".to_string()],
            is_required: false,
        });

        actions.push(ActionSpec {
            name: "move_word_end_prev".to_string(),
            description: "Move cursor to end of previous word".to_string(),
            examples: vec!["ge".to_string()],
            is_required: false,
        });

        actions.push(ActionSpec {
            name: "move_line_start".to_string(),
            description: "Move cursor to beginning of line".to_string(),
            examples: vec!["Home".to_string(), "0".to_string()],
            is_required: false,
        });

        actions.push(ActionSpec {
            name: "move_line_end".to_string(),
            description: "Move cursor to end of line".to_string(),
            examples: vec!["End".to_string(), "$".to_string()],
            is_required: false,
        });

        actions.push(ActionSpec {
            name: "move_first_line".to_string(),
            description: "Move to first field".to_string(),
            examples: vec!["Ctrl+Home".to_string(), "gg".to_string()],
            is_required: false,
        });

        actions.push(ActionSpec {
            name: "move_last_line".to_string(),
            description: "Move to last field".to_string(),
            examples: vec!["Ctrl+End".to_string(), "G".to_string()],
            is_required: false,
        });

        actions.push(ActionSpec {
            name: "delete_char_forward".to_string(),
            description: "Delete character after cursor".to_string(),
            examples: vec!["Delete".to_string()],
            is_required: false,
        });

        HandlerCapabilities {
            mode_name: "edit".to_string(),
            actions,
            auto_handled: vec![
                "insert_char".to_string(), // Any printable character
            ],
        }
    }

    fn validate_capabilities() -> Result<(), String> {
        // TODO: Could add runtime validation that the handler actually
        // implements all the actions it claims to support

        // For now, just validate that we have the essential actions
        let caps = Self::introspect();
        let required_count = caps.actions.iter().filter(|a| a.is_required).count();

        if required_count < 7 { // We expect at least 7 required actions
            return Err(format!(
                "Edit handler claims only {} required actions, expected at least 7",
                required_count
            ));
        }

        Ok(())
    }
}
