// canvas/src/actions/edit.rs

use crate::state::{CanvasState, AutocompleteCanvasState, ActionContext};
use crate::actions::types::{CanvasAction, ActionResult};
use crossterm::event::{KeyCode, KeyEvent};
use anyhow::Result;

/// Execute a typed canvas action on any CanvasState implementation
pub async fn execute_canvas_action<S: CanvasState>(
    action: CanvasAction,
    state: &mut S,
    ideal_cursor_column: &mut usize,
) -> Result<ActionResult> {
    // 1. Try feature-specific handler first
    let context = ActionContext {
        key_code: None,
        ideal_cursor_column: *ideal_cursor_column,
        current_input: state.get_current_input().to_string(),
        current_field: state.current_field(),
    };

    if let Some(result) = state.handle_feature_action(&action, &context) {
        return Ok(ActionResult::HandledByFeature(result));
    }

    // 2. Handle autocomplete actions (falls back to legacy methods)
    if let Some(result) = handle_autocomplete_action(&action, state)? {
        return Ok(result);
    }

    // 3. Handle generic canvas actions
    handle_generic_canvas_action(action, state, ideal_cursor_column).await
}

/// Version for states that implement rich autocomplete
pub async fn execute_canvas_action_with_autocomplete<S: CanvasState + AutocompleteCanvasState>(
    action: CanvasAction,
    state: &mut S,
    ideal_cursor_column: &mut usize,
) -> Result<ActionResult> {
    // 1. Try feature-specific handler first
    let context = ActionContext {
        key_code: None,
        ideal_cursor_column: *ideal_cursor_column,
        current_input: state.get_current_input().to_string(),
        current_field: state.current_field(),
    };

    if let Some(result) = state.handle_feature_action(&action, &context) {
        return Ok(ActionResult::HandledByFeature(result));
    }

    // 2. Handle rich autocomplete actions
    if let Some(result) = handle_rich_autocomplete_action(&action, state)? {
        return Ok(result);
    }

    // 3. Handle generic canvas actions
    handle_generic_canvas_action(action, state, ideal_cursor_column).await
}

/// Legacy function for string-based actions (backwards compatibility)
pub async fn execute_edit_action<S: CanvasState>(
    action: &str,
    key: KeyEvent,
    state: &mut S,
    ideal_cursor_column: &mut usize,
) -> Result<String> {
    let typed_action = match action {
        "insert_char" => {
            if let KeyCode::Char(c) = key.code {
                CanvasAction::InsertChar(c)
            } else {
                return Ok("Error: insert_char called without a char key.".to_string());
            }
        }
        _ => CanvasAction::from_string(action),
    };

    let result = execute_canvas_action(typed_action, state, ideal_cursor_column).await?;

    // Convert ActionResult back to string for backwards compatibility
    Ok(result.message().unwrap_or("").to_string())
}

/// Handle autocomplete actions for basic CanvasState (uses legacy methods)
fn handle_autocomplete_action<S: CanvasState>(
    action: &CanvasAction,
    state: &mut S,
) -> Result<Option<ActionResult>> {
    match action {
        CanvasAction::TriggerAutocomplete => {
            // For basic CanvasState, just return an error or no-op
            Ok(Some(ActionResult::error("Autocomplete not supported - implement AutocompleteCanvasState for rich autocomplete")))
        }

        CanvasAction::SuggestionDown => {
            // Try legacy suggestions
            if let Some(suggestions) = state.get_suggestions() {
                if !suggestions.is_empty() {
                    let current = state.get_selected_suggestion_index().unwrap_or(0);
                    let next = (current + 1) % suggestions.len();
                    state.set_selected_suggestion_index(Some(next));
                    return Ok(Some(ActionResult::success()));
                }
            }
            Ok(None)
        }

        CanvasAction::SuggestionUp => {
            // Try legacy suggestions
            if let Some(suggestions) = state.get_suggestions() {
                if !suggestions.is_empty() {
                    let current = state.get_selected_suggestion_index().unwrap_or(0);
                    let prev = if current == 0 { suggestions.len() - 1 } else { current - 1 };
                    state.set_selected_suggestion_index(Some(prev));
                    return Ok(Some(ActionResult::success()));
                }
            }
            Ok(None)
        }

        CanvasAction::SelectSuggestion => {
            // Try legacy suggestions
            if let Some(suggestions) = state.get_suggestions() {
                if let Some(index) = state.get_selected_suggestion_index() {
                    if let Some(selected) = suggestions.get(index) {
                        // Clone the string first to avoid borrowing issues
                        let selected_text = selected.clone();
                        
                        // Now we can mutate state without holding any references
                        *state.get_current_input_mut() = selected_text.clone();
                        state.set_current_cursor_pos(selected_text.len());
                        state.set_has_unsaved_changes(true);
                        state.deactivate_suggestions();
                        return Ok(Some(ActionResult::success_with_message(
                            format!("Selected: {}", selected_text)
                        )));
                    }
                }
            }
            Ok(None)
        }

        CanvasAction::ExitSuggestions => {
            state.deactivate_suggestions();
            Ok(Some(ActionResult::success_with_message("Suggestions cancelled")))
        }

        _ => Ok(None),
    }
}

