// canvas/src/actions/edit.rs

use crate::state::CanvasState;
use crossterm::event::{KeyCode, KeyEvent};
use anyhow::Result;

/// Execute a generic edit action on any CanvasState implementation.
/// This is the core function that makes the mode system work across all features.
pub async fn execute_edit_action<S: CanvasState>(
    action: &str,
    key: KeyEvent,
    state: &mut S,
    ideal_cursor_column: &mut usize,
) -> Result<String> {
    // 1. Try feature-specific handler first (for autocomplete, field-specific logic, etc.)
    let context = crate::state::ActionContext {
        key_code: Some(key.code),
        ideal_cursor_column: *ideal_cursor_column,
        current_input: state.get_current_input().to_string(),
        current_field: state.current_field(),
    };
    
    if let Some(result) = state.handle_feature_action(action, &context) {
        return Ok(result);
    }
    
    // 2. Handle suggestion-related actions generically
    if handle_suggestion_actions(action, state)? {
        return Ok("".to_string()); // Suggestion action handled
    }
    
    // 3. Fall back to generic canvas actions (handles 95% of all actions)
    handle_generic_action(action, key, state, ideal_cursor_column).await
}

/// Handle suggestion/autocomplete actions generically
fn handle_suggestion_actions<S: CanvasState>(action: &str, state: &mut S) -> Result<bool> {
    match action {
        "suggestion_down" => {
            if let Some(suggestions) = state.get_suggestions() {
                if !suggestions.is_empty() {
                    let current = state.get_selected_suggestion_index().unwrap_or(0);
                    let next = (current + 1) % suggestions.len();
                    state.set_selected_suggestion_index(Some(next));
                    return Ok(true);
                }
            }
            Ok(false)
        }
        "suggestion_up" => {
            if let Some(suggestions) = state.get_suggestions() {
                if !suggestions.is_empty() {
                    let current = state.get_selected_suggestion_index().unwrap_or(0);
                    let prev = if current == 0 { suggestions.len() - 1 } else { current - 1 };
                    state.set_selected_suggestion_index(Some(prev));
                    return Ok(true);
                }
            }
            Ok(false)
        }
        "select_suggestion" => {
            // Let feature handle this via handle_feature_action since it's feature-specific
            Ok(false)
        }
        "exit_suggestions" => {
            state.deactivate_suggestions();
            Ok(true)
        }
        _ => Ok(false)
    }
}

