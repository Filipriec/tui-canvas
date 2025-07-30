// canvas/src/canvas/actions/edit.rs

use crate::canvas::state::{CanvasState, ActionContext};
use crate::canvas::actions::types::{CanvasAction, ActionResult};
use crate::config::CanvasConfig;
use anyhow::Result;

/// Execute a typed canvas action on any CanvasState implementation
pub async fn execute_canvas_action<S: CanvasState>(
    action: CanvasAction,
    state: &mut S,
    ideal_cursor_column: &mut usize,
    config: Option<&CanvasConfig>,
) -> Result<ActionResult> {
    let context = ActionContext {
        key_code: None,
        ideal_cursor_column: *ideal_cursor_column,
        current_input: state.get_current_input().to_string(),
        current_field: state.current_field(),
    };

    if let Some(result) = state.handle_feature_action(&action, &context) {
        return Ok(ActionResult::HandledByFeature(result));
    }

    handle_generic_canvas_action(action, state, ideal_cursor_column, config).await
}

/// Handle core canvas actions with full type safety
pub async fn handle_generic_canvas_action<S: CanvasState>(
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

        CanvasAction::NextField | CanvasAction::PrevField => {
            let old_field = state.current_field();
            let total_fields = state.fields().len();

            // Perform field navigation
            let new_field = match action {
                CanvasAction::NextField => {
                    if config.map_or(true, |c| c.behavior.wrap_around_fields) {
                        (old_field + 1) % total_fields
                    } else {
                        (old_field + 1).min(total_fields - 1)
                    }
                }
                CanvasAction::PrevField => {
                    if config.map_or(true, |c| c.behavior.wrap_around_fields) {
                        if old_field == 0 { total_fields - 1 } else { old_field - 1 }
                    } else {
                        old_field.saturating_sub(1)
                    }
                }
                _ => unreachable!(),
            };

            state.set_current_field(new_field);
            *ideal_cursor_column = state.current_cursor_pos();
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
            let cursor_pos = state.current_cursor_pos();
            if cursor_pos > 0 {
                state.set_current_cursor_pos(cursor_pos - 1);
                *ideal_cursor_column = cursor_pos - 1;
            }
            Ok(ActionResult::success())
        }

        CanvasAction::MoveRight => {
            let cursor_pos = state.current_cursor_pos();
            let current_input = state.get_current_input();
            if cursor_pos < current_input.len() {
                state.set_current_cursor_pos(cursor_pos + 1);
                *ideal_cursor_column = cursor_pos + 1;
            }
            Ok(ActionResult::success())
        }

        CanvasAction::MoveLineStart => {
            state.set_current_cursor_pos(0);
            *ideal_cursor_column = 0;
            Ok(ActionResult::success())
        }

        CanvasAction::MoveLineEnd => {
            let end_pos = state.get_current_input().len();
            state.set_current_cursor_pos(end_pos);
            *ideal_cursor_column = end_pos;
            Ok(ActionResult::success())
        }

        CanvasAction::MoveUp => {
            // For single-line fields, move to previous field
            let current_field = state.current_field();
            if current_field > 0 {
                state.set_current_field(current_field - 1);
                *ideal_cursor_column = state.current_cursor_pos();
            }
            Ok(ActionResult::success())
        }

        CanvasAction::MoveDown => {
            // For single-line fields, move to next field
            let current_field = state.current_field();
            let total_fields = state.fields().len();
            if current_field < total_fields - 1 {
                state.set_current_field(current_field + 1);
                *ideal_cursor_column = state.current_cursor_pos();
            }
            Ok(ActionResult::success())
        }

        CanvasAction::MoveFirstLine => {
            state.set_current_field(0);
            state.set_current_cursor_pos(0);
            *ideal_cursor_column = 0;
            Ok(ActionResult::success())
        }

        CanvasAction::MoveLastLine => {
            let last_field = state.fields().len() - 1;
            state.set_current_field(last_field);
            let end_pos = state.get_current_input().len();
            state.set_current_cursor_pos(end_pos);
            *ideal_cursor_column = end_pos;
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

        CanvasAction::Custom(action_str) => {
            Ok(ActionResult::success_with_message(&format!("Custom action: {}", action_str)))
        }

        _ => Ok(ActionResult::success_with_message("Action not implemented")),
    }
}

// Helper functions for word navigation
fn find_next_word_start(text: &str, cursor_pos: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let mut pos = cursor_pos;
    
    // Skip current word
    while pos < chars.len() && chars[pos].is_alphanumeric() {
        pos += 1;
    }
    
    // Skip whitespace
    while pos < chars.len() && chars[pos].is_whitespace() {
        pos += 1;
    }
    
    pos
}

fn find_word_end(text: &str, cursor_pos: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let mut pos = cursor_pos;
    
    // Move to end of current word
    while pos < chars.len() && chars[pos].is_alphanumeric() {
        pos += 1;
    }
    
    pos
}

fn find_prev_word_start(text: &str, cursor_pos: usize) -> usize {
    if cursor_pos == 0 {
        return 0;
    }
    
    let chars: Vec<char> = text.chars().collect();
    let mut pos = cursor_pos.saturating_sub(1);
    
    // Skip whitespace
    while pos > 0 && chars[pos].is_whitespace() {
        pos -= 1;
    }
    
    // Skip to start of word
    while pos > 0 && chars[pos - 1].is_alphanumeric() {
        pos -= 1;
    }
    
    pos
}