/// Handle rich autocomplete actions for AutocompleteCanvasState
fn handle_rich_autocomplete_action<S: CanvasState + AutocompleteCanvasState>(
    action: &CanvasAction,
    state: &mut S,
) -> Result<Option<ActionResult>> {
    match action {
        CanvasAction::TriggerAutocomplete => {
            if state.supports_autocomplete(state.current_field()) {
                state.activate_autocomplete();
                Ok(Some(ActionResult::success_with_message("Autocomplete activated - fetching suggestions...")))
            } else {
                Ok(Some(ActionResult::error("Autocomplete not supported for this field")))
            }
        }

        CanvasAction::SuggestionDown => {
            if state.is_autocomplete_ready() {
                if let Some(autocomplete_state) = state.autocomplete_state_mut() {
                    autocomplete_state.select_next();
                    return Ok(Some(ActionResult::success()));
                }
            }
            Ok(None)
        }

        CanvasAction::SuggestionUp => {
            if state.is_autocomplete_ready() {
                if let Some(autocomplete_state) = state.autocomplete_state_mut() {
                    autocomplete_state.select_previous();
                    return Ok(Some(ActionResult::success()));
                }
            }
            Ok(None)
        }

        CanvasAction::SelectSuggestion => {
            if state.is_autocomplete_ready() {
                if let Some(message) = state.apply_autocomplete_selection() {
                    return Ok(Some(ActionResult::success_with_message(message)));
                } else {
                    return Ok(Some(ActionResult::error("No suggestion selected")));
                }
            }
            Ok(None)
        }

        CanvasAction::ExitSuggestions => {
            if state.is_autocomplete_active() {
                state.deactivate_autocomplete();
                Ok(Some(ActionResult::success_with_message("Autocomplete cancelled")))
            } else {
                Ok(None)
            }
        }

        _ => Ok(None),
    }
}

