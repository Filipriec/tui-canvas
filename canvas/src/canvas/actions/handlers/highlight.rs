// src/canvas/actions/handlers/highlight.rs

use crate::canvas::actions::types::{CanvasAction, ActionResult};
use crate::config::introspection::{ActionHandlerIntrospection, HandlerCapabilities, ActionSpec};

use crate::canvas::actions::movement::*;
use crate::canvas::state::CanvasState;
use crate::config::CanvasConfig;
use anyhow::Result;

const FOR_EDIT_MODE: bool = false; // Highlight mode uses read-only cursor behavior
                                   
pub struct HighlightHandler;

/// Handle actions in highlight/visual mode
/// TODO: Implement selection logic and highlight-specific behaviors
pub async fn handle_highlight_action<S: CanvasState>(
    action: CanvasAction,
    state: &mut S,
    ideal_cursor_column: &mut usize,
    config: Option<&CanvasConfig>,
) -> Result<ActionResult> {
    match action {
        // Movement actions work similar to read-only mode but with selection
        CanvasAction::MoveLeft => {
            let new_pos = move_left(state.current_cursor_pos());
            state.set_current_cursor_pos(new_pos);
            *ideal_cursor_column = new_pos;
            // TODO: Update selection range
            Ok(ActionResult::success())
        }

        CanvasAction::MoveRight => {
            let current_input = state.get_current_input();
            let current_pos = state.current_cursor_pos();
            let new_pos = move_right(current_pos, current_input, FOR_EDIT_MODE);
            state.set_current_cursor_pos(new_pos);
            *ideal_cursor_column = new_pos;
            // TODO: Update selection range
            Ok(ActionResult::success())
        }

        CanvasAction::MoveWordNext => {
            let current_input = state.get_current_input();
            if !current_input.is_empty() {
                let new_pos = find_next_word_start(current_input, state.current_cursor_pos());
                let final_pos = clamp_cursor_position(new_pos, current_input, FOR_EDIT_MODE);
                state.set_current_cursor_pos(final_pos);
                *ideal_cursor_column = final_pos;
                // TODO: Update selection range
            }
            Ok(ActionResult::success())
        }

        CanvasAction::MoveWordEnd => {
            let current_input = state.get_current_input();
            if !current_input.is_empty() {
                let new_pos = find_word_end(current_input, state.current_cursor_pos());
                let final_pos = clamp_cursor_position(new_pos, current_input, FOR_EDIT_MODE);
                state.set_current_cursor_pos(final_pos);
                *ideal_cursor_column = final_pos;
                // TODO: Update selection range
            }
            Ok(ActionResult::success())
        }

        CanvasAction::MoveWordPrev => {
            let current_input = state.get_current_input();
            if !current_input.is_empty() {
                let new_pos = find_prev_word_start(current_input, state.current_cursor_pos());
                state.set_current_cursor_pos(new_pos);
                *ideal_cursor_column = new_pos;
                // TODO: Update selection range
            }
            Ok(ActionResult::success())
        }

        CanvasAction::MoveLineStart => {
            let new_pos = line_start_position();
            state.set_current_cursor_pos(new_pos);
            *ideal_cursor_column = new_pos;
            // TODO: Update selection range
            Ok(ActionResult::success())
        }

        CanvasAction::MoveLineEnd => {
            let current_input = state.get_current_input();
            let new_pos = line_end_position(current_input, FOR_EDIT_MODE);
            state.set_current_cursor_pos(new_pos);
            *ideal_cursor_column = new_pos;
            // TODO: Update selection range
            Ok(ActionResult::success())
        }

        // Highlight mode doesn't handle editing actions
        CanvasAction::InsertChar(_) | 
        CanvasAction::DeleteBackward | 
        CanvasAction::DeleteForward => {
            Ok(ActionResult::success_with_message("Action not available in highlight mode"))
        }

        CanvasAction::Custom(action_str) => {
            Ok(ActionResult::success_with_message(&format!("Custom highlight action: {}", action_str)))
        }

        _ => {
            Ok(ActionResult::success_with_message("Action not implemented for highlight mode"))
        }
    }
}

impl ActionHandlerIntrospection for HighlightHandler {
    fn introspect() -> HandlerCapabilities {
        let mut actions = Vec::new();

        // For now, highlight mode uses similar movement to readonly
        // but this will be discovered from actual implementation

        // REQUIRED ACTIONS - Basic movement in highlight mode
        actions.push(ActionSpec {
            name: "move_left".to_string(),
            description: "Move cursor left and extend selection".to_string(),
            examples: vec!["h".to_string(), "Left".to_string()],
            is_required: true,
        });

        actions.push(ActionSpec {
            name: "move_right".to_string(),
            description: "Move cursor right and extend selection".to_string(),
            examples: vec!["l".to_string(), "Right".to_string()],
            is_required: true,
        });

        actions.push(ActionSpec {
            name: "move_up".to_string(),
            description: "Move up and extend selection".to_string(),
            examples: vec!["k".to_string(), "Up".to_string()],
            is_required: true,
        });

        actions.push(ActionSpec {
            name: "move_down".to_string(),
            description: "Move down and extend selection".to_string(),
            examples: vec!["j".to_string(), "Down".to_string()],
            is_required: true,
        });

        // OPTIONAL ACTIONS - Advanced highlight movement
        actions.push(ActionSpec {
            name: "move_word_next".to_string(),
            description: "Move to next word and extend selection".to_string(),
            examples: vec!["w".to_string()],
            is_required: false,
        });

        actions.push(ActionSpec {
            name: "move_word_end".to_string(),
            description: "Move to word end and extend selection".to_string(),
            examples: vec!["e".to_string()],
            is_required: false,
        });

        actions.push(ActionSpec {
            name: "move_word_prev".to_string(),
            description: "Move to previous word and extend selection".to_string(),
            examples: vec!["b".to_string()],
            is_required: false,
        });

        actions.push(ActionSpec {
            name: "move_line_start".to_string(),
            description: "Move to line start and extend selection".to_string(),
            examples: vec!["0".to_string()],
            is_required: false,
        });

        actions.push(ActionSpec {
            name: "move_line_end".to_string(),
            description: "Move to line end and extend selection".to_string(),
            examples: vec!["$".to_string()],
            is_required: false,
        });

        HandlerCapabilities {
            mode_name: "highlight".to_string(),
            actions,
            auto_handled: vec![], // Highlight mode has no auto-handled actions
        }
    }

    fn validate_capabilities() -> Result<(), String> {
        let caps = Self::introspect();
        let required_count = caps.actions.iter().filter(|a| a.is_required).count();

        if required_count < 4 { // We expect at least 4 required actions (basic movement)
            return Err(format!(
                "Highlight handler claims only {} required actions, expected at least 4",
                required_count
            ));
        }

        Ok(())
    }
}