/// Handle generic canvas actions (movement, editing, etc.)
async fn handle_generic_action<S: CanvasState>(
    action: &str,
    key: KeyEvent,
    state: &mut S,
    ideal_cursor_column: &mut usize,
) -> Result<String> {
    match action {
        "insert_char" => {
            if let KeyCode::Char(c) = key.code {
                let cursor_pos = state.current_cursor_pos();
                let field_value = state.get_current_input_mut();
                let mut chars: Vec<char> = field_value.chars().collect();
                if cursor_pos <= chars.len() {
                    chars.insert(cursor_pos, c);
                    *field_value = chars.into_iter().collect();
                    state.set_current_cursor_pos(cursor_pos + 1);
                    state.set_has_unsaved_changes(true);
                    *ideal_cursor_column = state.current_cursor_pos();
                }
            } else {
                return Ok("Error: insert_char called without a char key.".to_string());
            }
            Ok("".to_string())
        }

        "delete_char_backward" => {
            if state.current_cursor_pos() > 0 {
                let cursor_pos = state.current_cursor_pos();
                let field_value = state.get_current_input_mut();
                let mut chars: Vec<char> = field_value.chars().collect();
                if cursor_pos <= chars.len() {
                    chars.remove(cursor_pos - 1);
                    *field_value = chars.into_iter().collect();
                    let new_pos = cursor_pos - 1;
                    state.set_current_cursor_pos(new_pos);
                    state.set_has_unsaved_changes(true);
                    *ideal_cursor_column = new_pos;
                }
            }
            Ok("".to_string())
        }

        "delete_char_forward" => {
            let cursor_pos = state.current_cursor_pos();
            let field_value = state.get_current_input_mut();
            let mut chars: Vec<char> = field_value.chars().collect();
            if cursor_pos < chars.len() {
                chars.remove(cursor_pos);
                *field_value = chars.into_iter().collect();
                state.set_has_unsaved_changes(true);
                *ideal_cursor_column = cursor_pos;
            }
            Ok("".to_string())
        }

        "next_field" => {
            let num_fields = state.fields().len();
            if num_fields > 0 {
                let current_field = state.current_field();
                let new_field = (current_field + 1) % num_fields;
                state.set_current_field(new_field);
                let current_input = state.get_current_input();
                let max_pos = current_input.len();
                state.set_current_cursor_pos((*ideal_cursor_column).min(max_pos));
            }
            Ok("".to_string())
        }

        "prev_field" => {
            let num_fields = state.fields().len();
            if num_fields > 0 {
                let current_field = state.current_field();
                let new_field = if current_field == 0 {
                    num_fields - 1
                } else {
                    current_field - 1
                };
                state.set_current_field(new_field);
                let current_input = state.get_current_input();
                let max_pos = current_input.len();
                state.set_current_cursor_pos((*ideal_cursor_column).min(max_pos));
            }
            Ok("".to_string())
        }

        "move_left" => {
            let new_pos = state.current_cursor_pos().saturating_sub(1);
            state.set_current_cursor_pos(new_pos);
            *ideal_cursor_column = new_pos;
            Ok("".to_string())
        }

        "move_right" => {
            let current_input = state.get_current_input();
            let current_pos = state.current_cursor_pos();
            if current_pos < current_input.len() {
                let new_pos = current_pos + 1;
                state.set_current_cursor_pos(new_pos);
                *ideal_cursor_column = new_pos;
            }
            Ok("".to_string())
        }

        "move_up" => {
            let num_fields = state.fields().len();
            if num_fields > 0 {
                let current_field = state.current_field();
                let new_field = current_field.saturating_sub(1);
                state.set_current_field(new_field);
                let current_input = state.get_current_input();
                let max_pos = current_input.len();
                state.set_current_cursor_pos((*ideal_cursor_column).min(max_pos));
            }
            Ok("".to_string())
        }

        "move_down" => {
            let num_fields = state.fields().len();
            if num_fields > 0 {
                let new_field = (state.current_field() + 1).min(num_fields - 1);
                state.set_current_field(new_field);
                let current_input = state.get_current_input();
                let max_pos = current_input.len();
                state.set_current_cursor_pos((*ideal_cursor_column).min(max_pos));
            }
            Ok("".to_string())
        }

        "move_line_start" => {
            state.set_current_cursor_pos(0);
            *ideal_cursor_column = 0;
            Ok("".to_string())
        }

        "move_line_end" => {
            let current_input = state.get_current_input();
            let new_pos = current_input.len();
            state.set_current_cursor_pos(new_pos);
            *ideal_cursor_column = new_pos;
            Ok("".to_string())
        }

        "move_first_line" => {
            let num_fields = state.fields().len();
            if num_fields > 0 {
                state.set_current_field(0);
                let current_input = state.get_current_input();
                let max_pos = current_input.len();
                state.set_current_cursor_pos((*ideal_cursor_column).min(max_pos));
            }
            Ok("Moved to first field".to_string())
        }

        "move_last_line" => {
            let num_fields = state.fields().len();
            if num_fields > 0 {
                let new_field = num_fields - 1;
                state.set_current_field(new_field);
                let current_input = state.get_current_input();
                let max_pos = current_input.len();
                state.set_current_cursor_pos((*ideal_cursor_column).min(max_pos));
            }
            Ok("Moved to last field".to_string())
        }

        "move_word_next" => {
            let current_input = state.get_current_input();
            if !current_input.is_empty() {
                let new_pos = find_next_word_start(current_input, state.current_cursor_pos());
                let final_pos = new_pos.min(current_input.len());
                state.set_current_cursor_pos(final_pos);
                *ideal_cursor_column = final_pos;
            }
            Ok("".to_string())
        }

        "move_word_end" => {
            let current_input = state.get_current_input();
            if !current_input.is_empty() {
                let current_pos = state.current_cursor_pos();
                let new_pos = find_word_end(current_input, current_pos);

                let final_pos = if new_pos == current_pos {
                    find_word_end(current_input, new_pos + 1)
                } else {
                    new_pos
                };

                let max_valid_index = current_input.len().saturating_sub(1);
                let clamped_pos = final_pos.min(max_valid_index);
                state.set_current_cursor_pos(clamped_pos);
                *ideal_cursor_column = clamped_pos;
            }
            Ok("".to_string())
        }

        "move_word_prev" => {
            let current_input = state.get_current_input();
            if !current_input.is_empty() {
                let new_pos = find_prev_word_start(current_input, state.current_cursor_pos());
                state.set_current_cursor_pos(new_pos);
                *ideal_cursor_column = new_pos;
            }
            Ok("".to_string())
        }

        "move_word_end_prev" => {
            let current_input = state.get_current_input();
            if !current_input.is_empty() {
                let new_pos = find_prev_word_end(current_input, state.current_cursor_pos());
                state.set_current_cursor_pos(new_pos);
                *ideal_cursor_column = new_pos;
            }
            Ok("Moved to previous word end".to_string())
        }

        _ => Ok(format!("Unknown or unhandled edit action: {}", action)),
    }
}