/// Handle core canvas actions with full type safety
async fn handle_generic_canvas_action<S: CanvasState>(
    action: CanvasAction,
    state: &mut S,
    ideal_cursor_column: &mut usize,
) -> Result<ActionResult> {
    match action {
        CanvasAction::InsertChar(c) => {
            let cursor_pos = state.current_cursor_pos();
            let field_value = state.get_current_input_mut();
            let mut chars: Vec<char> = field_value.chars().collect();

            if cursor_pos <= chars.len() {
                chars.insert(cursor_pos, c);
                *field_value = chars.into_iter().collect();
                state.set_current_cursor_pos(cursor_pos + 1);
                state.set_has_unsaved_changes(true);
                *ideal_cursor_column = state.current_cursor_pos();
                Ok(ActionResult::success())
            } else {
                Ok(ActionResult::error("Invalid cursor position for character insertion"))
            }
        }

        CanvasAction::DeleteBackward => {
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
            Ok(ActionResult::success())
        }

        CanvasAction::DeleteForward => {
            let cursor_pos = state.current_cursor_pos();
            let field_value = state.get_current_input_mut();
            let mut chars: Vec<char> = field_value.chars().collect();

            if cursor_pos < chars.len() {
                chars.remove(cursor_pos);
                *field_value = chars.into_iter().collect();
                state.set_has_unsaved_changes(true);
                *ideal_cursor_column = cursor_pos;
            }
            Ok(ActionResult::success())
        }

        CanvasAction::NextField => {
            let num_fields = state.fields().len();
            if num_fields > 0 {
                let current_field = state.current_field();
                let new_field = (current_field + 1) % num_fields;
                state.set_current_field(new_field);
                let current_input = state.get_current_input();
                let max_pos = current_input.len();
                state.set_current_cursor_pos((*ideal_cursor_column).min(max_pos));
            }
            Ok(ActionResult::success())
        }

        CanvasAction::PrevField => {
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
            Ok(ActionResult::success())
        }

        CanvasAction::MoveLeft => {
            let new_pos = state.current_cursor_pos().saturating_sub(1);
            state.set_current_cursor_pos(new_pos);
            *ideal_cursor_column = new_pos;
            Ok(ActionResult::success())
        }

        CanvasAction::MoveRight => {
            let current_input = state.get_current_input();
            let current_pos = state.current_cursor_pos();
            if current_pos < current_input.len() {
                let new_pos = current_pos + 1;
                state.set_current_cursor_pos(new_pos);
                *ideal_cursor_column = new_pos;
            }
            Ok(ActionResult::success())
        }

        CanvasAction::MoveUp => {
            let num_fields = state.fields().len();
            if num_fields > 0 {
                let current_field = state.current_field();
                let new_field = current_field.saturating_sub(1);
                state.set_current_field(new_field);
                let current_input = state.get_current_input();
                let max_pos = current_input.len();
                state.set_current_cursor_pos((*ideal_cursor_column).min(max_pos));
            }
            Ok(ActionResult::success())
        }

        CanvasAction::MoveDown => {
            let num_fields = state.fields().len();
            if num_fields > 0 {
                let new_field = (state.current_field() + 1).min(num_fields - 1);
                state.set_current_field(new_field);
                let current_input = state.get_current_input();
                let max_pos = current_input.len();
                state.set_current_cursor_pos((*ideal_cursor_column).min(max_pos));
            }
            Ok(ActionResult::success())
        }

        CanvasAction::MoveLineStart => {
            state.set_current_cursor_pos(0);
            *ideal_cursor_column = 0;
            Ok(ActionResult::success())
        }

        CanvasAction::MoveLineEnd => {
            let current_input = state.get_current_input();
            let new_pos = current_input.len();
            state.set_current_cursor_pos(new_pos);
            *ideal_cursor_column = new_pos;
            Ok(ActionResult::success())
        }

        CanvasAction::MoveFirstLine => {
            let num_fields = state.fields().len();
            if num_fields > 0 {
                state.set_current_field(0);
                let current_input = state.get_current_input();
                let max_pos = current_input.len();
                state.set_current_cursor_pos((*ideal_cursor_column).min(max_pos));
            }
            Ok(ActionResult::success_with_message("Moved to first field"))
        }

        CanvasAction::MoveLastLine => {
            let num_fields = state.fields().len();
            if num_fields > 0 {
                let new_field = num_fields - 1;
                state.set_current_field(new_field);
                let current_input = state.get_current_input();
                let max_pos = current_input.len();
                state.set_current_cursor_pos((*ideal_cursor_column).min(max_pos));
            }
            Ok(ActionResult::success_with_message("Moved to last field"))
        }

        CanvasAction::MoveWordNext => {
            let current_input = state.get_current_input();
            if !current_input.is_empty() {
                let new_pos = find_next_word_start(current_input, state.current_cursor_pos());
                let final_pos = new_pos.min(current_input.len());
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
            Ok(ActionResult::success_with_message("Moved to previous word end"))
        }

        CanvasAction::Custom(action_str) => {
            Ok(ActionResult::error(format!("Unknown or unhandled custom action: {}", action_str)))
        }

        CanvasAction::TriggerAutocomplete | CanvasAction::SuggestionUp | CanvasAction::SuggestionDown |
        CanvasAction::SelectSuggestion | CanvasAction::ExitSuggestions => {
            Ok(ActionResult::error("Autocomplete action not handled properly"))
        }
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
