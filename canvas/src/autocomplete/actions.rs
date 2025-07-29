// src/autocomplete/actions.rs

use crate::canvas::state::{CanvasState, ActionContext};
use crate::autocomplete::state::AutocompleteCanvasState;
use crate::canvas::actions::types::{CanvasAction, ActionResult};
use crate::canvas::actions::edit::handle_generic_canvas_action;  // Import the core function
use anyhow::Result;

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