// Word movement helper functions
#[derive(PartialEq)]
enum CharType {
    Whitespace,
    Alphanumeric,
    Punctuation,
}

fn get_char_type(c: char) -> CharType {
    if c.is_whitespace() {
        CharType::Whitespace
    } else if c.is_alphanumeric() {
        CharType::Alphanumeric
    } else {
        CharType::Punctuation
    }
}

fn find_next_word_start(text: &str, current_pos: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    if len == 0 || current_pos >= len {
        return len;
    }

    let mut pos = current_pos;
    let initial_type = get_char_type(chars[pos]);

    while pos < len && get_char_type(chars[pos]) == initial_type {
        pos += 1;
    }

    while pos < len && get_char_type(chars[pos]) == CharType::Whitespace {
        pos += 1;
    }

    pos
}

fn find_word_end(text: &str, current_pos: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    if len == 0 {
        return 0;
    }

    let mut pos = current_pos.min(len - 1);

    if get_char_type(chars[pos]) == CharType::Whitespace {
        pos = find_next_word_start(text, pos);
    }

    if pos >= len {
        return len.saturating_sub(1);
    }

    let word_type = get_char_type(chars[pos]);
    while pos < len && get_char_type(chars[pos]) == word_type {
        pos += 1;
    }

    pos.saturating_sub(1).min(len.saturating_sub(1))
}

fn find_prev_word_start(text: &str, current_pos: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() || current_pos == 0 {
        return 0;
    }

    let mut pos = current_pos.saturating_sub(1);

    while pos > 0 && get_char_type(chars[pos]) == CharType::Whitespace {
        pos -= 1;
    }

    if pos == 0 && get_char_type(chars[pos]) == CharType::Whitespace {
        return 0;
    }

    let word_type = get_char_type(chars[pos]);
    while pos > 0 && get_char_type(chars[pos - 1]) == word_type {
        pos -= 1;
    }

    pos
}

fn find_prev_word_end(text: &str, current_pos: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    if len == 0 || current_pos == 0 {
        return 0;
    }

    let mut pos = current_pos.saturating_sub(1);

    while pos > 0 && get_char_type(chars[pos]) == CharType::Whitespace {
        pos -= 1;
    }

    if pos == 0 && get_char_type(chars[pos]) == CharType::Whitespace {
        return 0;
    }
    if pos == 0 && get_char_type(chars[pos]) != CharType::Whitespace {
        return 0;
    }

    let word_type = get_char_type(chars[pos]);
    while pos > 0 && get_char_type(chars[pos - 1]) == word_type {
        pos -= 1;
    }

    while pos > 0 && get_char_type(chars[pos - 1]) == CharType::Whitespace {
        pos -= 1;
    }

    if pos > 0 {
        pos - 1
    } else {
        0
    }
}
