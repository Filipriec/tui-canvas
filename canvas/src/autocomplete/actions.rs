// src/autocomplete/actions.rs

use crate::canvas::state::CanvasState;
use crate::autocomplete::state::AutocompleteCanvasState;
use crate::canvas::actions::types::{CanvasAction, ActionResult};
use crate::canvas::actions::execute;
use anyhow::Result;

/// Enhanced execute function for states that support autocomplete
/// This is the main entry point for autocomplete-aware canvas execution
///
/// Use this instead of canvas::execute() if you want autocomplete behavior:
/// ```rust
/// execute_with_autocomplete(action, &mut state).await?;
/// ```
pub async fn execute_with_autocomplete<S: CanvasState + AutocompleteCanvasState + Send>(
    action: CanvasAction,
    state: &mut S,
) -> Result<ActionResult> {
    match &action {
        // === AUTOCOMPLETE-SPECIFIC ACTIONS ===

        CanvasAction::TriggerAutocomplete => {
            if state.supports_autocomplete(state.current_field()) {
                state.trigger_autocomplete_suggestions().await;
                Ok(ActionResult::success_with_message("Triggered autocomplete"))
            } else {
                Ok(ActionResult::success_with_message("Autocomplete not supported for this field"))
            }
        }

        CanvasAction::SuggestionUp => {
            if state.has_autocomplete_suggestions() {
                state.move_suggestion_selection(-1);
                Ok(ActionResult::success())
            } else {
                Ok(ActionResult::success_with_message("No suggestions available"))
            }
        }

        CanvasAction::SuggestionDown => {
            if state.has_autocomplete_suggestions() {
                state.move_suggestion_selection(1);
                Ok(ActionResult::success())
            } else {
                Ok(ActionResult::success_with_message("No suggestions available"))
            }
        }

        CanvasAction::SelectSuggestion => {
            if let Some(message) = state.apply_selected_suggestion() {
                Ok(ActionResult::success_with_message(&message))
            } else {
                Ok(ActionResult::success_with_message("No suggestion to select"))
            }
        }

        CanvasAction::ExitSuggestions => {
            state.clear_autocomplete_suggestions();
            Ok(ActionResult::success_with_message("Closed autocomplete"))
        }

        // === TEXT INSERTION WITH AUTO-TRIGGER ===

        CanvasAction::InsertChar(_) => {
            // First, execute the character insertion normally
            let result = execute(action, state).await?;

            // After successful insertion, check if we should auto-trigger autocomplete
            if result.is_success() && state.should_trigger_autocomplete() {
                state.trigger_autocomplete_suggestions().await;
            }

            Ok(result)
        }

        // === NAVIGATION/EDITING ACTIONS (clear autocomplete first) ===

        CanvasAction::MoveLeft | CanvasAction::MoveRight |
        CanvasAction::MoveUp | CanvasAction::MoveDown |
        CanvasAction::NextField | CanvasAction::PrevField |
        CanvasAction::DeleteBackward | CanvasAction::DeleteForward => {
            // Clear autocomplete when navigating/editing
            if state.is_autocomplete_active() {
                state.clear_autocomplete_suggestions();
            }

            // Execute the action normally
            execute(action, state).await
        }

        // === ALL OTHER ACTIONS (normal execution) ===

        _ => {
            // For all other actions, just execute normally
            execute(action, state).await
        }
    }
}

/// Helper function to integrate autocomplete actions with CanvasState.handle_feature_action()
///
/// Use this in your CanvasState implementation like this:
/// ```rust
/// fn handle_feature_action(&mut self, action: &CanvasAction, context: &ActionContext) -> Option<String> {
///     // Try autocomplete first
///     if let Some(result) = handle_autocomplete_feature_action(action, self) {
///         return Some(result);
///     }
///
///     // Handle your other custom actions...
///     None
/// }
/// ```
pub fn handle_autocomplete_feature_action<S: CanvasState + AutocompleteCanvasState + Send>(
    action: &CanvasAction,
    state: &S,
) -> Option<String> {
    match action {
        CanvasAction::TriggerAutocomplete => {
            if state.supports_autocomplete(state.current_field()) {
                if state.is_autocomplete_active() {
                    Some("Autocomplete already active".to_string())
                } else {
                    None // Let execute_with_autocomplete handle it
                }
            } else {
                Some("Autocomplete not available for this field".to_string())
            }
        }

        CanvasAction::SuggestionUp | CanvasAction::SuggestionDown => {
            if state.is_autocomplete_active() {
                None // Let execute_with_autocomplete handle navigation
            } else {
                Some("No autocomplete suggestions to navigate".to_string())
            }
        }

        CanvasAction::SelectSuggestion => {
            if state.has_autocomplete_suggestions() {
                None // Let execute_with_autocomplete handle selection
            } else {
                Some("No suggestion to select".to_string())
            }
        }

        CanvasAction::ExitSuggestions => {
            if state.is_autocomplete_active() {
                None // Let execute_with_autocomplete handle exit
            } else {
                Some("No autocomplete to close".to_string())
            }
        }

        _ => None // Not an autocomplete action
    }
}

/// Legacy compatibility function - kept for backward compatibility
/// This is the old function signature, now it just wraps the new system
#[deprecated(note = "Use execute_with_autocomplete instead")]
pub async fn execute_canvas_action_with_autocomplete<S: CanvasState + AutocompleteCanvasState + Send>(
    action: CanvasAction,
    state: &mut S,
    _ideal_cursor_column: &mut usize, // Ignored - new system manages this internally
    _config: Option<&()>, // Ignored - no more config system
) -> Result<ActionResult> {
    execute_with_autocomplete(action, state).await
}
