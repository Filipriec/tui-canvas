// src/autocomplete/actions.rs

use crate::canvas::state::{CanvasState, ActionContext};
use crate::autocomplete::state::AutocompleteCanvasState;
use crate::canvas::actions::types::{CanvasAction, ActionResult};
use crate::dispatcher::ActionDispatcher; // NEW: Use dispatcher directly
use crate::config::CanvasConfig;
use anyhow::Result;

/// Version for states that implement rich autocomplete
pub async fn execute_canvas_action_with_autocomplete<S: CanvasState + AutocompleteCanvasState>(
    action: CanvasAction,
    state: &mut S,
    ideal_cursor_column: &mut usize,
    config: Option<&CanvasConfig>,
) -> Result<ActionResult> {
    // 1. Try feature-specific handler first
    let context = ActionContext {
        key_code: None,
        ideal_cursor_column: *ideal_cursor_column,
        current_input: state.get_current_input().to_string(),
        current_field: state.current_field(),
    };

    if let Some(result) = handle_rich_autocomplete_action(action.clone(), state, &context) {
        return Ok(result);
    }

    // 2. Handle generic actions using the new dispatcher directly
    let result = ActionDispatcher::dispatch_with_config(action.clone(), state, ideal_cursor_column, config).await?;

    // 3. AUTO-TRIGGER LOGIC: Check if we should activate/deactivate autocomplete
    if let Some(cfg) = config {
        if cfg.should_auto_trigger_autocomplete() {
            match action {
                CanvasAction::InsertChar(_) => {
                    let current_field = state.current_field();
                    let current_input = state.get_current_input();

                    if state.supports_autocomplete(current_field)
                        && !state.is_autocomplete_active()
                        && current_input.len() >= 1
                    {
                        state.activate_autocomplete();
                    }
                }

                CanvasAction::NextField | CanvasAction::PrevField => {
                    let current_field = state.current_field();

                    if state.supports_autocomplete(current_field) && !state.is_autocomplete_active() {
                        state.activate_autocomplete();
                    } else if !state.supports_autocomplete(current_field) && state.is_autocomplete_active() {
                        state.deactivate_autocomplete();
                    }
                }

                _ => {} // No auto-trigger for other actions
            }
        }
    }

    Ok(result)
}

/// Handle rich autocomplete actions for AutocompleteCanvasState
fn handle_rich_autocomplete_action<S: CanvasState + AutocompleteCanvasState>(
    action: CanvasAction,
    state: &mut S,
    _context: &ActionContext,
) -> Option<ActionResult> {
    match action {
        CanvasAction::TriggerAutocomplete => {
            let current_field = state.current_field();
            if state.supports_autocomplete(current_field) {
                state.activate_autocomplete();
                Some(ActionResult::success_with_message("Autocomplete activated"))
            } else {
                Some(ActionResult::success_with_message("Autocomplete not supported for this field"))
            }
        }

        CanvasAction::SuggestionUp => {
            if state.is_autocomplete_ready() {
                if let Some(autocomplete_state) = state.autocomplete_state_mut() {
                    autocomplete_state.select_previous();
                }
                Some(ActionResult::success())
            } else {
                Some(ActionResult::success_with_message("No suggestions available"))
            }
        }

        CanvasAction::SuggestionDown => {
            if state.is_autocomplete_ready() {
                if let Some(autocomplete_state) = state.autocomplete_state_mut() {
                    autocomplete_state.select_next();
                }
                Some(ActionResult::success())
            } else {
                Some(ActionResult::success_with_message("No suggestions available"))
            }
        }

        CanvasAction::SelectSuggestion => {
            if state.is_autocomplete_ready() {
                if let Some(msg) = state.apply_autocomplete_selection() {
                    Some(ActionResult::success_with_message(&msg))
                } else {
                    Some(ActionResult::success_with_message("No suggestion selected"))
                }
            } else {
                Some(ActionResult::success_with_message("No suggestions available"))
            }
        }

        CanvasAction::ExitSuggestions => {
            if state.is_autocomplete_active() {
                state.deactivate_autocomplete();
                Some(ActionResult::success_with_message("Exited autocomplete"))
            } else {
                Some(ActionResult::success())
            }
        }

        _ => None, // Not a rich autocomplete action
    }
}
