// src/autocomplete/actions.rs

use crate::canvas::state::{CanvasState, ActionContext};
use crate::autocomplete::state::AutocompleteCanvasState;
use crate::canvas::actions::types::{CanvasAction, ActionResult};
use crate::canvas::actions::execute;
use anyhow::Result;

/// Version for states that implement rich autocomplete
pub async fn execute_canvas_action_with_autocomplete<S: CanvasState + AutocompleteCanvasState>(
    action: CanvasAction,
    state: &mut S,
    _ideal_cursor_column: &mut usize, // Keep for compatibility
    _config: Option<&()>, // Remove CanvasConfig, keep for compatibility
) -> Result<ActionResult> {
    // Check for autocomplete-specific actions first
    match &action {
        CanvasAction::InsertChar(_) => {
            // Character insertion - execute then potentially trigger autocomplete
            let result = execute(action, state).await?;
            
            // Check if we should trigger autocomplete after character insertion
            if state.should_trigger_autocomplete() {
                state.trigger_autocomplete_suggestions().await;
            }
            
            Ok(result)
        }
        
        _ => {
            // For other actions, clear suggestions and execute
            let result = execute(action, state).await?;
            
            // Clear autocomplete on navigation/other actions
            match action {
                CanvasAction::MoveLeft | CanvasAction::MoveRight | 
                CanvasAction::MoveUp | CanvasAction::MoveDown |
                CanvasAction::NextField | CanvasAction::PrevField => {
                    state.clear_autocomplete_suggestions();
                }
                _ => {}
            }
            
            Ok(result)
        }
    }
}

/// Handle autocomplete-specific actions (called from handle_feature_action)
pub async fn handle_autocomplete_action<S: CanvasState + AutocompleteCanvasState>(
    action: CanvasAction,
    state: &mut S,
    _context: &ActionContext,
) -> Result<ActionResult> {
    match action {        
        CanvasAction::TriggerAutocomplete => {
            // Manual trigger of autocomplete
            state.trigger_autocomplete_suggestions().await;
            Ok(ActionResult::success_with_message("Triggered autocomplete"))
        }

        CanvasAction::SuggestionUp => {
            // Navigate up in suggestions
            if state.has_autocomplete_suggestions() {
                state.move_suggestion_selection(-1);
                Ok(ActionResult::success())
            } else {
                Ok(ActionResult::success_with_message("No suggestions available"))
            }
        }

        CanvasAction::SuggestionDown => {
            // Navigate down in suggestions  
            if state.has_autocomplete_suggestions() {
                state.move_suggestion_selection(1);
                Ok(ActionResult::success())
            } else {
                Ok(ActionResult::success_with_message("No suggestions available"))
            }
        }

        CanvasAction::SelectSuggestion => {
            // Accept the selected suggestion
            if let Some(suggestion) = state.get_selected_suggestion() {
                state.apply_suggestion(&suggestion);
                state.clear_autocomplete_suggestions();
                Ok(ActionResult::success_with_message("Applied suggestion"))
            } else {
                Ok(ActionResult::success_with_message("No suggestion selected"))
            }
        }

        CanvasAction::ExitSuggestions => {
            // Cancel autocomplete
            state.clear_autocomplete_suggestions();
            Ok(ActionResult::success_with_message("Cleared suggestions"))
        }

        _ => {
            // Not an autocomplete action
            Ok(ActionResult::success_with_message("Not an autocomplete action"))
        }
    }
}
