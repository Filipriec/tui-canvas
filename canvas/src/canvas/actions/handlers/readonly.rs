// src/canvas/actions/handlers/readonly.rs

use crate::canvas::actions::types::{CanvasAction, ActionResult};
use crate::config::introspection::{ActionHandlerIntrospection, HandlerCapabilities, ActionSpec};
use crate::canvas::actions::movement::*;
use crate::canvas::state::CanvasState;
use crate::config::CanvasConfig;
use anyhow::Result;

const FOR_EDIT_MODE: bool = false; // Read-only mode flag

/// Handle actions in read-only mode with read-only specific cursor behavior
pub async fn handle_readonly_action<S: CanvasState>(
    action: CanvasAction,
    state: &mut S,
    ideal_cursor_column: &mut usize,
    config: Option<&CanvasConfig>,
) -> Result<ActionResult> {
    match action {
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
            let current_field = state.current_field();
            let new_field = current_field.saturating_sub(1);
            state.set_current_field(new_field);
            
            // Apply ideal cursor column with read-only bounds
            let current_input = state.get_current_input();
            let new_pos = safe_cursor_position(current_input, *ideal_cursor_column, FOR_EDIT_MODE);
            state.set_current_cursor_pos(new_pos);
            Ok(ActionResult::success())
        }

        CanvasAction::MoveDown => {
            let current_field = state.current_field();
            let total_fields = state.fields().len();
            if total_fields == 0 {
                return Ok(ActionResult::success_with_message("No fields to navigate"));
            }
            
            let new_field = (current_field + 1).min(total_fields - 1);
            state.set_current_field(new_field);
            
            // Apply ideal cursor column with read-only bounds
            let current_input = state.get_current_input();
            let new_pos = safe_cursor_position(current_input, *ideal_cursor_column, FOR_EDIT_MODE);
            state.set_current_cursor_pos(new_pos);
            Ok(ActionResult::success())
        }

        CanvasAction::MoveFirstLine => {
            let total_fields = state.fields().len();
            if total_fields == 0 {
                return Ok(ActionResult::success_with_message("No fields to navigate"));
            }
            
            state.set_current_field(0);
            let current_input = state.get_current_input();
            let new_pos = safe_cursor_position(current_input, *ideal_cursor_column, FOR_EDIT_MODE);
            state.set_current_cursor_pos(new_pos);
            *ideal_cursor_column = new_pos;
            Ok(ActionResult::success())
        }

        CanvasAction::MoveLastLine => {
            let total_fields = state.fields().len();
            if total_fields == 0 {
                return Ok(ActionResult::success_with_message("No fields to navigate"));
            }
            
            let last_field = total_fields - 1;
            state.set_current_field(last_field);
            let current_input = state.get_current_input();
            let new_pos = safe_cursor_position(current_input, *ideal_cursor_column, FOR_EDIT_MODE);
            state.set_current_cursor_pos(new_pos);
            *ideal_cursor_column = new_pos;
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

        CanvasAction::MoveWordNext => {
            let current_input = state.get_current_input();
            if !current_input.is_empty() {
                let new_pos = find_next_word_start(current_input, state.current_cursor_pos());
                let final_pos = clamp_cursor_position(new_pos, current_input, FOR_EDIT_MODE);
                state.set_current_cursor_pos(final_pos);
                *ideal_cursor_column = final_pos;
            }
            Ok(ActionResult::success())
        }

        CanvasAction::MoveWordEnd => {
            let current_input = state.get_current_input();
            if !current_input.is_empty() {
                let current_pos = state.current_cursor_pos();
                let new_pos = find_word_end(current_input, current_pos);
                let final_pos = clamp_cursor_position(new_pos, current_input, FOR_EDIT_MODE);
                state.set_current_cursor_pos(final_pos);
                *ideal_cursor_column = final_pos;
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
            *ideal_cursor_column = state.current_cursor_pos();
            Ok(ActionResult::success())
        }

        // Read-only mode doesn't handle editing actions
        CanvasAction::InsertChar(_) | 
        CanvasAction::DeleteBackward | 
        CanvasAction::DeleteForward => {
            Ok(ActionResult::success_with_message("Action not available in read-only mode"))
        }

        CanvasAction::Custom(action_str) => {
            Ok(ActionResult::success_with_message(&format!("Custom readonly action: {}", action_str)))
        }

        _ => {
            Ok(ActionResult::success_with_message("Action not implemented for read-only mode"))
        }
    }
}

pub struct ReadOnlyHandler;

impl ActionHandlerIntrospection for ReadOnlyHandler {
    fn introspect() -> HandlerCapabilities {
        let mut actions = Vec::new();

        // REQUIRED ACTIONS - Navigation is essential in read-only mode
        actions.push(ActionSpec {
            name: "move_left".to_string(),
            description: "Move cursor one position to the left".to_string(),
            examples: vec!["h".to_string(), "Left".to_string()],
            is_required: true,
        });

        actions.push(ActionSpec {
            name: "move_right".to_string(),
            description: "Move cursor one position to the right".to_string(),
            examples: vec!["l".to_string(), "Right".to_string()],
            is_required: true,
        });

        actions.push(ActionSpec {
            name: "move_up".to_string(),
            description: "Move to previous field".to_string(),
            examples: vec!["k".to_string(), "Up".to_string()],
            is_required: true,
        });

        actions.push(ActionSpec {
            name: "move_down".to_string(),
            description: "Move to next field".to_string(),
            examples: vec!["j".to_string(), "Down".to_string()],
            is_required: true,
        });

        // OPTIONAL ACTIONS - Advanced navigation features
        actions.push(ActionSpec {
            name: "move_word_next".to_string(),
            description: "Move cursor to start of next word".to_string(),
            examples: vec!["w".to_string()],
            is_required: false,
        });

        actions.push(ActionSpec {
            name: "move_word_prev".to_string(),
            description: "Move cursor to start of previous word".to_string(),
            examples: vec!["b".to_string()],
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
            examples: vec!["0".to_string()],
            is_required: false,
        });

        actions.push(ActionSpec {
            name: "move_line_end".to_string(),
            description: "Move cursor to end of line".to_string(),
            examples: vec!["$".to_string()],
            is_required: false,
        });

        actions.push(ActionSpec {
            name: "move_first_line".to_string(),
            description: "Move to first field".to_string(),
            examples: vec!["gg".to_string()],
            is_required: false,
        });

        actions.push(ActionSpec {
            name: "move_last_line".to_string(),
            description: "Move to last field".to_string(),
            examples: vec!["G".to_string()],
            is_required: false,
        });

        actions.push(ActionSpec {
            name: "next_field".to_string(),
            description: "Move to next input field".to_string(),
            examples: vec!["Tab".to_string()],
            is_required: false,
        });

        actions.push(ActionSpec {
            name: "prev_field".to_string(),
            description: "Move to previous input field".to_string(),
            examples: vec!["Shift+Tab".to_string()],
            is_required: false,
        });

        HandlerCapabilities {
            mode_name: "read_only".to_string(),
            actions,
            auto_handled: vec![], // Read-only mode has no auto-handled actions
        }
    }

    fn validate_capabilities() -> Result<(), String> {
        let caps = Self::introspect();
        let required_count = caps.actions.iter().filter(|a| a.is_required).count();

        if required_count < 4 { // We expect at least 4 required actions (basic movement)
            return Err(format!(
                "ReadOnly handler claims only {} required actions, expected at least 4",
                required_count
            ));
        }

        Ok(())
    }
}
